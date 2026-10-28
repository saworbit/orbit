use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifyMode {
    Hash,
    Size,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputMode {
    Human,
    Json,
    Quiet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRequest {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub replace: bool,
    pub verify: VerifyMode,
    pub ignore_unsupported: bool,
    pub output: OutputMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopyRequest {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub verify: VerifyMode,
    pub output: OutputMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrbitRequest {
    Plan(PlanRequest),
    Copy(CopyRequest),
}
