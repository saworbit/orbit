use std::io::Write;
use std::path::Path;

use crate::paths::resolve_endpoints;
use crate::plan::build_plan;
use crate::report::{write_blocking_diagnostics, write_error, write_plan};
use crate::request::{OutputMode, PlanRequest};
use crate::scan::scan_source;

pub fn run_plan(
    request: PlanRequest,
    cwd: &Path,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let output = request.output;
    let result = (|| {
        let endpoints = resolve_endpoints(&request.source, &request.destination, cwd)?;
        let snapshot = scan_source(&endpoints.source)?;
        build_plan(&request, &endpoints, &snapshot)
    })();

    match result {
        Ok(plan) => {
            if output == OutputMode::Quiet && !plan.is_executable() {
                return match write_blocking_diagnostics(stderr, &plan).and_then(|()| stderr.flush())
                {
                    Ok(()) => 3,
                    Err(_) => 1,
                };
            }
            if let Err(error) = write_plan(stdout, output, &plan) {
                write_report_failure(stderr, &error);
                return 1;
            }
            if output != OutputMode::Quiet {
                if let Err(error) = stdout.flush() {
                    write_report_failure(stderr, &error);
                    return 1;
                }
            }
            if plan.is_executable() { 0 } else { 3 }
        }
        Err(error) => {
            let code = error.exit_code();
            let report_result = if output == OutputMode::Json {
                write_error(stdout, output, &error).and_then(|()| stdout.flush())
            } else {
                write_error(stderr, output, &error).and_then(|()| stderr.flush())
            };
            if let Err(report_error) = report_result {
                if output == OutputMode::Json {
                    write_report_failure(stderr, &report_error);
                }
                return 1;
            }
            code
        }
    }
}

fn write_report_failure(stderr: &mut impl Write, error: &std::io::Error) {
    if writeln!(stderr, "error: cannot write report: {error}").is_ok() {
        let _ = stderr.flush();
    }
}
