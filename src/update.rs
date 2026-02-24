use std::path::{Path, PathBuf};
use std::process::Command;

const GITHUB_API_URL: &str = "https://api.github.com/repos/yhzion/didyoumean/releases/latest";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

// --- Pure data types ---

pub struct UpdateNotification {
    pub message: Option<String>,
    pub should_bg_check: bool,
}

pub enum UpdateDecision {
    AlreadyUpToDate(String),
    CheckOnly {
        current: String,
        latest: String,
    },
    NeedUpdate {
        current: String,
        latest: String,
        latest_tag: String,
    },
    FetchError(String),
    ParseError,
}

pub struct DownloadPlan {
    pub filename: String,
    pub tarball_url: String,
    pub checksum_url: String,
}

pub enum UpdateAction {
    Exit {
        stderr_lines: Vec<String>,
        exit_code: i32,
        cache_value: Option<String>,
    },
    DoUpdate {
        current: String,
        latest: String,
        latest_tag: String,
    },
}

// --- Pure functions ---

pub fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let v = version.strip_prefix('v').unwrap_or(version);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver(current), parse_semver(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

pub fn parse_latest_version(json: &str) -> Option<String> {
    let tag_key = "\"tag_name\"";
    let pos = json.find(tag_key)?;
    let rest = &json[pos + tag_key.len()..];
    let colon_pos = rest.find(':')?;
    let after_colon = &rest[colon_pos + 1..];
    let quote_start = after_colon.find('"')? + 1;
    let after_quote = &after_colon[quote_start..];
    let quote_end = after_quote.find('"')?;
    Some(after_quote[..quote_end].to_string())
}

pub fn detect_target() -> Result<String, String> {
    let os = if cfg!(target_os = "linux") {
        "unknown-linux-musl"
    } else if cfg!(target_os = "macos") {
        "apple-darwin"
    } else {
        return Err("unsupported OS".to_string());
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return Err("unsupported architecture".to_string());
    };

    Ok(format!("{}-{}", arch, os))
}

pub fn parse_checksum_for_file(checksums: &str, filename: &str) -> Option<String> {
    for line in checksums.lines() {
        if line.contains(filename) {
            return line.split_whitespace().next().map(|s| s.to_string());
        }
    }
    None
}

pub fn cache_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        Some(PathBuf::from(xdg).join("didyoumean"))
    } else {
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".cache/didyoumean"))
    }
}

pub fn cache_version_path() -> Option<PathBuf> {
    cache_dir().map(|d| d.join("latest-version"))
}

pub fn should_check_update(cache_path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(cache_path) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let age = std::time::SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    age.as_secs() > CHECK_INTERVAL_SECS
}

pub fn read_cached_version(cache_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cache_path).ok()?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Pure function: decide what update notification to show and whether to background-check.
pub fn build_update_notification(
    cached_version: Option<&str>,
    current_version: &str,
    cache_is_stale: bool,
) -> UpdateNotification {
    let message = cached_version.and_then(|latest| {
        let latest_clean = latest.strip_prefix('v').unwrap_or(latest);
        if is_newer_version(current_version, latest_clean) {
            Some(format!(
                "[dym] v{} available (current: v{}). Run 'didyoumean update'",
                latest_clean, current_version
            ))
        } else {
            None
        }
    });

    UpdateNotification {
        message,
        should_bg_check: cache_is_stale,
    }
}

/// Pure function: decide update action from fetched JSON result.
pub fn decide_update(
    current: &str,
    fetch_result: Result<&str, &str>,
    check_only: bool,
) -> UpdateDecision {
    let json = match fetch_result {
        Ok(j) => j,
        Err(e) => return UpdateDecision::FetchError(e.to_string()),
    };

    let latest_tag = match parse_latest_version(json) {
        Some(v) => v,
        None => return UpdateDecision::ParseError,
    };

    let latest = latest_tag
        .strip_prefix('v')
        .unwrap_or(&latest_tag)
        .to_string();

    if !is_newer_version(current, &latest) {
        return UpdateDecision::AlreadyUpToDate(current.to_string());
    }

    if check_only {
        return UpdateDecision::CheckOnly {
            current: current.to_string(),
            latest,
        };
    }

    UpdateDecision::NeedUpdate {
        current: current.to_string(),
        latest,
        latest_tag,
    }
}

/// Pure function: build download URLs from version tag and target triple.
pub fn build_download_plan(latest_tag: &str, target: &str) -> DownloadPlan {
    let base_url = format!(
        "https://github.com/yhzion/didyoumean/releases/download/{}",
        latest_tag
    );
    let filename = format!("didyoumean-{}.tar.gz", target);
    let tarball_url = format!("{}/{}", base_url, filename);
    let checksum_url = format!("{}/SHA256SUMS", base_url);

    DownloadPlan {
        filename,
        tarball_url,
        checksum_url,
    }
}

/// Pure function: verify checksum matches expected.
pub fn verify_checksum(
    checksums_content: &str,
    filename: &str,
    actual_hash: &str,
) -> Result<(), String> {
    let expected = parse_checksum_for_file(checksums_content, filename)
        .ok_or_else(|| format!("checksum not found for {}", filename))?;

    if expected != actual_hash {
        return Err(format!(
            "checksum mismatch\n  Expected: {}\n  Actual:   {}",
            expected, actual_hash
        ));
    }

    Ok(())
}

/// Pure function: convert UpdateDecision into actionable output.
/// Returns either a terminal output (messages + exit code) or a do-update signal.
pub fn format_update_decision(decision: UpdateDecision) -> UpdateAction {
    match decision {
        UpdateDecision::FetchError(e) => UpdateAction::Exit {
            stderr_lines: vec![
                format!("Error: {}", e),
                "Reinstall: curl -sSfL https://raw.githubusercontent.com/yhzion/didyoumean/main/install.sh | bash".to_string(),
            ],
            exit_code: 1,
            cache_value: None,
        },
        UpdateDecision::ParseError => UpdateAction::Exit {
            stderr_lines: vec!["Error: could not parse latest version".to_string()],
            exit_code: 1,
            cache_value: None,
        },
        UpdateDecision::AlreadyUpToDate(v) => UpdateAction::Exit {
            stderr_lines: vec![format!("Already up to date (v{}).", v)],
            exit_code: 0,
            cache_value: Some(format!("v{}", v)),
        },
        UpdateDecision::CheckOnly { current, latest } => UpdateAction::Exit {
            stderr_lines: vec![
                format!("v{} available (current: v{})", latest, current),
                "  Run 'didyoumean update' to install".to_string(),
            ],
            exit_code: 0,
            cache_value: Some(format!("v{}", latest)),
        },
        UpdateDecision::NeedUpdate {
            current,
            latest,
            latest_tag,
        } => UpdateAction::DoUpdate {
            current,
            latest,
            latest_tag,
        },
    }
}

// --- I/O functions ---

fn fetch_url(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-sSfL", url])
        .output()
        .or_else(|_| Command::new("wget").args(["-q", "-O", "-", url]).output())
        .map_err(|e| format!("curl/wget not available: {}", e))?;

    if !output.status.success() {
        return Err(format!("Failed to fetch {}", url));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Invalid response: {}", e))
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args(["-sSfL", url, "-o"])
        .arg(dest)
        .status()
        .or_else(|_| {
            Command::new("wget")
                .args(["-q", "-O"])
                .arg(dest)
                .arg(url)
                .status()
        })
        .map_err(|e| format!("curl/wget not available: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to download {}", url));
    }
    Ok(())
}

fn compute_sha256(path: &Path) -> Result<String, String> {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .or_else(|_| {
            Command::new("shasum")
                .args(["-a", "256"])
                .arg(path)
                .output()
        })
        .map_err(|e| format!("sha256sum/shasum not available: {}", e))?;

    if !output.status.success() {
        return Err("Failed to compute checksum".to_string());
    }

    String::from_utf8(output.stdout)
        .map_err(|_| "Invalid checksum output".to_string())?
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| "Empty checksum output".to_string())
}

fn write_cache(version: &str) {
    let Some(cache_path) = cache_version_path() else {
        return;
    };
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cache_path, version);
}

// --- Commands (thin I/O wrappers) ---

/// Called from cmd_suggest: show notification if update available, spawn background check if stale.
pub fn maybe_notify_update() {
    let Some(cache_path) = cache_version_path() else {
        return;
    };

    let cached = read_cached_version(&cache_path);
    let current = env!("CARGO_PKG_VERSION");
    let stale = should_check_update(&cache_path);

    let notification = build_update_notification(cached.as_deref(), current, stale);

    if let Some(msg) = notification.message {
        eprintln!("{}", msg);
    }

    if notification.should_bg_check {
        if let Ok(exe) = std::env::current_exe() {
            let _ = Command::new(exe)
                .arg("--check-update-bg")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}

/// Internal background command: fetch latest version and write to cache.
pub fn background_check() -> i32 {
    let json = match fetch_url(GITHUB_API_URL) {
        Ok(j) => j,
        Err(_) => return 1,
    };

    let Some(version) = parse_latest_version(&json) else {
        return 1;
    };

    write_cache(&version);
    0
}

/// `didyoumean update [--check]`
pub fn update(check_only: bool) -> i32 {
    let current = env!("CARGO_PKG_VERSION");

    eprintln!("Checking for updates...");

    let fetch_result = fetch_url(GITHUB_API_URL);
    let decision = decide_update(
        current,
        fetch_result.as_deref().map_err(|e| e.as_str()),
        check_only,
    );

    let action = format_update_decision(decision);

    match action {
        UpdateAction::Exit {
            stderr_lines,
            exit_code,
            cache_value,
        } => {
            if let Some(v) = cache_value {
                write_cache(&v);
            }
            for line in &stderr_lines {
                eprintln!("{}", line);
            }
            exit_code
        }
        UpdateAction::DoUpdate {
            current: c,
            latest: l,
            latest_tag: tag,
        } => {
            write_cache(&tag);
            do_update(&c, &l, &tag)
        }
    }
}

/// Performs the actual download + verify + replace sequence.
fn do_update(current: &str, latest: &str, latest_tag: &str) -> i32 {
    let target = match detect_target() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let plan = build_download_plan(latest_tag, &target);

    eprintln!("Updating didyoumean v{} \u{2192} v{}...", current, latest);

    // Create temp dir
    let tmpdir = std::env::temp_dir().join(format!("dym_update_{}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&tmpdir) {
        eprintln!("Error creating temp directory: {}", e);
        return 1;
    }
    let _cleanup = TempDirGuard(tmpdir.clone());

    // Download
    let tarball_path = tmpdir.join(&plan.filename);
    let checksums_path = tmpdir.join("SHA256SUMS");

    eprintln!("  Downloading {}...", plan.filename);
    if let Err(e) = download_file(&plan.tarball_url, &tarball_path) {
        eprintln!("Error: {}", e);
        return 1;
    }
    if let Err(e) = download_file(&plan.checksum_url, &checksums_path) {
        eprintln!("Error: {}", e);
        return 1;
    }

    // Verify checksum
    eprintln!("  Verifying checksum...");
    let checksums_content = match std::fs::read_to_string(&checksums_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading checksums: {}", e);
            return 1;
        }
    };

    let actual = match compute_sha256(&tarball_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    if let Err(e) = verify_checksum(&checksums_content, &plan.filename, &actual) {
        eprintln!("Error: {}", e);
        return 1;
    }

    // Extract
    let status = Command::new("tar")
        .args(["xzf"])
        .arg(&tarball_path)
        .arg("-C")
        .arg(&tmpdir)
        .status();

    match status {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("Error: failed to extract archive");
            return 1;
        }
    }

    let new_binary = tmpdir.join("didyoumean");
    if !new_binary.exists() {
        eprintln!("Error: binary not found in archive");
        return 1;
    }

    // Replace binary
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: could not determine binary path: {}", e);
            return 1;
        }
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&new_binary, std::fs::Permissions::from_mode(0o755));
    }

    // Try atomic rename, fallback to copy
    if std::fs::rename(&new_binary, &current_exe).is_err() {
        if let Err(e) = std::fs::copy(&new_binary, &current_exe) {
            eprintln!("Error replacing binary: {}", e);
            return 1;
        }
    }

    eprintln!("  Replaced {}", current_exe.display());
    eprintln!("Updated successfully (v{} \u{2192} v{}).", current, latest);
    0
}

/// RAII guard to clean up temp directory
struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_semver ---

    #[test]
    fn test_parse_semver_basic() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn test_parse_semver_with_v() {
        assert_eq!(parse_semver("v1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn test_parse_semver_zero() {
        assert_eq!(parse_semver("0.0.0"), Some((0, 0, 0)));
    }

    #[test]
    fn test_parse_semver_invalid_two_parts() {
        assert_eq!(parse_semver("1.2"), None);
    }

    #[test]
    fn test_parse_semver_invalid_non_numeric() {
        assert_eq!(parse_semver("1.2.abc"), None);
    }

    #[test]
    fn test_parse_semver_invalid_empty() {
        assert_eq!(parse_semver(""), None);
    }

    // --- is_newer_version ---

    #[test]
    fn test_is_newer_patch() {
        assert!(is_newer_version("0.1.0", "0.1.1"));
    }

    #[test]
    fn test_is_newer_minor() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
    }

    #[test]
    fn test_is_newer_major() {
        assert!(is_newer_version("0.1.0", "1.0.0"));
    }

    #[test]
    fn test_not_newer_same() {
        assert!(!is_newer_version("0.1.0", "0.1.0"));
    }

    #[test]
    fn test_not_newer_older() {
        assert!(!is_newer_version("0.2.0", "0.1.0"));
    }

    #[test]
    fn test_is_newer_with_v_prefix() {
        assert!(is_newer_version("v0.1.0", "v0.2.0"));
    }

    #[test]
    fn test_is_newer_invalid_input() {
        assert!(!is_newer_version("abc", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "abc"));
    }

    // --- parse_latest_version ---

    #[test]
    fn test_parse_latest_version_basic() {
        let json = r#"{"tag_name": "v0.2.0", "name": "Release"}"#;
        assert_eq!(parse_latest_version(json), Some("v0.2.0".to_string()));
    }

    #[test]
    fn test_parse_latest_version_whitespace() {
        let json = r#"{ "tag_name" : "v0.3.1" }"#;
        assert_eq!(parse_latest_version(json), Some("v0.3.1".to_string()));
    }

    #[test]
    fn test_parse_latest_version_in_large_json() {
        let json = r#"{"url":"https://api.github.com/...","tag_name":"v1.0.0","target_commitish":"main","name":"v1.0.0","draft":false}"#;
        assert_eq!(parse_latest_version(json), Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_parse_latest_version_no_tag() {
        assert_eq!(parse_latest_version(r#"{"name": "test"}"#), None);
    }

    #[test]
    fn test_parse_latest_version_empty() {
        assert_eq!(parse_latest_version(""), None);
    }

    // --- detect_target ---

    #[test]
    fn test_detect_target_succeeds() {
        let target = detect_target();
        assert!(target.is_ok());
        let t = target.unwrap();
        assert!(t.contains("apple-darwin") || t.contains("unknown-linux-musl"));
        assert!(t.starts_with("x86_64") || t.starts_with("aarch64"));
    }

    // --- parse_checksum_for_file ---

    #[test]
    fn test_parse_checksum_found() {
        let checksums = "abc123def456  didyoumean-aarch64-apple-darwin.tar.gz\n789xyz  didyoumean-x86_64-unknown-linux-musl.tar.gz\n";
        assert_eq!(
            parse_checksum_for_file(checksums, "didyoumean-aarch64-apple-darwin.tar.gz"),
            Some("abc123def456".to_string())
        );
    }

    #[test]
    fn test_parse_checksum_second_entry() {
        let checksums = "abc123  file-a.tar.gz\ndef456  file-b.tar.gz\n";
        assert_eq!(
            parse_checksum_for_file(checksums, "file-b.tar.gz"),
            Some("def456".to_string())
        );
    }

    #[test]
    fn test_parse_checksum_not_found() {
        let checksums = "abc123  file-a.tar.gz\n";
        assert_eq!(parse_checksum_for_file(checksums, "file-b.tar.gz"), None);
    }

    #[test]
    fn test_parse_checksum_empty() {
        assert_eq!(parse_checksum_for_file("", "file.tar.gz"), None);
    }

    // --- cache_dir ---

    #[test]
    fn test_cache_dir_returns_some() {
        let dir = cache_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.to_str().unwrap().contains("didyoumean"));
    }

    // --- should_check_update ---

    #[test]
    fn test_should_check_nonexistent() {
        assert!(should_check_update(Path::new(
            "/nonexistent/dym_test_cache"
        )));
    }

    #[test]
    fn test_should_not_check_fresh_cache() {
        let tmp = std::env::temp_dir().join("dym_test_fresh_cache");
        std::fs::write(&tmp, "v0.2.0").unwrap();
        assert!(!should_check_update(&tmp));
        let _ = std::fs::remove_file(&tmp);
    }

    // --- read_cached_version ---

    #[test]
    fn test_read_cached_version_exists() {
        let tmp = std::env::temp_dir().join("dym_test_read_cache");
        std::fs::write(&tmp, "v0.3.0\n").unwrap();
        assert_eq!(read_cached_version(&tmp), Some("v0.3.0".to_string()));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_cached_version_empty() {
        let tmp = std::env::temp_dir().join("dym_test_read_cache_empty");
        std::fs::write(&tmp, "").unwrap();
        assert_eq!(read_cached_version(&tmp), None);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_cached_version_nonexistent() {
        assert_eq!(read_cached_version(Path::new("/nonexistent/cache")), None);
    }

    // --- cache_dir with XDG ---

    #[test]
    fn test_cache_dir_xdg() {
        let old = std::env::var("XDG_CACHE_HOME").ok();
        std::env::set_var("XDG_CACHE_HOME", "/tmp/dym_xdg_test");
        let dir = cache_dir();
        assert_eq!(dir, Some(PathBuf::from("/tmp/dym_xdg_test/didyoumean")));
        match old {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
    }

    // --- compute_sha256 ---

    #[test]
    fn test_compute_sha256_known_file() {
        let tmp = std::env::temp_dir().join("dym_test_sha256");
        std::fs::write(&tmp, "hello\n").unwrap();
        let result = compute_sha256(&tmp);
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 64); // SHA256 = 64 hex chars
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_compute_sha256_nonexistent() {
        let result = compute_sha256(Path::new("/nonexistent/dym_test_sha256"));
        assert!(result.is_err());
    }

    // --- write_cache ---

    #[test]
    fn test_write_cache_creates_file() {
        let tmp = std::env::temp_dir().join("dym_test_write_cache_dir");
        let _ = std::fs::remove_dir_all(&tmp);

        let old_xdg = std::env::var("XDG_CACHE_HOME").ok();
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        write_cache("v0.5.0");

        let cached = std::fs::read_to_string(tmp.join("didyoumean/latest-version"));
        assert!(cached.is_ok());
        assert_eq!(cached.unwrap(), "v0.5.0");

        match old_xdg {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // --- cache_version_path ---

    #[test]
    fn test_cache_version_path_returns_some() {
        let path = cache_version_path();
        assert!(path.is_some());
        assert!(path.unwrap().to_str().unwrap().contains("latest-version"));
    }

    // --- TempDirGuard ---

    #[test]
    fn test_temp_dir_guard_cleanup() {
        let tmp = std::env::temp_dir().join("dym_test_guard");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("file.txt"), "test").unwrap();
        assert!(tmp.exists());
        {
            let _guard = TempDirGuard(tmp.clone());
        } // guard dropped here
        assert!(!tmp.exists());
    }

    // === NEW: build_update_notification ===

    #[test]
    fn test_notification_newer_available() {
        let n = build_update_notification(Some("v1.0.0"), "0.2.0", false);
        assert!(n.message.is_some());
        let msg = n.message.unwrap();
        assert!(msg.contains("1.0.0"));
        assert!(msg.contains("0.2.0"));
        assert!(msg.contains("didyoumean update"));
        assert!(!n.should_bg_check);
    }

    #[test]
    fn test_notification_already_up_to_date() {
        let n = build_update_notification(Some("v0.2.0"), "0.2.0", false);
        assert!(n.message.is_none());
        assert!(!n.should_bg_check);
    }

    #[test]
    fn test_notification_older_cached() {
        let n = build_update_notification(Some("v0.1.0"), "0.2.0", false);
        assert!(n.message.is_none());
    }

    #[test]
    fn test_notification_no_cache() {
        let n = build_update_notification(None, "0.2.0", true);
        assert!(n.message.is_none());
        assert!(n.should_bg_check);
    }

    #[test]
    fn test_notification_stale_cache_triggers_bg_check() {
        let n = build_update_notification(Some("v0.2.0"), "0.2.0", true);
        assert!(n.message.is_none());
        assert!(n.should_bg_check);
    }

    #[test]
    fn test_notification_newer_with_v_prefix() {
        let n = build_update_notification(Some("0.5.0"), "0.2.0", false);
        assert!(n.message.is_some());
        assert!(n.message.unwrap().contains("0.5.0"));
    }

    // === NEW: decide_update ===

    #[test]
    fn test_decide_update_fetch_error() {
        let d = decide_update("0.2.0", Err("network error"), false);
        assert!(matches!(d, UpdateDecision::FetchError(e) if e.contains("network")));
    }

    #[test]
    fn test_decide_update_parse_error() {
        let d = decide_update("0.2.0", Ok(r#"{"no_tag": true}"#), false);
        assert!(matches!(d, UpdateDecision::ParseError));
    }

    #[test]
    fn test_decide_update_already_up_to_date() {
        let json = r#"{"tag_name": "v0.2.0"}"#;
        let d = decide_update("0.2.0", Ok(json), false);
        assert!(matches!(d, UpdateDecision::AlreadyUpToDate(v) if v == "0.2.0"));
    }

    #[test]
    fn test_decide_update_check_only() {
        let json = r#"{"tag_name": "v1.0.0"}"#;
        let d = decide_update("0.2.0", Ok(json), true);
        match d {
            UpdateDecision::CheckOnly { current, latest } => {
                assert_eq!(current, "0.2.0");
                assert_eq!(latest, "1.0.0");
            }
            other => panic!(
                "Expected CheckOnly, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn test_decide_update_need_update() {
        let json = r#"{"tag_name": "v1.0.0"}"#;
        let d = decide_update("0.2.0", Ok(json), false);
        match d {
            UpdateDecision::NeedUpdate {
                current,
                latest,
                latest_tag,
            } => {
                assert_eq!(current, "0.2.0");
                assert_eq!(latest, "1.0.0");
                assert_eq!(latest_tag, "v1.0.0");
            }
            other => panic!(
                "Expected NeedUpdate, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn test_decide_update_older_remote() {
        let json = r#"{"tag_name": "v0.1.0"}"#;
        let d = decide_update("0.2.0", Ok(json), false);
        assert!(matches!(d, UpdateDecision::AlreadyUpToDate(_)));
    }

    #[test]
    fn test_decide_update_without_v_prefix() {
        let json = r#"{"tag_name": "1.0.0"}"#;
        let d = decide_update("0.2.0", Ok(json), false);
        match d {
            UpdateDecision::NeedUpdate { latest_tag, .. } => {
                assert_eq!(latest_tag, "1.0.0");
            }
            other => panic!(
                "Expected NeedUpdate, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    // === NEW: build_download_plan ===

    #[test]
    fn test_download_plan_basic() {
        let plan = build_download_plan("v1.0.0", "aarch64-apple-darwin");
        assert_eq!(
            plan.tarball_url,
            "https://github.com/yhzion/didyoumean/releases/download/v1.0.0/didyoumean-aarch64-apple-darwin.tar.gz"
        );
        assert_eq!(
            plan.checksum_url,
            "https://github.com/yhzion/didyoumean/releases/download/v1.0.0/SHA256SUMS"
        );
        assert_eq!(plan.filename, "didyoumean-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn test_download_plan_linux() {
        let plan = build_download_plan("v0.3.0", "x86_64-unknown-linux-musl");
        assert!(plan.tarball_url.contains("x86_64-unknown-linux-musl"));
        assert!(plan.tarball_url.contains("v0.3.0"));
    }

    #[test]
    fn test_download_plan_without_v() {
        let plan = build_download_plan("1.0.0", "aarch64-apple-darwin");
        assert!(plan.tarball_url.contains("/1.0.0/"));
    }

    // === NEW: verify_checksum ===

    #[test]
    fn test_verify_checksum_match() {
        let checksums = "abc123def456  didyoumean-aarch64-apple-darwin.tar.gz\n";
        let result = verify_checksum(
            checksums,
            "didyoumean-aarch64-apple-darwin.tar.gz",
            "abc123def456",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_checksum_mismatch() {
        let checksums = "abc123  file.tar.gz\n";
        let result = verify_checksum(checksums, "file.tar.gz", "xyz789");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("mismatch"));
        assert!(err.contains("abc123"));
        assert!(err.contains("xyz789"));
    }

    #[test]
    fn test_verify_checksum_not_found() {
        let checksums = "abc123  unrelated.tar.gz\n";
        let result = verify_checksum(checksums, "didyoumean-linux.tar.gz", "abc123");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_verify_checksum_empty_checksums() {
        let result = verify_checksum("", "file.tar.gz", "abc123");
        assert!(result.is_err());
    }

    // === format_update_decision ===

    #[test]
    fn test_format_decision_fetch_error() {
        let d = UpdateDecision::FetchError("network timeout".to_string());
        let action = format_update_decision(d);
        match action {
            UpdateAction::Exit {
                stderr_lines,
                exit_code,
                cache_value,
            } => {
                assert_eq!(exit_code, 1);
                assert!(stderr_lines[0].contains("network timeout"));
                assert!(stderr_lines[1].contains("Reinstall"));
                assert!(cache_value.is_none());
            }
            UpdateAction::DoUpdate { .. } => panic!("Expected Exit"),
        }
    }

    #[test]
    fn test_format_decision_parse_error() {
        let d = UpdateDecision::ParseError;
        let action = format_update_decision(d);
        match action {
            UpdateAction::Exit {
                stderr_lines,
                exit_code,
                cache_value,
            } => {
                assert_eq!(exit_code, 1);
                assert!(stderr_lines[0].contains("could not parse"));
                assert!(cache_value.is_none());
            }
            UpdateAction::DoUpdate { .. } => panic!("Expected Exit"),
        }
    }

    #[test]
    fn test_format_decision_already_up_to_date() {
        let d = UpdateDecision::AlreadyUpToDate("0.2.0".to_string());
        let action = format_update_decision(d);
        match action {
            UpdateAction::Exit {
                stderr_lines,
                exit_code,
                cache_value,
            } => {
                assert_eq!(exit_code, 0);
                assert!(stderr_lines[0].contains("Already up to date"));
                assert!(stderr_lines[0].contains("v0.2.0"));
                assert_eq!(cache_value, Some("v0.2.0".to_string()));
            }
            UpdateAction::DoUpdate { .. } => panic!("Expected Exit"),
        }
    }

    #[test]
    fn test_format_decision_check_only() {
        let d = UpdateDecision::CheckOnly {
            current: "0.2.0".to_string(),
            latest: "1.0.0".to_string(),
        };
        let action = format_update_decision(d);
        match action {
            UpdateAction::Exit {
                stderr_lines,
                exit_code,
                cache_value,
            } => {
                assert_eq!(exit_code, 0);
                assert!(stderr_lines[0].contains("1.0.0"));
                assert!(stderr_lines[0].contains("0.2.0"));
                assert!(stderr_lines[1].contains("didyoumean update"));
                assert_eq!(cache_value, Some("v1.0.0".to_string()));
            }
            UpdateAction::DoUpdate { .. } => panic!("Expected Exit"),
        }
    }

    #[test]
    fn test_format_decision_need_update() {
        let d = UpdateDecision::NeedUpdate {
            current: "0.2.0".to_string(),
            latest: "1.0.0".to_string(),
            latest_tag: "v1.0.0".to_string(),
        };
        let action = format_update_decision(d);
        match action {
            UpdateAction::DoUpdate {
                current,
                latest,
                latest_tag,
            } => {
                assert_eq!(current, "0.2.0");
                assert_eq!(latest, "1.0.0");
                assert_eq!(latest_tag, "v1.0.0");
            }
            UpdateAction::Exit { .. } => panic!("Expected DoUpdate"),
        }
    }
}
