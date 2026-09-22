use std::io::{self, Write};
use std::path::Path;

use crate::error::OrbitError;
use crate::plan::{CopyPlan, Disposition, PlanEntry};
use crate::request::OutputMode;

pub fn write_plan(writer: &mut impl Write, mode: OutputMode, plan: &CopyPlan) -> io::Result<()> {
    match mode {
        OutputMode::Quiet => Ok(()),
        OutputMode::Human => write_human_plan(writer, plan),
        OutputMode::Json => write_json_plan(writer, plan),
    }
}

pub fn write_blocking_diagnostics(writer: &mut impl Write, plan: &CopyPlan) -> io::Result<()> {
    for entry in plan.entries.iter().filter(|entry| entry.blocking) {
        let path = if entry.disposition == Disposition::Unsupported {
            &entry.source
        } else {
            &entry.destination
        };
        writeln!(
            writer,
            "error: {} {} — {}",
            disposition_label(entry.disposition),
            display_path(path),
            entry.reason
        )?;
    }
    Ok(())
}

pub fn write_error(
    writer: &mut impl Write,
    mode: OutputMode,
    error: &OrbitError,
) -> io::Result<()> {
    let message = error_message(error);
    match mode {
        OutputMode::Json => write_json_line(
            writer,
            &JsonEvent {
                schema_version: 1,
                event: "error",
                data: ErrorData {
                    code: error.code(),
                    exit_code: error.exit_code(),
                    message,
                },
            },
        ),
        OutputMode::Human | OutputMode::Quiet => writeln!(writer, "error: {message}"),
    }
}

fn write_human_plan(writer: &mut impl Write, plan: &CopyPlan) -> io::Result<()> {
    writeln!(writer, "Source: {}", display_path(&plan.source))?;
    writeln!(writer, "Destination: {}", display_path(&plan.destination))?;
    for entry in &plan.entries {
        writeln!(
            writer,
            "{} {} — {}",
            disposition_label(entry.disposition),
            display_path(&entry.relative_path),
            entry.reason
        )?;
    }

    let blocking = blocking_entries(plan);
    if blocking == 0 {
        writeln!(writer, "Plan ready: {} entries", plan.entries.len())
    } else {
        let noun = if blocking == 1 {
            "conflict"
        } else {
            "conflicts"
        };
        writeln!(writer, "Plan blocked: {blocking} {noun}")
    }
}

fn write_json_plan(writer: &mut impl Write, plan: &CopyPlan) -> io::Result<()> {
    for entry in &plan.entries {
        write_json_line(
            writer,
            &JsonEvent {
                schema_version: 1,
                event: "plan_entry",
                data: JsonPlanEntry::from(entry),
            },
        )?;
    }

    write_json_line(
        writer,
        &JsonEvent {
            schema_version: 1,
            event: "plan_result",
            data: PlanResult {
                operation_id: &plan.operation_id,
                executable: plan.is_executable(),
                entries: plan.entries.len(),
                blocking: blocking_entries(plan),
            },
        },
    )
}

fn blocking_entries(plan: &CopyPlan) -> usize {
    plan.entries.iter().filter(|entry| entry.blocking).count()
}

fn disposition_label(disposition: Disposition) -> &'static str {
    match disposition {
        Disposition::Copy => "COPY",
        Disposition::SkipIdentical => "SKIP",
        Disposition::Replace => "REPLACE",
        Disposition::Conflict => "CONFLICT",
        Disposition::Unsupported => "UNSUPPORTED",
    }
}

fn error_message(error: &OrbitError) -> String {
    match error {
        OrbitError::SourceMissing(path) => format!("source does not exist: {}", display_path(path)),
        OrbitError::SameEndpoint(path) => format!(
            "source and destination resolve to the same path: {}",
            display_path(path)
        ),
        OrbitError::DestinationInsideSource { destination } => {
            format!(
                "destination is inside the source tree: {}",
                display_path(destination)
            )
        }
        OrbitError::Io {
            operation,
            path,
            source,
        } => format!("cannot {operation} {}: {source}", display_path(path)),
    }
}

#[derive(serde::Serialize)]
struct JsonEvent<'a, T: serde::Serialize> {
    schema_version: u32,
    event: &'a str,
    data: T,
}

#[derive(serde::Serialize)]
struct JsonPath {
    encoding: &'static str,
    value: String,
}

impl JsonPath {
    fn from_path(path: &Path) -> Self {
        match path.to_str() {
            Some(value) => Self {
                encoding: "utf8",
                value: value.to_owned(),
            },
            None => Self {
                encoding: native_path_encoding(),
                value: native_path_hex(path),
            },
        }
    }
}

#[derive(serde::Serialize)]
struct JsonPlanEntry<'a> {
    relative_path: JsonPath,
    source: JsonPath,
    destination: JsonPath,
    kind: crate::scan::EntryKind,
    disposition: Disposition,
    length: u64,
    source_digest: Option<&'a str>,
    reason: &'a str,
    blocking: bool,
}

impl<'a> From<&'a PlanEntry> for JsonPlanEntry<'a> {
    fn from(entry: &'a PlanEntry) -> Self {
        Self {
            relative_path: JsonPath::from_path(&entry.relative_path),
            source: JsonPath::from_path(&entry.source),
            destination: JsonPath::from_path(&entry.destination),
            kind: entry.kind,
            disposition: entry.disposition,
            length: entry.length,
            source_digest: entry.source_digest.as_deref(),
            reason: &entry.reason,
            blocking: entry.blocking,
        }
    }
}

#[derive(serde::Serialize)]
struct PlanResult<'a> {
    operation_id: &'a str,
    executable: bool,
    entries: usize,
    blocking: usize,
}

#[derive(serde::Serialize)]
struct ErrorData<'a> {
    code: &'a str,
    exit_code: u8,
    message: String,
}

fn write_json_line(writer: &mut impl Write, value: &impl serde::Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writeln!(writer)
}

fn display_path(path: &Path) -> String {
    match path.to_str() {
        Some(value) => value.to_owned(),
        None => format!("[{}:{}]", native_path_encoding(), native_path_hex(path)),
    }
}

#[cfg(unix)]
fn native_path_encoding() -> &'static str {
    "unix_bytes_hex"
}

#[cfg(windows)]
fn native_path_encoding() -> &'static str {
    "windows_utf16le_hex"
}

#[cfg(unix)]
fn native_path_hex(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;

    hex_encode(path.as_os_str().as_bytes())
}

#[cfg(windows)]
fn native_path_hex(path: &Path) -> String {
    use std::os::windows::ffi::OsStrExt;

    let bytes: Vec<u8> = path
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect();
    hex_encode(&bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{write_blocking_diagnostics, write_error, write_plan};
    use crate::error::OrbitError;
    use crate::plan::{CopyPlan, Disposition, PlanEntry};
    use crate::request::OutputMode;
    use crate::scan::EntryKind;
    use std::path::PathBuf;

    fn fixture() -> CopyPlan {
        CopyPlan {
            operation_id: "0123456789abcdef01234567".into(),
            source: PathBuf::from("C:/source"),
            destination: PathBuf::from("D:/target"),
            entries: vec![
                PlanEntry {
                    relative_path: PathBuf::from("new.txt"),
                    source: PathBuf::from("C:/source/new.txt"),
                    destination: PathBuf::from("D:/target/new.txt"),
                    kind: EntryKind::File,
                    disposition: Disposition::Copy,
                    length: 3,
                    source_digest: Some("abc".into()),
                    reason: "destination does not exist".into(),
                    blocking: false,
                },
                PlanEntry {
                    relative_path: PathBuf::from("blocked.txt"),
                    source: PathBuf::from("C:/source/blocked.txt"),
                    destination: PathBuf::from("D:/target/blocked.txt"),
                    kind: EntryKind::File,
                    disposition: Disposition::Conflict,
                    length: 7,
                    source_digest: Some("def".into()),
                    reason: "destination file has different contents".into(),
                    blocking: true,
                },
            ],
        }
    }

    #[test]
    fn human_report_has_ordered_actions_and_blocking_summary() {
        let mut bytes = Vec::new();
        write_plan(&mut bytes, OutputMode::Human, &fixture()).unwrap();

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "Source: C:/source\nDestination: D:/target\nCOPY new.txt — destination does not exist\nCONFLICT blocked.txt — destination file has different contents\nPlan blocked: 1 conflict\n"
        );
    }

    #[test]
    fn quiet_suppresses_plan_but_not_error() {
        let mut plan_bytes = Vec::new();
        write_plan(&mut plan_bytes, OutputMode::Quiet, &fixture()).unwrap();
        assert!(plan_bytes.is_empty());

        let mut error_bytes = Vec::new();
        write_error(
            &mut error_bytes,
            OutputMode::Quiet,
            &OrbitError::SourceMissing(PathBuf::from("missing")),
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(error_bytes).unwrap(),
            "error: source does not exist: missing\n"
        );
    }

    #[test]
    fn blocking_diagnostics_report_only_conflicts_and_unsupported_source_paths() {
        let mut plan = fixture();
        plan.entries.push(PlanEntry {
            relative_path: PathBuf::from("socket"),
            source: PathBuf::from("C:/source/socket"),
            destination: PathBuf::from("D:/target/socket"),
            kind: EntryKind::Unsupported,
            disposition: Disposition::Unsupported,
            length: 0,
            source_digest: None,
            reason: "source entry has unsupported fidelity".into(),
            blocking: true,
        });
        let mut bytes = Vec::new();
        write_blocking_diagnostics(&mut bytes, &plan).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            concat!(
                "error: CONFLICT D:/target/blocked.txt — destination file has different contents\n",
                "error: UNSUPPORTED C:/source/socket — source entry has unsupported fidelity\n",
            )
        );
        plan.entries[1].blocking = false;
        plan.entries[2].blocking = false;
        let mut bytes = Vec::new();
        write_blocking_diagnostics(&mut bytes, &plan).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn blocking_diagnostics_keep_native_path_units_losslessly() {
        #[cfg(windows)]
        let (path, expected) = {
            use std::os::windows::ffi::OsStringExt;
            (
                PathBuf::from(std::ffi::OsString::from_wide(&[0xd800])),
                "[windows_utf16le_hex:00d8]",
            )
        };
        #[cfg(unix)]
        let (path, expected) = {
            use std::os::unix::ffi::OsStringExt;
            (
                PathBuf::from(std::ffi::OsString::from_vec(vec![0x80])),
                "[unix_bytes_hex:80]",
            )
        };
        let mut plan = fixture();
        plan.entries[1].destination = path;
        let mut bytes = Vec::new();
        write_blocking_diagnostics(&mut bytes, &plan).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            format!("error: CONFLICT {expected} — destination file has different contents\n")
        );
    }

    #[test]
    fn json_is_ordered_versioned_ndjson_with_lossless_path_objects() {
        let mut bytes = Vec::new();
        write_plan(&mut bytes, OutputMode::Json, &fixture()).unwrap();

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            concat!(
                "{\"schema_version\":1,\"event\":\"plan_entry\",\"data\":{\"relative_path\":{\"encoding\":\"utf8\",\"value\":\"new.txt\"},\"source\":{\"encoding\":\"utf8\",\"value\":\"C:/source/new.txt\"},\"destination\":{\"encoding\":\"utf8\",\"value\":\"D:/target/new.txt\"},\"kind\":\"file\",\"disposition\":\"copy\",\"length\":3,\"source_digest\":\"abc\",\"reason\":\"destination does not exist\",\"blocking\":false}}\n",
                "{\"schema_version\":1,\"event\":\"plan_entry\",\"data\":{\"relative_path\":{\"encoding\":\"utf8\",\"value\":\"blocked.txt\"},\"source\":{\"encoding\":\"utf8\",\"value\":\"C:/source/blocked.txt\"},\"destination\":{\"encoding\":\"utf8\",\"value\":\"D:/target/blocked.txt\"},\"kind\":\"file\",\"disposition\":\"conflict\",\"length\":7,\"source_digest\":\"def\",\"reason\":\"destination file has different contents\",\"blocking\":true}}\n",
                "{\"schema_version\":1,\"event\":\"plan_result\",\"data\":{\"operation_id\":\"0123456789abcdef01234567\",\"executable\":false,\"entries\":2,\"blocking\":1}}\n"
            )
        );
    }

    #[test]
    fn json_error_has_stable_code_exit_code_and_message() {
        let mut bytes = Vec::new();
        write_error(
            &mut bytes,
            OutputMode::Json,
            &OrbitError::SourceMissing(PathBuf::from("missing")),
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\"schema_version\":1,\"event\":\"error\",\"data\":{\"code\":\"source_missing\",\"exit_code\":3,\"message\":\"source does not exist: missing\"}}\n"
        );
    }

    #[test]
    fn every_error_variant_has_a_stable_code_and_exit_code() {
        let errors = [
            (
                OrbitError::SourceMissing(PathBuf::from("source")),
                "source_missing",
            ),
            (
                OrbitError::SameEndpoint(PathBuf::from("same")),
                "same_endpoint",
            ),
            (
                OrbitError::DestinationInsideSource {
                    destination: PathBuf::from("inside"),
                },
                "destination_inside_source",
            ),
            (
                OrbitError::Io {
                    operation: "read",
                    path: PathBuf::from("path"),
                    source: std::io::Error::other("denied"),
                },
                "io_error",
            ),
        ];

        for (error, code) in errors {
            assert_eq!(error.code(), code);
            assert_eq!(error.exit_code(), 3);
        }
    }

    #[cfg(unix)]
    #[test]
    fn json_and_human_output_preserve_non_unicode_unix_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let native_path = PathBuf::from(OsString::from_vec(vec![b'a', 0x80]));
        let plan = CopyPlan {
            operation_id: "operation".into(),
            source: native_path.clone(),
            destination: PathBuf::from("target"),
            entries: vec![PlanEntry {
                relative_path: native_path.clone(),
                source: native_path.clone(),
                destination: native_path,
                kind: EntryKind::File,
                disposition: Disposition::Copy,
                length: 0,
                source_digest: None,
                reason: "test".into(),
                blocking: false,
            }],
        };

        let mut json = Vec::new();
        write_plan(&mut json, OutputMode::Json, &plan).unwrap();
        assert_eq!(
            String::from_utf8(json).unwrap(),
            concat!(
                "{\"schema_version\":1,\"event\":\"plan_entry\",\"data\":{\"relative_path\":{\"encoding\":\"unix_bytes_hex\",\"value\":\"6180\"},\"source\":{\"encoding\":\"unix_bytes_hex\",\"value\":\"6180\"},\"destination\":{\"encoding\":\"unix_bytes_hex\",\"value\":\"6180\"},\"kind\":\"file\",\"disposition\":\"copy\",\"length\":0,\"source_digest\":null,\"reason\":\"test\",\"blocking\":false}}\n",
                "{\"schema_version\":1,\"event\":\"plan_result\",\"data\":{\"operation_id\":\"operation\",\"executable\":true,\"entries\":1,\"blocking\":0}}\n"
            )
        );

        let mut human = Vec::new();
        write_plan(&mut human, OutputMode::Human, &plan).unwrap();
        assert_eq!(
            String::from_utf8(human).unwrap(),
            "Source: [unix_bytes_hex:6180]\nDestination: target\nCOPY [unix_bytes_hex:6180] — test\nPlan ready: 1 entries\n"
        );
    }

    #[cfg(windows)]
    #[test]
    fn json_and_human_output_preserve_unpaired_windows_surrogates() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let native_path = PathBuf::from(OsString::from_wide(&[0xd800]));
        let plan = CopyPlan {
            operation_id: "operation".into(),
            source: native_path.clone(),
            destination: PathBuf::from("target"),
            entries: vec![PlanEntry {
                relative_path: native_path.clone(),
                source: native_path.clone(),
                destination: native_path,
                kind: EntryKind::File,
                disposition: Disposition::Copy,
                length: 0,
                source_digest: None,
                reason: "test".into(),
                blocking: false,
            }],
        };

        let mut json = Vec::new();
        write_plan(&mut json, OutputMode::Json, &plan).unwrap();
        assert_eq!(
            String::from_utf8(json).unwrap(),
            concat!(
                "{\"schema_version\":1,\"event\":\"plan_entry\",\"data\":{\"relative_path\":{\"encoding\":\"windows_utf16le_hex\",\"value\":\"00d8\"},\"source\":{\"encoding\":\"windows_utf16le_hex\",\"value\":\"00d8\"},\"destination\":{\"encoding\":\"windows_utf16le_hex\",\"value\":\"00d8\"},\"kind\":\"file\",\"disposition\":\"copy\",\"length\":0,\"source_digest\":null,\"reason\":\"test\",\"blocking\":false}}\n",
                "{\"schema_version\":1,\"event\":\"plan_result\",\"data\":{\"operation_id\":\"operation\",\"executable\":true,\"entries\":1,\"blocking\":0}}\n"
            )
        );

        let mut human = Vec::new();
        write_plan(&mut human, OutputMode::Human, &plan).unwrap();
        assert_eq!(
            String::from_utf8(human).unwrap(),
            "Source: [windows_utf16le_hex:00d8]\nDestination: target\nCOPY [windows_utf16le_hex:00d8] — test\nPlan ready: 1 entries\n"
        );
    }
}
