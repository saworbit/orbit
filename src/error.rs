use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum OrbitError {
    #[error("source does not exist: {0}")]
    SourceMissing(PathBuf),
    #[error("source and destination resolve to the same path: {0}")]
    SameEndpoint(PathBuf),
    #[error("destination is inside the source tree: {destination}")]
    DestinationInsideSource { destination: PathBuf },
    #[error("copy source is not a regular file: {0}")]
    CopyRequiresRegularFile(PathBuf),
    #[error("destination already exists: {0}")]
    DestinationExists(PathBuf),
    #[error("destination parent does not exist or is not a directory: {0}")]
    DestinationParentMissing(PathBuf),
    #[error("staged copy did not match the source: {0}")]
    VerificationFailed(PathBuf),
    #[error("cannot remove staging file {path}: {source}")]
    StagingCleanup {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl OrbitError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SourceMissing(_) => "source_missing",
            Self::SameEndpoint(_) => "same_endpoint",
            Self::DestinationInsideSource { .. } => "destination_inside_source",
            Self::CopyRequiresRegularFile(_) => "copy_requires_regular_file",
            Self::DestinationExists(_) => "destination_exists",
            Self::DestinationParentMissing(_) => "destination_parent_missing",
            Self::VerificationFailed(_) => "verification_failed",
            Self::StagingCleanup { .. } => "staging_cleanup_failed",
            Self::Io { .. } => "io_error",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::SourceMissing(_)
            | Self::SameEndpoint(_)
            | Self::DestinationInsideSource { .. }
            | Self::CopyRequiresRegularFile(_)
            | Self::DestinationExists(_)
            | Self::DestinationParentMissing(_) => 3,
            Self::VerificationFailed(_) | Self::StagingCleanup { .. } => 1,
            Self::Io { .. } => 3,
        }
    }
}

pub type Result<T> = std::result::Result<T, OrbitError>;
