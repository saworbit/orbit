use std::fs;
use std::path::{Path, PathBuf};

use predicates::prelude::*;

fn orbit(cwd: &Path) -> assert_cmd::Command {
    let mut command = assert_cmd::cargo::cargo_bin_cmd!("orbit");
    command.current_dir(cwd);
    command
}

fn fingerprint(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    fn visit(root: &Path, path: &Path, entries: &mut Vec<(PathBuf, Option<Vec<u8>>)>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        entries.push((
            path.strip_prefix(root).unwrap().to_path_buf(),
            metadata.is_file().then(|| fs::read(path).unwrap()),
        ));
        if metadata.is_dir() {
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
    entries
}

#[test]
fn help_exposes_only_plan() {
    let temp = tempfile::tempdir().unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("copy").not());

    assert_eq!(fingerprint(temp.path()), before);
}

#[test]
fn invalid_arguments_exit_two() {
    let temp = tempfile::tempdir().unwrap();
    let before = fingerprint(temp.path());

    orbit(temp.path())
        .args(["plan", "source-only"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Usage:"));

    assert_eq!(fingerprint(temp.path()), before);
}
