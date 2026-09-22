use std::io::Write;
use std::path::Path;

use crate::paths::resolve_endpoints;
use crate::plan::build_plan;
use crate::report::{write_error, write_plan};
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
            if let Err(error) = write_plan(stdout, output, &plan) {
                let _ = writeln!(stderr, "error: cannot write report: {error}");
                return 1;
            }
            if plan.is_executable() { 0 } else { 3 }
        }
        Err(error) => {
            let code = error.exit_code();
            let report_result = if output == OutputMode::Json {
                write_error(stdout, output, &error)
            } else {
                write_error(stderr, output, &error)
            };
            if let Err(report_error) = report_result {
                if output == OutputMode::Json {
                    let _ = writeln!(stderr, "error: cannot write report: {report_error}");
                }
                return 1;
            }
            code
        }
    }
}
