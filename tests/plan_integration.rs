use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use orbit::app::run_plan;
use orbit::request::{OutputMode, PlanRequest, VerifyMode};
use predicates::prelude::*;

fn orbit(cwd: &Path) -> assert_cmd::Command {
    let mut command = assert_cmd::cargo::cargo_bin_cmd!("orbit");
    command.current_dir(cwd);
    command
}

#[derive(Debug, Eq, PartialEq)]
struct TreeFingerprint(Vec<(PathBuf, NodeFingerprint)>);

#[derive(Debug, Eq, PartialEq)]
enum NodeFingerprint {
    Directory { read_only: bool },
    File { contents: Vec<u8>, read_only: bool },
    Symlink { target: PathBuf, read_only: bool },
    Other { read_only: bool },
}

fn fingerprint(root: &Path) -> TreeFingerprint {
    fn visit(root: &Path, path: &Path, entries: &mut Vec<(PathBuf, NodeFingerprint)>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        let read_only = metadata.permissions().readonly();
        let kind = if metadata.file_type().is_symlink() {
            NodeFingerprint::Symlink {
                target: fs::read_link(path).unwrap(),
                read_only,
            }
        } else if metadata.is_file() {
            NodeFingerprint::File {
                contents: fs::read(path).unwrap(),
                read_only,
            }
        } else if metadata.is_dir() {
            NodeFingerprint::Directory { read_only }
        } else {
            NodeFingerprint::Other { read_only }
        };
        entries.push((relative, kind));

        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            let mut children: Vec<_> = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            children.sort();
            for child in children {
                visit(root, &child, entries);
            }
        }
    }

    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    TreeFingerprint(entries)
}

fn assert_no_planner_artifacts(root: &Path) {
    fn visit(path: &Path) {
        let metadata = fs::symlink_metadata(path).unwrap();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        assert_ne!(
            name, ".orbit-state",
            "planner created an Orbit state directory"
        );
        assert!(
            !name.ends_with(".partial") && !name.starts_with(".orbit-partial"),
            "planner created a temporary artifact: {}",
            path.display()
        );
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            for child in fs::read_dir(path).unwrap() {
                visit(&child.unwrap().path());
            }
        }
    }
    visit(root);
}

fn assert_tree_unchanged(root: &Path, before: &TreeFingerprint) {
    assert_eq!(&fingerprint(root), before);
    assert_no_planner_artifacts(root);
}

fn json_lines(bytes: &[u8]) -> Vec<serde_json::Value> {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn assert_json_path(value: &serde_json::Value) {
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), 2);
    assert!(matches!(
        object["encoding"].as_str(),
        Some("utf8" | "unix_bytes_hex" | "windows_utf16le_hex")
    ));
    assert!(object["value"].is_string());
}

#[test]
fn plan_missing_destination_reports_copy_and_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let destination = temp.path().join("destination.txt");
    fs::write(&source, b"payload").unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("COPY"));

    assert!(!destination.exists());
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn relative_endpoints_resolve_inside_the_fingerprinted_cwd_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.txt"), b"payload").unwrap();
    let before = fingerprint(temp.path());

    let output = orbit(temp.path())
        .args(["plan", "source.txt", "destination.txt", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let values = json_lines(&output.stdout);
    assert_eq!(values[0]["data"]["disposition"], "copy");
    assert_eq!(
        PathBuf::from(values[0]["data"]["source"]["value"].as_str().unwrap()),
        fs::canonicalize(temp.path()).unwrap().join("source.txt")
    );
    assert_eq!(
        PathBuf::from(values[0]["data"]["destination"]["value"].as_str().unwrap()),
        fs::canonicalize(temp.path())
            .unwrap()
            .join("destination.txt")
    );
    assert!(!temp.path().join("destination.txt").exists());
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn identical_destination_reports_skip_and_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::write(&source, b"identical").unwrap();
    fs::write(&destination, b"identical").unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("SKIP"));

    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn conflicting_destination_exits_three_and_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::write(&source, b"source").unwrap();
    fs::write(&destination, b"before").unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("CONFLICT"));

    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn replace_reports_replace_but_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::write(&source, b"new").unwrap();
    fs::write(&destination, b"old").unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
            "--replace",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("REPLACE"));

    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn json_is_deterministic_ndjson_with_path_objects_and_final_plan_result() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::write(&source, b"payload").unwrap();
    let before = fingerprint(temp.path());
    let args = [
        "plan",
        source.to_str().unwrap(),
        destination.to_str().unwrap(),
        "--json",
    ];

    let first = orbit(temp.path()).args(args).output().unwrap();
    let second = orbit(temp.path()).args(args).output().unwrap();

    assert!(first.status.success());
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert!(first.stderr.is_empty());
    let values = json_lines(&first.stdout);
    assert_eq!(values.last().unwrap()["event"], "plan_result");
    assert!(values.iter().all(|value| value["schema_version"] == 1));
    assert_json_path(&values[0]["data"]["relative_path"]);
    assert_json_path(&values[0]["data"]["source"]);
    assert_json_path(&values[0]["data"]["destination"]);
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn directory_destination_is_the_exact_root_not_a_basename_container() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("photos");
    let destination = temp.path().join("archive");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("one.jpg"), b"one").unwrap();
    let before = fingerprint(temp.path());

    let output = orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let values = json_lines(&output.stdout);
    let entries: Vec<_> = values
        .iter()
        .filter(|value| value["event"] == "plan_entry")
        .collect();
    assert_eq!(entries.len(), 2);
    let root = entries
        .iter()
        .find(|entry| entry["data"]["relative_path"]["value"] == "")
        .unwrap();
    let child = entries
        .iter()
        .find(|entry| entry["data"]["relative_path"]["value"] == "one.jpg")
        .unwrap();
    let resolved_parent = fs::canonicalize(temp.path()).unwrap();
    assert_eq!(
        PathBuf::from(root["data"]["destination"]["value"].as_str().unwrap()),
        resolved_parent.join("archive")
    );
    assert_eq!(
        PathBuf::from(child["data"]["destination"]["value"].as_str().unwrap()),
        resolved_parent.join("archive/one.jpg")
    );
    assert_ne!(
        PathBuf::from(child["data"]["destination"]["value"].as_str().unwrap()),
        resolved_parent.join("archive/photos/one.jpg")
    );
    assert!(!destination.exists());
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn unrelated_destination_entries_are_not_planned_or_changed() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&destination).unwrap();
    fs::write(source.join("new.txt"), b"new").unwrap();
    fs::write(destination.join("unrelated.txt"), b"keep").unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("unrelated.txt").not());

    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn json_domain_error_is_the_only_stdout_event_and_stderr_is_clean() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("missing");
    let destination = temp.path().join("target");
    let before = fingerprint(temp.path());

    let output = orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let values = json_lines(&output.stdout);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["schema_version"], 1);
    assert_eq!(values[0]["event"], "error");
    assert_eq!(values[0]["data"]["code"], "source_missing");
    assert_eq!(values[0]["data"]["exit_code"], 3);
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn quiet_errors_remain_visible_on_stderr() {
    let temp = tempfile::tempdir().unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            temp.path().join("missing").to_str().unwrap(),
            temp.path().join("target").to_str().unwrap(),
            "--quiet",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("source does not exist"));

    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn quiet_success_suppresses_the_plan() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::write(&source, b"payload").unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            temp.path().join("target").to_str().unwrap(),
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn quiet_conflict_reports_exact_blocking_paths_and_reasons_on_stderr() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&destination).unwrap();
    for name in ["a.txt", "b.txt"] {
        fs::write(source.join(name), b"source").unwrap();
        fs::write(destination.join(name), b"before").unwrap();
    }
    let before = fingerprint(temp.path());
    let output = orbit(temp.path())
        .args(["plan", "source", "destination", "--quiet"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let resolved_destination = fs::canonicalize(&destination).unwrap();
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "error: CONFLICT {} — destination file has different contents\nerror: CONFLICT {} — destination file has different contents\n",
            resolved_destination.join("a.txt").to_str().unwrap(),
            resolved_destination.join("b.txt").to_str().unwrap(),
        )
    );
    assert_tree_unchanged(temp.path(), &before);
}

#[cfg(unix)]
#[test]
fn quiet_unsupported_socket_reports_source_and_can_be_explicitly_ignored() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.socket");
    let _socket = std::os::unix::net::UnixListener::bind(&source).unwrap();
    let before = fingerprint(temp.path());
    let output = orbit(temp.path())
        .args(["plan", "source.socket", "target", "--quiet"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "error: UNSUPPORTED {} — source entry has unsupported fidelity\n",
            fs::canonicalize(&source).unwrap().to_str().unwrap(),
        )
    );
    orbit(temp.path())
        .args([
            "plan",
            "source.socket",
            "target",
            "--quiet",
            "--ignore-unsupported",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn destination_inside_source_exits_three_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            source.join("nested/target").to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("inside the source tree"));

    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn symbolic_link_is_planned_without_traversing_its_target() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target-directory");
    let source = temp.path().join("source-link");
    let destination = temp.path().join("destination-link");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("must-not-be-scanned.txt"), b"sentinel").unwrap();
    if create_directory_link(&target, &source) == SymlinkCapability::PermissionDenied {
        eprintln!("skipping symlink assertion: symbolic-link creation is not permitted");
        return;
    }
    let before = fingerprint(temp.path());

    let output = orbit(temp.path())
        .args([
            "plan",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let values = json_lines(&output.stdout);
    let entries: Vec<_> = values
        .iter()
        .filter(|value| value["event"] == "plan_entry")
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["data"]["kind"], "symlink");
    assert_eq!(entries[0]["data"]["relative_path"]["value"], "");
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("must-not-be-scanned.txt")
    );
    assert_tree_unchanged(temp.path(), &before);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SymlinkCapability {
    Created,
    PermissionDenied,
}

#[cfg(unix)]
fn create_directory_link(target: &Path, link: &Path) -> SymlinkCapability {
    std::os::unix::fs::symlink(target, link).unwrap();
    SymlinkCapability::Created
}

#[cfg(windows)]
fn create_directory_link(target: &Path, link: &Path) -> SymlinkCapability {
    match std::os::windows::fs::symlink_dir(target, link) {
        Ok(()) => SymlinkCapability::Created,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            SymlinkCapability::PermissionDenied
        }
        Err(error) => panic!("cannot create test symlink: {error}"),
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FlushFailingSink;

impl Write for FlushFailingSink {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
    }
}

struct InteractionForbiddenWriter;

impl Write for InteractionForbiddenWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        panic!("quiet success must not write")
    }

    fn flush(&mut self) -> io::Result<()> {
        panic!("quiet success must not flush")
    }
}

fn request(source: PathBuf, destination: PathBuf, output: OutputMode) -> PlanRequest {
    PlanRequest {
        source,
        destination,
        replace: false,
        verify: VerifyMode::Hash,
        ignore_unsupported: false,
        output,
    }
}

#[test]
fn plan_writer_failure_returns_internal_error_and_reports_to_stderr() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::write(&source, b"payload").unwrap();
    let before = fingerprint(temp.path());
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();

    let exit = run_plan(
        request(source, temp.path().join("target"), OutputMode::Human),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("cannot write report")
    );
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn json_error_writer_failure_returns_internal_error_and_uses_stderr_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let before = fingerprint(temp.path());
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();

    let exit = run_plan(
        request(
            temp.path().join("missing"),
            temp.path().join("target"),
            OutputMode::Json,
        ),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("cannot write report")
    );
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn human_error_writer_failure_never_returns_the_domain_exit_code_silently() {
    let temp = tempfile::tempdir().unwrap();
    let before = fingerprint(temp.path());
    let mut stdout = Vec::new();
    let mut stderr = FailingWriter;

    let exit = run_plan(
        request(
            temp.path().join("missing"),
            temp.path().join("target"),
            OutputMode::Human,
        ),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    assert_tree_unchanged(temp.path(), &before);
}

#[test]
fn human_plan_flush_failure_returns_internal_error_with_stderr_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::write(&source, b"payload").unwrap();
    let mut stdout = BufWriter::new(FlushFailingSink);
    let mut stderr = Vec::new();

    let exit = run_plan(
        request(source, temp.path().join("target"), OutputMode::Human),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("cannot write report: flush failed")
    );
}

#[test]
fn json_plan_flush_failure_returns_internal_error_with_stderr_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::write(&source, b"payload").unwrap();
    let mut stdout = BufWriter::new(FlushFailingSink);
    let mut stderr = Vec::new();

    let exit = run_plan(
        request(source, temp.path().join("target"), OutputMode::Json),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("cannot write report: flush failed")
    );
}

#[test]
fn json_domain_error_flush_failure_returns_internal_error_with_stderr_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let mut stdout = BufWriter::new(FlushFailingSink);
    let mut stderr = Vec::new();

    let exit = run_plan(
        request(
            temp.path().join("missing"),
            temp.path().join("target"),
            OutputMode::Json,
        ),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("cannot write report: flush failed")
    );
}

#[test]
fn human_domain_error_flush_failure_returns_internal_error() {
    let temp = tempfile::tempdir().unwrap();
    let mut stdout = Vec::new();
    let mut stderr = BufWriter::new(FlushFailingSink);

    let exit = run_plan(
        request(
            temp.path().join("missing"),
            temp.path().join("target"),
            OutputMode::Human,
        ),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
}

#[test]
fn quiet_domain_error_flush_failure_returns_internal_error() {
    let temp = tempfile::tempdir().unwrap();
    let mut stdout = Vec::new();
    let mut stderr = BufWriter::new(FlushFailingSink);

    let exit = run_plan(
        request(
            temp.path().join("missing"),
            temp.path().join("target"),
            OutputMode::Quiet,
        ),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
}

#[test]
fn quiet_success_does_not_interact_with_either_stream() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::write(&source, b"payload").unwrap();
    let mut stdout = InteractionForbiddenWriter;
    let mut stderr = InteractionForbiddenWriter;

    let exit = run_plan(
        request(source, temp.path().join("target"), OutputMode::Quiet),
        temp.path(),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit, 0);
}

#[test]
fn quiet_blocked_plan_report_write_and_flush_failures_return_internal_error() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::write(&source, b"source").unwrap();
    fs::write(&destination, b"before").unwrap();
    let mut stdout = InteractionForbiddenWriter;
    assert_eq!(
        run_plan(
            request(source.clone(), destination.clone(), OutputMode::Quiet),
            temp.path(),
            &mut stdout,
            &mut FailingWriter
        ),
        1
    );
    assert_eq!(
        run_plan(
            request(source.clone(), destination.clone(), OutputMode::Quiet),
            temp.path(),
            &mut stdout,
            &mut BufWriter::new(FailingWriter)
        ),
        1
    );
    assert_eq!(
        run_plan(
            request(source, destination, OutputMode::Quiet),
            temp.path(),
            &mut stdout,
            &mut BufWriter::new(FlushFailingSink)
        ),
        1
    );
}
