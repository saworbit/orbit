use predicates::prelude::*;

#[test]
fn help_exposes_only_plan() {
    assert_cmd::cargo::cargo_bin_cmd!("orbit")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("copy").not());
}

#[test]
fn invalid_arguments_exit_two() {
    assert_cmd::cargo::cargo_bin_cmd!("orbit")
        .args(["plan", "source-only"])
        .assert()
        .code(2);
}
