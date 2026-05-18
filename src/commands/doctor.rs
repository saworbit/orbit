//! Doctor command — diagnose configuration and probe backend connectivity.
//!
//! Doctor is informational. It never mutates state and currently always exits
//! with code 0 regardless of probe failures; a `--strict` mode is tracked in
//! [ROADMAP-v0.7.md].

use crate::cli_style::{self, section_header, Icons, Theme};
use crate::config::CopyConfig;
use crate::get_zero_copy_capabilities;

/// Entry point for `orbit doctor [--target <uri>...]`.
pub fn run_doctor(targets: &[String]) {
    cli_style::print_banner();
    section_header(&format!("{} Orbit Doctor", Icons::WRENCH));
    println!();

    print_config_section();
    print_platform_section();
    print_hardware_section();
    print_features_section();
    print_env_section();
    print_backend_probes_section(targets);

    println!("  {}", Theme::success("Doctor check complete."));
    println!();
}

fn print_config_section() {
    let home = dirs::home_dir();
    let path = home.as_ref().map(|h| h.join(".orbit").join("orbit.toml"));
    let exists = path.as_ref().map(|p| p.exists()).unwrap_or(false);

    if let (Some(path), true) = (path.as_ref(), exists) {
        println!(
            "  {} {} {}",
            Icons::SUCCESS,
            Theme::muted("Config file:"),
            Theme::success(path.display())
        );
        match CopyConfig::from_file(path) {
            Ok(_) => println!(
                "  {} {}",
                Icons::SUCCESS,
                Theme::success("Config file is valid TOML")
            ),
            Err(e) => println!(
                "  {} {} {}",
                Icons::ERROR,
                Theme::error("Config parse error:"),
                e
            ),
        }
    } else {
        println!(
            "  {} {} {}",
            Icons::WARNING,
            Theme::warning("No config file found."),
            Theme::muted("Run 'orbit init' to create one.")
        );
    }
    println!();
}

fn print_platform_section() {
    section_header(&format!("{} Platform", Icons::GEAR));
    println!(
        "  {} {} {} / {}",
        Icons::BULLET,
        Theme::muted("OS:"),
        Theme::value(std::env::consts::OS),
        Theme::muted(std::env::consts::ARCH)
    );

    let caps = get_zero_copy_capabilities();
    println!(
        "  {} {} {}",
        if caps.available {
            Icons::SUCCESS
        } else {
            Icons::WARNING
        },
        Theme::muted("Zero-copy:"),
        if caps.available {
            Theme::success(caps.method)
        } else {
            Theme::warning("unavailable")
        }
    );
    println!();
}

fn print_hardware_section() {
    section_header(&format!("{} Hardware", Icons::LIGHTNING));
    match crate::core::probe::Probe::scan(&std::env::current_dir().unwrap_or_default()) {
        Ok(profile) => {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("CPU cores:"),
                Theme::value(profile.logical_cores)
            );
            println!(
                "  {} {} {} GB",
                Icons::BULLET,
                Theme::muted("RAM:"),
                Theme::value(profile.available_ram_gb)
            );
            println!(
                "  {} {} ~{:.0} MB/s",
                Icons::BULLET,
                Theme::muted("I/O throughput:"),
                profile.estimated_io_throughput
            );
        }
        Err(e) => {
            println!(
                "  {} {} {}",
                Icons::WARNING,
                Theme::warning("Probe failed:"),
                e
            );
        }
    }
    println!();
}

fn print_features_section() {
    section_header(&format!("{} Compiled Features", Icons::GEAR));
    let features: Vec<(&str, bool)> = vec![
        ("s3-native", cfg!(feature = "s3-native")),
        ("s3-cli", cfg!(feature = "s3-cli")),
        ("smb-native", cfg!(feature = "smb-native")),
        ("ssh-backend", cfg!(feature = "ssh-backend")),
        ("azure-native", cfg!(feature = "azure-native")),
        ("gcs-native", cfg!(feature = "gcs-native")),
        ("backend-abstraction", cfg!(feature = "backend-abstraction")),
        ("opentelemetry", cfg!(feature = "opentelemetry")),
    ];
    for (name, enabled) in &features {
        println!(
            "  {} {}",
            if *enabled {
                Icons::SUCCESS
            } else {
                Icons::BULLET
            },
            if *enabled {
                Theme::success(*name).to_string()
            } else {
                Theme::muted(*name).to_string()
            }
        );
    }
    println!();
}

fn print_env_section() {
    section_header(&format!("{} Environment", Icons::GLOBE));
    let jwt_set = std::env::var("ORBIT_JWT_SECRET").is_ok();
    println!(
        "  {} {} {}",
        if jwt_set {
            Icons::SUCCESS
        } else {
            Icons::BULLET
        },
        Theme::muted("ORBIT_JWT_SECRET:"),
        if jwt_set {
            Theme::success("set")
        } else {
            Theme::muted("not set (dashboard auth disabled)")
        }
    );

    let stats_env = std::env::var("ORBIT_STATS").unwrap_or_else(|_| "on".to_string());
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("ORBIT_STATS:"),
        Theme::value(&stats_env)
    );
    println!();
}

// ---------------------------------------------------------------------------
// Backend connectivity probes
// ---------------------------------------------------------------------------

#[cfg(not(feature = "backend-abstraction"))]
fn print_backend_probes_section(targets: &[String]) {
    section_header(&format!("{} Backend Connectivity", Icons::SATELLITE));
    if targets.is_empty() {
        println!(
            "  {} {}",
            Icons::BULLET,
            Theme::muted("No backend-abstraction feature compiled — probes unavailable.")
        );
    } else {
        println!(
            "  {} {} {}",
            Icons::WARNING,
            Theme::warning("Probe targets ignored:"),
            Theme::muted("rebuild with --features backend-abstraction (or a backend feature)")
        );
        for t in targets {
            println!("    {} {}", Icons::BULLET, Theme::muted(t));
        }
    }
    println!();
}

#[cfg(feature = "backend-abstraction")]
fn print_backend_probes_section(targets: &[String]) {
    use crate::backend::{from_env, BackendRegistry};

    let env_type = std::env::var("ORBIT_BACKEND_TYPE")
        .ok()
        .map(|s| s.to_lowercase());
    let env_probe_enabled = env_type
        .as_deref()
        .map(|t| t != "local") // skip local — nothing to probe
        .unwrap_or(false);

    if targets.is_empty() && !env_probe_enabled {
        section_header(&format!("{} Backend Connectivity", Icons::SATELLITE));
        println!(
            "  {} {}",
            Icons::BULLET,
            Theme::muted("No probe targets. Pass --target <uri> or set ORBIT_BACKEND_TYPE.")
        );
        println!();
        return;
    }

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            section_header(&format!("{} Backend Connectivity", Icons::SATELLITE));
            println!(
                "  {} {} {}",
                Icons::ERROR,
                Theme::error("Could not start runtime:"),
                e
            );
            println!();
            return;
        }
    };

    section_header(&format!("{} Backend Connectivity", Icons::SATELLITE));
    let registry = BackendRegistry::new();

    for target in targets {
        runtime.block_on(probe_uri(&registry, target));
    }

    if env_probe_enabled {
        match from_env() {
            Ok(config) => {
                let label = format!("env:{}", env_type.as_deref().unwrap_or("?"));
                runtime.block_on(probe_config(&registry, &label, &config, None));
            }
            Err(e) => {
                println!(
                    "  {} {} {}",
                    Icons::ERROR,
                    Theme::error("env config error:"),
                    e
                );
                println!(
                    "    {} {}",
                    Icons::ARROW_RIGHT,
                    Theme::muted(
                        "set the required ORBIT_* / AWS_* / AZURE_* / GOOGLE_* variables for this backend"
                    )
                );
            }
        }
    }

    println!();
}

#[cfg(feature = "backend-abstraction")]
async fn probe_uri(registry: &crate::backend::BackendRegistry, uri: &str) {
    use crate::backend::parse_uri;

    let (config, path) = match parse_uri(uri) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "  {} {} {} {}",
                Icons::ERROR,
                Theme::error(uri),
                Theme::muted("→ URI parse error:"),
                e
            );
            println!(
                "    {} {}",
                Icons::ARROW_RIGHT,
                Theme::muted("check the URI scheme and query parameters")
            );
            return;
        }
    };
    probe_config(registry, uri, &config, Some(&path)).await;
}

/// Choose what path to pass to `Backend::list` when probing.
///
/// `parse_uri` returns a `(config, path)` pair where `path` is the URI's
/// path segment. For backends that already encode that segment as a `prefix`
/// inside the config (S3, Azure, GCS, SMB), passing it again to `list` would
/// double-prefix the request — e.g. `s3://bucket/foo` would end up listing
/// `foo/foo`. Those backends should be probed at their root. Local and SSH
/// don't bake a prefix into config, so the parsed path is the real probe
/// target.
#[cfg(feature = "backend-abstraction")]
fn probe_path_for(
    config: &crate::backend::BackendConfig,
    list_path: Option<&std::path::Path>,
) -> std::path::PathBuf {
    // Import only when at least one provider arm below is enabled; otherwise
    // `BackendConfig` is unused inside this function under
    // `--features backend-abstraction` alone and `-D unused-imports` fails.
    // When adding a new backend variant, extend both the `cfg(any(...))` here
    // and add a matching arm.
    #[cfg(any(
        feature = "s3-native",
        feature = "azure-native",
        feature = "gcs-native",
        feature = "smb-native",
    ))]
    use crate::backend::BackendConfig;
    match config {
        #[cfg(feature = "s3-native")]
        BackendConfig::S3 { .. } => std::path::PathBuf::new(),
        #[cfg(feature = "azure-native")]
        BackendConfig::Azure { .. } => std::path::PathBuf::new(),
        #[cfg(feature = "gcs-native")]
        BackendConfig::Gcs { .. } => std::path::PathBuf::new(),
        #[cfg(feature = "smb-native")]
        BackendConfig::Smb(_) => std::path::PathBuf::new(),
        // Local and SSH: the parsed path is the actual probe target.
        _ => list_path
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default(),
    }
}

#[cfg(feature = "backend-abstraction")]
async fn probe_config(
    registry: &crate::backend::BackendRegistry,
    label: &str,
    config: &crate::backend::BackendConfig,
    list_path: Option<&std::path::Path>,
) {
    use crate::backend::types::ListOptions;
    use futures::StreamExt;
    use std::time::Instant;

    let started = Instant::now();
    let backend = match registry.create(config).await {
        Ok(b) => b,
        Err(e) => {
            println!(
                "  {} {} {} {}",
                Icons::ERROR,
                Theme::error(label),
                Theme::muted("→"),
                e
            );
            print_suggestion(&e);
            return;
        }
    };

    let probe_path = probe_path_for(config, list_path);
    let opts = ListOptions {
        max_entries: Some(1),
        ..Default::default()
    };

    match backend.list(&probe_path, opts).await {
        Ok(mut stream) => {
            let first = stream.next().await;
            let elapsed = started.elapsed();
            match first {
                Some(Ok(_)) | None => {
                    println!(
                        "  {} {} {} {}ms",
                        Icons::SUCCESS,
                        Theme::success(label),
                        Theme::muted("→ connected in"),
                        elapsed.as_millis()
                    );
                }
                Some(Err(e)) => {
                    println!(
                        "  {} {} {} {}",
                        Icons::ERROR,
                        Theme::error(label),
                        Theme::muted("→ list error:"),
                        e
                    );
                    print_suggestion(&e);
                }
            }
        }
        Err(e) => {
            println!(
                "  {} {} {} {}",
                Icons::ERROR,
                Theme::error(label),
                Theme::muted("→ list error:"),
                e
            );
            print_suggestion(&e);
        }
    }
}

#[cfg(feature = "backend-abstraction")]
fn print_suggestion(err: &crate::backend::BackendError) {
    if let Some(hint) = suggest_for_error(err) {
        println!("    {} {}", Icons::ARROW_RIGHT, Theme::muted(hint));
    }
}

#[cfg(feature = "backend-abstraction")]
fn suggest_for_error(err: &crate::backend::BackendError) -> Option<&'static str> {
    use crate::backend::BackendError as E;
    match err {
        E::AuthenticationFailed { backend, .. } => Some(match backend.as_str() {
            "s3" => "check AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY (or run `aws configure`)",
            "ssh" => "verify the SSH key path, or run `ssh-add <key>` for agent auth",
            "smb" => "check ORBIT_SMB_USER and ORBIT_SMB_PASSWORD",
            "azure" => {
                "set AZURE_STORAGE_CONNECTION_STRING (or AZURE_STORAGE_ACCOUNT + AZURE_STORAGE_KEY)"
            }
            "gcs" => "set GOOGLE_SERVICE_ACCOUNT_KEY (path to JSON credentials)",
            _ => "check backend credentials",
        }),
        E::ConnectionFailed { backend, .. } => Some(match backend.as_str() {
            "s3" => "check endpoint + region; confirm the bucket exists and DNS resolves",
            "ssh" => "confirm the host is reachable on the configured port",
            "smb" => "confirm the SMB share is reachable and the port is open",
            "azure" | "gcs" => "confirm network reachability to the storage endpoint",
            _ => "verify network reachability to the endpoint",
        }),
        E::NotFound { backend, .. } => Some(match backend.as_str() {
            "s3" => "bucket or prefix not found (or missing ListBucket permission)",
            "azure" => "container or prefix not found (or missing list permission)",
            "gcs" => "bucket or prefix not found (or missing storage.objects.list)",
            _ => "the path does not exist on the backend",
        }),
        E::PermissionDenied { .. } => {
            Some("backend rejected the request — check IAM / ACL / share permissions")
        }
        E::InvalidConfig { .. } => Some("review the URI / env vars for typos or missing fields"),
        E::Timeout { .. } => Some("network or backend is slow — try again, or check firewall"),
        E::Network { .. } => Some("network error — check connectivity, DNS, and proxy settings"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn local_uri_probes_succeed() {
        // Smoke test: running doctor against a local temp dir should not panic.
        let tmp = tempfile::tempdir().unwrap();
        let uri = tmp.path().to_string_lossy().to_string();
        run_doctor(&[uri]);
    }

    #[test]
    fn run_doctor_with_no_targets_does_not_panic() {
        run_doctor(&[]);
    }

    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn suggest_for_auth_failure_returns_hint() {
        use crate::backend::BackendError;
        let err = BackendError::AuthenticationFailed {
            backend: "s3".to_string(),
            message: "no creds".to_string(),
        };
        assert!(suggest_for_error(&err).is_some());
    }

    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn suggest_for_unknown_backend_falls_back() {
        use crate::backend::BackendError;
        let err = BackendError::AuthenticationFailed {
            backend: "weird".to_string(),
            message: "x".to_string(),
        };
        assert_eq!(suggest_for_error(&err), Some("check backend credentials"));
    }

    // ----- probe_path_for: keep `parse_uri` + Backend::list from double-prefixing -----

    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn probe_path_for_local_uses_parsed_path() {
        use crate::backend::parse_uri;
        use std::path::PathBuf;
        let (config, path) = parse_uri("/tmp/data").unwrap();
        assert_eq!(
            probe_path_for(&config, Some(&path)),
            PathBuf::from("/tmp/data")
        );
    }

    /// Regression: `s3://bucket/prefix` would otherwise double-prefix to `prefix/prefix`
    /// because `parse_uri` stores `prefix` in `BackendConfig::S3` AND returns it as
    /// the parsed path, and `S3Backend::path_to_key` concatenates the two.
    #[test]
    #[cfg(all(feature = "backend-abstraction", feature = "s3-native"))]
    fn probe_path_for_s3_with_prefix_uses_root() {
        use crate::backend::parse_uri;
        use std::path::PathBuf;
        let (config, path) = parse_uri("s3://my-bucket/some/prefix").unwrap();
        assert_eq!(probe_path_for(&config, Some(&path)), PathBuf::new());
    }

    #[test]
    #[cfg(all(feature = "backend-abstraction", feature = "azure-native"))]
    fn probe_path_for_azure_with_prefix_uses_root() {
        use crate::backend::parse_uri;
        use std::path::PathBuf;
        let (config, path) = parse_uri("azblob://my-container/some/prefix").unwrap();
        assert_eq!(probe_path_for(&config, Some(&path)), PathBuf::new());
    }

    #[test]
    #[cfg(all(feature = "backend-abstraction", feature = "gcs-native"))]
    fn probe_path_for_gcs_with_prefix_uses_root() {
        use crate::backend::parse_uri;
        use std::path::PathBuf;
        let (config, path) = parse_uri("gs://my-bucket/some/prefix").unwrap();
        assert_eq!(probe_path_for(&config, Some(&path)), PathBuf::new());
    }

    #[test]
    #[cfg(all(feature = "backend-abstraction", feature = "smb-native"))]
    fn probe_path_for_smb_with_subpath_uses_root() {
        use crate::backend::parse_uri;
        use std::path::PathBuf;
        let (config, path) = parse_uri("smb://server/share/some/sub").unwrap();
        assert_eq!(probe_path_for(&config, Some(&path)), PathBuf::new());
    }

    #[test]
    #[cfg(all(feature = "backend-abstraction", feature = "ssh-backend"))]
    fn probe_path_for_ssh_uses_parsed_path() {
        use crate::backend::parse_uri;
        use std::path::PathBuf;
        let (config, path) = parse_uri("ssh://user@host/remote/dir").unwrap();
        assert_eq!(
            probe_path_for(&config, Some(&path)),
            PathBuf::from("/remote/dir")
        );
    }
}
