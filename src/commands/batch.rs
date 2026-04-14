use std::path::PathBuf;
use std::sync::Arc;

use crate::cli_style::{format_duration, print_info, section_header, Icons, Theme};
use crate::error::{OrbitError, Result};

pub fn split_command_line(line: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if !in_single => {
                if let Some(next) = chars.peek().copied() {
                    if next.is_whitespace() || next == '"' || next == '\'' || next == '\\' {
                        chars.next();
                        current.push(next);
                    } else {
                        current.push('\\');
                    }
                } else {
                    current.push('\\');
                }
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
                while let Some(next) = chars.peek() {
                    if next.is_whitespace() {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            _ => current.push(c),
        }
    }

    if in_single || in_double {
        return Err(OrbitError::Config(
            "Unclosed quote in batch command line".to_string(),
        ));
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

pub fn normalize_batch_args(line: &str, tokens: Vec<String>) -> Result<Vec<String>> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let first = tokens[0].as_str();

    if first == "orbit" {
        let args = tokens[1..].to_vec();
        if args.first().map(|s| s.as_str()) == Some("run") {
            return Err(OrbitError::Config(
                "Nested 'orbit run' is not supported in batch mode".to_string(),
            ));
        }
        return Ok(args);
    }

    if first == "cp" || first == "copy" || first == "sync" {
        if tokens.len() < 3 {
            return Err(OrbitError::Config(format!(
                "Invalid command (need: {} <src> <dest>): {}",
                first, line
            )));
        }

        let mut args = vec![
            "--source".to_string(),
            tokens[1].clone(),
            "--dest".to_string(),
            tokens[2].clone(),
        ];

        if first == "sync" {
            args.push("--mode".to_string());
            args.push("sync".to_string());
        }

        args.extend(tokens[3..].iter().cloned());
        return Ok(args);
    }

    if first == "run" {
        return Err(OrbitError::Config(
            "Nested 'orbit run' is not supported in batch mode".to_string(),
        ));
    }

    Ok(tokens)
}

pub fn handle_run_command(file: Option<PathBuf>, workers: usize) -> Result<()> {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    section_header(&format!("{} Batch Execution", Icons::ROCKET));
    println!();

    // Read commands from file or stdin
    let lines: Vec<String> = if let Some(path) = file {
        let file = std::fs::File::open(&path).map_err(|e| {
            OrbitError::Other(format!(
                "Failed to open command file {}: {}",
                path.display(),
                e
            ))
        })?;
        std::io::BufReader::new(file)
            .lines()
            .map_while(|line| line.ok())
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .collect()
    } else {
        let stdin = std::io::stdin();
        stdin
            .lock()
            .lines()
            .map_while(|line| line.ok())
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .collect()
    };

    if lines.is_empty() {
        print_info("No commands to execute.");
        return Ok(());
    }

    let mut parsed_commands = Vec::new();
    let mut invalid = 0usize;

    for line in &lines {
        let tokens = match split_command_line(line) {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("WARN: {}: {}", line, e);
                invalid += 1;
                continue;
            }
        };

        if tokens.is_empty() {
            continue;
        }

        match normalize_batch_args(line, tokens) {
            Ok(args) => {
                if !args.is_empty() {
                    parsed_commands.push((line.clone(), args));
                }
            }
            Err(e) => {
                eprintln!("WARN: {}: {}", line, e);
                invalid += 1;
            }
        }
    }

    let total = parsed_commands.len() + invalid;
    let worker_count = if workers == 0 { 1 } else { workers };

    if parsed_commands.is_empty() {
        section_header(&format!("{} Batch Complete", Icons::SUCCESS));
        println!();
        println!(
            "  {} {} {} succeeded, {} failed in {}",
            Icons::BULLET,
            Theme::value(total),
            Theme::success(0),
            Theme::error(invalid),
            Theme::value(format_duration(0.0))
        );
        println!();
        return Ok(());
    }

    println!(
        "  {} {} commands with {} workers",
        Icons::BULLET,
        Theme::value(total),
        Theme::value(worker_count)
    );
    println!();

    let succeeded = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(invalid));
    let start = Instant::now();

    let exe = std::env::current_exe()
        .map_err(|e| OrbitError::Other(format!("Failed to resolve current executable: {}", e)))?;

    // Use a thread pool to execute commands in parallel
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count.min(parsed_commands.len().max(1)))
        .build()
        .map_err(|e| OrbitError::Other(format!("Failed to create thread pool: {}", e)))?;

    pool.scope(|s| {
        for (cmd_line, args) in &parsed_commands {
            let succeeded = succeeded.clone();
            let failed = failed.clone();
            let exe = exe.clone();
            let args = args.clone();
            let cmd_line = cmd_line.clone();
            s.spawn(move |_| {
                let status = Command::new(&exe)
                    .args(&args)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status();

                match status {
                    Ok(status) if status.success() => {
                        succeeded.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(status) => {
                        let code = status.code().unwrap_or(-1);
                        eprintln!("ERROR: command failed (exit {}): {}", code, cmd_line);
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("ERROR: failed to run '{}': {}", cmd_line, e);
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });

    let elapsed = start.elapsed();
    let ok = succeeded.load(Ordering::Relaxed);
    let err = failed.load(Ordering::Relaxed);

    println!();
    section_header(&format!("{} Batch Complete", Icons::SUCCESS));
    println!();
    println!(
        "  {} {} {} succeeded, {} failed in {}",
        Icons::BULLET,
        Theme::value(total),
        Theme::success(ok),
        if err > 0 {
            Theme::error(err).to_string()
        } else {
            Theme::muted(err).to_string()
        },
        Theme::value(format_duration(elapsed.as_secs_f64()))
    );
    println!();

    Ok(())
}
