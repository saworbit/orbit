//! API endpoints module

pub mod auth;
pub mod backends;
pub mod jobs;

pub use auth::{login_handler, logout_handler, me_handler, LoginResponse};
pub use backends::{get_backend, list_backends, BackendInfo};
pub use jobs::{
    cancel_job, create_job, delete_job, get_job_stats, list_jobs, run_job, CreateJobRequest,
    JobInfo,
};
