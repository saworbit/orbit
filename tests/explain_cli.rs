use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn test_explain_cli_reports_advanced_plan_details() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    let link_dest = tmp.path().join("snapshots");
    let batch = tmp.path().join("batch.orbit");

    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::create_dir_all(&link_dest).unwrap();
    std::fs::write(src.join("file.txt"), b"hello").unwrap();

    let mut cmd = Command::cargo_bin("orbit").unwrap();
    cmd.arg("--mode")
        .arg("mirror")
        .arg("--detect-renames")
        .arg("--link-dest")
        .arg(&link_dest)
        .arg("--write-batch")
        .arg(&batch)
        .arg("--include")
        .arg("*.txt")
        .arg("--exclude")
        .arg("*.tmp")
        .arg("--no-clobber")
        .arg("--if-source-newer")
        .arg("--dry-run")
        .arg("explain")
        .arg(&src)
        .arg(&dest);

    cmd.assert()
        .success()
        .stdout(contains("Transfer Plan"))
        .stdout(contains("Mirror"))
        .stdout(contains("Recursive"))
        .stdout(contains("Filters"))
        .stdout(contains("Advanced"))
        .stdout(contains("Conditions"))
        .stdout(contains("DRY RUN"));
}
