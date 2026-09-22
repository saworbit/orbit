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
            Self::Io { .. } => "io_error",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::SourceMissing(_)
            | Self::SameEndpoint(_)
            | Self::DestinationInsideSource { .. } => 3,
            Self::Io { .. } => 3,
        }
    }
}

pub type Result<T> = std::result::Result<T, OrbitError>;
