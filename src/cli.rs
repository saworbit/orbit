use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::request::{CopyRequest, OrbitRequest, OutputMode, PlanRequest, VerifyMode};

#[derive(Debug, Parser)]
#[command(name = "orbit", version, about = "A trustworthy local data copier")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect exactly what Orbit would do without writing anything.
    Plan(PlanArgs),
    /// Safely copy one regular local file to a new destination.
    Copy(CopyArgs),
}

#[derive(Debug, Args)]
struct PlanArgs {
    source: PathBuf,
    destination: PathBuf,
    #[arg(long)]
    replace: bool,
    #[arg(long, value_enum, default_value_t = VerifyArg::Hash)]
    verify: VerifyArg,
    #[arg(long)]
    ignore_unsupported: bool,
    #[arg(long, conflicts_with = "quiet")]
    json: bool,
    #[arg(long, conflicts_with = "json")]
    quiet: bool,
}

#[derive(Debug, Args)]
struct CopyArgs {
    source: PathBuf,
    destination: PathBuf,
    #[arg(long, value_enum, default_value_t = VerifyArg::Hash)]
    verify: VerifyArg,
    #[arg(long, conflicts_with = "quiet")]
    json: bool,
    #[arg(long, conflicts_with = "json")]
    quiet: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum VerifyArg {
    Hash,
    Size,
}

impl Cli {
    pub fn parse_request_from<I, T>(args: I) -> Result<OrbitRequest, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let cli = Self::try_parse_from(args)?;
        Ok(cli.into_request())
    }

    pub fn into_request(self) -> OrbitRequest {
        match self.command {
            Command::Plan(args) => OrbitRequest::Plan(PlanRequest {
                source: args.source,
                destination: args.destination,
                replace: args.replace,
                verify: args.verify.into(),
                ignore_unsupported: args.ignore_unsupported,
                output: output_mode(args.json, args.quiet),
            }),
            Command::Copy(args) => OrbitRequest::Copy(CopyRequest {
                source: args.source,
                destination: args.destination,
                verify: args.verify.into(),
                output: output_mode(args.json, args.quiet),
            }),
        }
    }
}

impl From<VerifyArg> for VerifyMode {
    fn from(value: VerifyArg) -> Self {
        match value {
            VerifyArg::Hash => Self::Hash,
            VerifyArg::Size => Self::Size,
        }
    }
}

fn output_mode(json: bool, quiet: bool) -> OutputMode {
    if json {
        OutputMode::Json
    } else if quiet {
        OutputMode::Quiet
    } else {
        OutputMode::Human
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use crate::request::{OrbitRequest, OutputMode, VerifyMode};

    #[test]
    fn parses_plan_with_safe_defaults() {
        let OrbitRequest::Plan(request) =
            Cli::parse_request_from(["orbit", "plan", "from", "to"]).unwrap()
        else {
            panic!("expected plan request");
        };
        assert_eq!(request.verify, VerifyMode::Hash);
        assert_eq!(request.output, OutputMode::Human);
        assert!(!request.replace);
        assert!(!request.ignore_unsupported);
    }

    #[test]
    fn parses_copy_with_safe_defaults() {
        let OrbitRequest::Copy(request) =
            Cli::parse_request_from(["orbit", "copy", "from", "to"]).unwrap()
        else {
            panic!("expected copy request");
        };
        assert_eq!(request.verify, VerifyMode::Hash);
        assert_eq!(request.output, OutputMode::Human);
    }

    #[test]
    fn rejects_json_and_quiet_together() {
        let error = Cli::parse_request_from(["orbit", "plan", "from", "to", "--json", "--quiet"])
            .unwrap_err();
        assert_eq!(error.exit_code(), 2);
    }
}
