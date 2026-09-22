use std::process::ExitCode;

use clap::Parser;
use orbit::app::{run_copy, run_plan};
use orbit::cli::Cli;
use orbit::request::OrbitRequest;

fn main() -> ExitCode {
    let request = Cli::parse().into_request();
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("error: cannot determine current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let exit = match request {
        OrbitRequest::Plan(request) => run_plan(
            request,
            &cwd,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ),
        OrbitRequest::Copy(request) => run_copy(
            request,
            &cwd,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ),
    };
    ExitCode::from(exit)
}
