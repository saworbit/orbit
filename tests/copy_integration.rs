use std::fs;
use std::path::Path;

use predicates::prelude::*;

fn orbit(cwd: &Path) -> assert_cmd::Command {
    let mut command = assert_cmd::cargo::cargo_bin_cmd!("orbit");
    command.current_dir(cwd);
    command
}

fn partials(root: &Path) -> Vec<String> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".orbit-") && name.ends_with(".partial"))
        .collect()
}

#[test]
fn copies_one_regular_file_to_a_new_destination() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"trusted payload").unwrap();

    orbit(temp.path())
        .args(["copy", "source.bin", "destination.bin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Copied"));

    assert_eq!(
        fs::read(temp.path().join("destination.bin")).unwrap(),
        b"trusted payload"
    );
    assert!(partials(temp.path()).is_empty());
}

#[test]
fn supports_explicit_size_verification() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"payload").unwrap();

    orbit(temp.path())
        .args([
            "copy",
            "source.bin",
            "destination.bin",
            "--verify",
            "size",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    assert_eq!(
        fs::read(temp.path().join("destination.bin")).unwrap(),
        b"payload"
    );
}

#[test]
fn identical_destination_is_a_successful_no_op() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"same").unwrap();
    fs::write(temp.path().join("destination.bin"), b"same").unwrap();

    orbit(temp.path())
        .args(["copy", "source.bin", "destination.bin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Already identical"));

    assert_eq!(
        fs::read(temp.path().join("destination.bin")).unwrap(),
        b"same"
    );
    assert!(partials(temp.path()).is_empty());
}

#[test]
fn different_destination_is_refused_and_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"new").unwrap();
    fs::write(temp.path().join("destination.bin"), b"keep this").unwrap();

    orbit(temp.path())
        .args(["copy", "source.bin", "destination.bin"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("destination already exists"));

    assert_eq!(
        fs::read(temp.path().join("destination.bin")).unwrap(),
        b"keep this"
    );
    assert!(partials(temp.path()).is_empty());
}

#[test]
fn directory_source_is_refused_without_creating_destination() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("source")).unwrap();

    orbit(temp.path())
        .args(["copy", "source", "destination"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("regular file"));

    assert!(!temp.path().join("destination").exists());
    assert!(partials(temp.path()).is_empty());
}

#[test]
fn missing_destination_parent_is_refused_without_creating_directories() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"payload").unwrap();

    orbit(temp.path())
        .args(["copy", "source.bin", "missing/destination.bin"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("destination parent"));

    assert!(!temp.path().join("missing").exists());
    assert!(partials(temp.path()).is_empty());
}

#[test]
fn trailing_separator_destination_is_refused_without_creating_a_file() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"payload").unwrap();

    #[cfg(windows)]
    let destinations = ["new_dir/", "new_dir\\"];
    #[cfg(not(windows))]
    let destinations = ["new_dir/"];

    for destination in destinations {
        orbit(temp.path())
            .args(["copy", "source.bin", destination])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("trailing path separator"));

        assert!(!temp.path().join("new_dir").exists());
        assert!(partials(temp.path()).is_empty());
    }
}

#[test]
fn json_success_is_one_machine_readable_result() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"payload").unwrap();

    let output = orbit(temp.path())
        .args(["copy", "source.bin", "destination.bin", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let lines: Vec<_> = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(lines.len(), 1);
    let value: serde_json::Value = serde_json::from_slice(lines[0]).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["event"], "copy_result");
    assert_eq!(value["data"]["status"], "copied");
    assert_eq!(value["data"]["verification"], "hash");
}

#[test]
fn json_refusal_is_one_error_event_on_stdout() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"new").unwrap();
    fs::write(temp.path().join("destination.bin"), b"old").unwrap();

    let output = orbit(temp.path())
        .args(["copy", "source.bin", "destination.bin", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["event"], "error");
    assert_eq!(value["data"]["code"], "destination_exists");
}

#[test]
fn quiet_refusal_remains_visible_on_stderr() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"new").unwrap();
    fs::write(temp.path().join("destination.bin"), b"old").unwrap();

    orbit(temp.path())
        .args(["copy", "source.bin", "destination.bin", "--quiet"])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("destination already exists"));
}

#[cfg(windows)]
#[test]
fn error_messages_use_ordinary_path_spelling_on_windows() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.bin"), b"payload").unwrap();

    let output = orbit(temp.path())
        .args(["copy", "source.bin", "missing/destination.bin"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains(r"\\?\"),
        "error message leaked verbatim prefix: {stderr}"
    );
    assert!(
        stderr.contains("destination parent"),
        "unexpected message: {stderr}"
    );
}

#[test]
fn symbolic_link_source_is_refused_without_following_it() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target.bin");
    let source = temp.path().join("source-link.bin");
    fs::write(&target, b"must not be copied through a link").unwrap();
    if !create_file_link(&target, &source) {
        return;
    }

    orbit(temp.path())
        .args(["copy", "source-link.bin", "destination.bin"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("regular file"));

    assert!(!temp.path().join("destination.bin").exists());
    assert!(partials(temp.path()).is_empty());
}

#[cfg(unix)]
fn create_file_link(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).unwrap();
    true
}

#[cfg(windows)]
fn create_file_link(target: &Path, link: &Path) -> bool {
    match std::os::windows::fs::symlink_file(target, link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(error) => panic!("cannot create test symlink: {error}"),
    }
}
