use orbit::core::resume::{save_resume_info_full, ResumeInfo};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_atomic_resume_save() {
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().join("dest.bin");

    let mut info = ResumeInfo {
        bytes_copied: 42,
        compressed_bytes: Some(21),
        ..ResumeInfo::default()
    };
    info.verified_chunks.insert(0, "deadbeef".to_string());
    info.verified_windows.push(7);

    save_resume_info_full(&dest_path, &info, false).unwrap();

    let resume_path = dest_path.with_extension("orbit_resume");
    let temp_extension = resume_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!("{ext}.tmp"))
        .unwrap_or_else(|| "tmp".to_string());
    let temp_path = resume_path.with_extension(temp_extension);

    assert!(
        resume_path.exists(),
        "resume file should be present after save"
    );
    assert!(
        !temp_path.exists(),
        "temp resume file should be removed once rename succeeds"
    );

    let content = fs::read_to_string(resume_path).unwrap();
    assert!(content.contains("\"bytes_copied\": 42"));
    assert!(
        content.contains("deadbeef"),
        "serialized resume content should be intact"
    );
}

#[test]
fn test_crash_simulation_leaves_temp_and_preserves_previous_state() {
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().join("dest_crash.bin");

    let original = ResumeInfo {
        bytes_copied: 10,
        compressed_bytes: None,
        ..ResumeInfo::default()
    };
    save_resume_info_full(&dest_path, &original, false).unwrap();

    let resume_path = dest_path.with_extension("orbit_resume");
    let temp_extension = resume_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!("{ext}.tmp"))
        .unwrap_or_else(|| "tmp".to_string());
    let temp_path = resume_path.with_extension(temp_extension);

    let baseline = fs::read_to_string(&resume_path).unwrap();

    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--ignored")
        .arg("--exact")
        .arg("resume_crash_helper")
        .env("ORBIT_RESUME_CRASH_HELPER", "1")
        .env(
            "ORBIT_RESUME_DEST_PATH",
            dest_path.to_string_lossy().to_string(),
        )
        .env("ORBIT_RESUME_SLEEP_BEFORE_RENAME_MS", "3000")
        .spawn()
        .expect("failed to spawn helper");

    let mut seen_temp = false;
    for _ in 0..30 {
        if temp_path.exists() {
            seen_temp = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    assert!(
        seen_temp,
        "temp file should appear while helper is sleeping"
    );

    child.kill().ok();
    child.wait().ok();

    let final_content = fs::read_to_string(&resume_path).unwrap();
    assert_eq!(
        final_content, baseline,
        "existing resume file should remain intact when crash occurs before rename"
    );
    assert!(
        temp_path.exists(),
        "temp file should remain if process crashes before rename"
    );
}

#[ignore]
#[test]
fn resume_crash_helper() {
    if std::env::var("ORBIT_RESUME_CRASH_HELPER").is_err() {
        return;
    }

    let dest_path = PathBuf::from(
        std::env::var("ORBIT_RESUME_DEST_PATH").expect("dest path required for crash helper"),
    );

    let mut info = ResumeInfo {
        bytes_copied: 1337,
        compressed_bytes: Some(9001),
        ..ResumeInfo::default()
    };
    info.verified_chunks.insert(1, "feedface".to_string());
    info.verified_windows.push(99);

    save_resume_info_full(&dest_path, &info, false).unwrap();
}
