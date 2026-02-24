use std::path::{Path, PathBuf};
use std::process::Command;

const GITHUB_API_URL: &str = "https://api.github.com/repos/yhzion/didyoumean/releases/latest";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

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

// --- Commands ---

/// Called from cmd_suggest: show notification if update available, spawn background check if stale.
pub fn maybe_notify_update() {
    let Some(cache_path) = cache_version_path() else {
        return;
    };

    // Check cached version
    if let Some(latest) = read_cached_version(&cache_path) {
        let current = env!("CARGO_PKG_VERSION");
        let latest_clean = latest.strip_prefix('v').unwrap_or(&latest);
        if is_newer_version(current, latest_clean) {
            eprintln!(
                "[dym] v{} available (current: v{}). Run 'didyoumean update'",
                latest_clean, current
            );
        }
    }

    // Spawn background check if stale
    if should_check_update(&cache_path) {
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

    let json = match fetch_url(GITHUB_API_URL) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Reinstall: curl -sSfL https://raw.githubusercontent.com/yhzion/didyoumean/main/install.sh | bash");
            return 1;
        }
    };

    let latest_tag = match parse_latest_version(&json) {
        Some(v) => v,
        None => {
            eprintln!("Error: could not parse latest version");
            return 1;
        }
    };

    let latest = latest_tag.strip_prefix('v').unwrap_or(&latest_tag);

    // Update cache regardless
    write_cache(&latest_tag);

    if !is_newer_version(current, latest) {
        eprintln!("Already up to date (v{}).", current);
        return 0;
    }

    if check_only {
        eprintln!("v{} available (current: v{})", latest, current);
        eprintln!("  Run 'didyoumean update' to install");
        return 0;
    }

    let target = match detect_target() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let base_url = format!(
        "https://github.com/yhzion/didyoumean/releases/download/{}",
        latest_tag
    );
    let filename = format!("didyoumean-{}.tar.gz", target);
    let tarball_url = format!("{}/{}", base_url, filename);
    let checksum_url = format!("{}/SHA256SUMS", base_url);

    eprintln!("Updating didyoumean v{} → v{}...", current, latest);

    // Create temp dir
    let tmpdir = std::env::temp_dir().join(format!("dym_update_{}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&tmpdir) {
        eprintln!("Error creating temp directory: {}", e);
        return 1;
    }
    let _cleanup = TempDirGuard(tmpdir.clone());

    // Download
    let tarball_path = tmpdir.join(&filename);
    let checksums_path = tmpdir.join("SHA256SUMS");

    eprintln!("  Downloading {}...", filename);
    if let Err(e) = download_file(&tarball_url, &tarball_path) {
        eprintln!("Error: {}", e);
        return 1;
    }
    if let Err(e) = download_file(&checksum_url, &checksums_path) {
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

    let expected = match parse_checksum_for_file(&checksums_content, &filename) {
        Some(c) => c,
        None => {
            eprintln!("Error: checksum not found for {}", filename);
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

    if expected != actual {
        eprintln!("Error: checksum mismatch");
        eprintln!("  Expected: {}", expected);
        eprintln!("  Actual:   {}", actual);
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
    eprintln!("Updated successfully (v{} → v{}).", current, latest);
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
        // HOME is always set in test environment
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
}
