use crate::{
    error::WorkerError,
    job::Job,
    request::{JobReport, JobState},
    response::JobResult,
};
use actix::{Message, Recipient};
use chrono::{DateTime, Utc};
use http::Extensions;
use std::any::Any;

/// The context for a job is represented here
/// Used to provide a context when a job is defined through the [Job] trait
#[derive(Debug)]
pub struct JobContext {
    data: Extensions,
    job_tracker: Option<JobTracker>,
}

impl Default for JobContext {
    fn default() -> Self {
        JobContext::new()
    }
}

impl Clone for JobContext {
    fn clone(&self) -> Self {
        JobContext {
            /// Extensions are not clone and hence should be loaded via the AddExtension Layer
            data: Default::default(),
            job_tracker: self.job_tracker.clone(),
        }
    }
}

impl JobContext {
    pub fn new() -> Self {
        JobContext {
            data: Default::default(),
            job_tracker: None,
        }
    }

    /// Get a reference to a type previously inserted on this `JobContext`.
    ///
    /// # Example
    ///
    /// ```
    /// # use apalis_core::context::JobContext;
    /// let mut ctx = JobContext::new();
    /// assert!(ctx.data_opt::<i32>().is_none());
    /// ctx.insert(5i32);
    ///
    /// assert_eq!(ctx.data_opt::<i32>(), Some(&5i32));
    /// ```
    pub fn data_opt<D: Any + Send + Sync>(&self) -> Option<&D> {
        self.data.get()
    }

    /// Insert a type into this `JobContext`.
    ///
    /// Important for embedding data for a job.
    /// If a extension of this type already existed, it will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use apalis_core::context::JobContext;
    /// let mut ctx = JobContext::new();
    /// assert!(ctx.insert(5i32).is_none());
    /// assert!(ctx.insert(4u8).is_none());
    /// assert_eq!(ctx.insert(9i32), Some(5i32));
    /// ```
    pub(crate) fn insert<D: Any + Send + Sync>(&mut self, data: D) -> Option<D> {
        self.data.insert(data)
    }

    pub(crate) fn set_tracker(&mut self, tracker: JobTracker) {
        self.job_tracker = Some(tracker);
    }

    pub fn update_progress(&self, progress: u8) {
        match &self.job_tracker {
            Some(tracker) => {
                tracker.update_progress(progress);
            }
            None => {}
        }
    }

    pub(crate) fn get_tracker(&self) -> &Option<JobTracker> {
        &self.job_tracker
    }

    pub fn ack(&mut self) -> JobResult {
        JobResult::Success
    }

    pub fn retry(&mut self) -> JobResult {
        JobResult::Retry
    }

    pub fn kill(&mut self) -> JobResult {
        JobResult::Kill
    }
}

#[derive(Debug, Clone)]
pub struct JobTracker {
    job_id: String,
    addr: Recipient<JobReport>,
}

impl JobTracker {
    fn update_progress(&self, progress: u8) {
        tracing::info!(target: "context", progress = progress, "progress.update");
        self.addr
            .do_send(JobReport::progress(self.job_id.clone(), progress));
    }

    pub(crate) fn new(job_id: String, addr: Recipient<JobReport>) -> JobTracker {
        JobTracker { job_id, addr }
    }
}
