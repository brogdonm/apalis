use std::time::Duration;

use apalis::{
    layers::{tracing::TraceLayer, RateLimitLayer},
    sqlite::SqliteStorage,
    Job, JobContext, JobFuture, JobHandler, Monitor, Storage, WorkerBuilder,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Email {
    to: String,
    subject: String,
    text: String,
}

impl Job for Email {
    const NAME: &'static str = "sqlite::Email";
}

impl JobHandler<Self> for Email {
    type Result = JobFuture<()>;

    fn handle(self, ctx: JobContext) -> Self::Result {
        let fut = async move {
            actix::clock::sleep(Duration::from_millis(5000)).await;
            tracing::info!(subject = ?self.to, "Sent email");
        };
        Box::pin(fut)
    }
}

async fn produce_jobs(storage: &SqliteStorage<Email>) {
    let mut storage = storage.clone();
    for i in 0..100 {
        storage
            .schedule(
                Email {
                    to: format!("test{}@example.com", i),
                    text: "Test backround job from Apalis".to_string(),
                    subject: "Background email job".to_string(),
                },
                Utc::now() + chrono::Duration::seconds(i),
            )
            .await
            .unwrap();
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug,sqlx::query=error");
    tracing_subscriber::fmt::init();

    let sqlite: SqliteStorage<Email> = SqliteStorage::connect("sqlite://data.db").await.unwrap();
    // Do migrations: Mainly for "sqlite::memory:"
    sqlite
        .setup()
        .await
        .expect("unable to run migrations for sqlite");

    // This can be in another part of the program
    // produce_jobs(&sqlite).await;

    Monitor::new()
        .register_with_count(5, move |_| {
            WorkerBuilder::new(sqlite.clone())
                .layer(RateLimitLayer::new(10, Duration::from_millis(10)))
                .layer(TraceLayer::new())
                .build()
                .start()
        })
        .run()
        .await
}
