//! API endpoints module

pub mod admin;
pub mod auth;
pub mod backends;
pub mod estimates;
pub mod files;
pub mod jobs;
pub mod stats;

pub use admin::{create_user, list_users, CreateUserReq, UserDto};
pub use auth::{login_handler, logout_handler, me_handler, LoginResponse};
pub use backends::{get_backend, list_backends, BackendInfo};
pub use estimates::{get_estimate, JobEstimate};
pub use files::{list_drives, list_files, FileEntry, ListFilesQuery};
pub use jobs::{
    cancel_job, create_job, delete_job, get_job_stats, list_jobs, run_job, CreateJobRequest,
    JobInfo,
};
pub use stats::{get_system_health, SystemHealth};
