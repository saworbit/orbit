use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::request::{OutputMode, PlanRequest, VerifyMode};

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

#[derive(Clone, Copy, Debug, ValueEnum)]
enum VerifyArg {
    Hash,
    Size,
}

impl Cli {
    pub fn parse_request_from<I, T>(args: I) -> Result<PlanRequest, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let cli = Self::try_parse_from(args)?;
        Ok(cli.into_request())
    }

    pub fn into_request(self) -> PlanRequest {
        let Command::Plan(args) = self.command;
        PlanRequest {
            source: args.source,
            destination: args.destination,
            replace: args.replace,
            verify: match args.verify {
                VerifyArg::Hash => VerifyMode::Hash,
                VerifyArg::Size => VerifyMode::Size,
            },
            ignore_unsupported: args.ignore_unsupported,
            output: if args.json {
                OutputMode::Json
            } else if args.quiet {
                OutputMode::Quiet
            } else {
                OutputMode::Human
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use crate::request::{OutputMode, VerifyMode};

    #[test]
    fn parses_plan_with_safe_defaults() {
        let request = Cli::parse_request_from(["orbit", "plan", "from", "to"]).unwrap();
        assert_eq!(request.verify, VerifyMode::Hash);
        assert_eq!(request.output, OutputMode::Human);
        assert!(!request.replace);
        assert!(!request.ignore_unsupported);
    }

    #[test]
    fn rejects_json_and_quiet_together() {
        let error = Cli::parse_request_from(["orbit", "plan", "from", "to", "--json", "--quiet"])
            .unwrap_err();
        assert_eq!(error.exit_code(), 2);
    }
}
