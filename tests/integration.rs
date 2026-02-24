use std::process::Command;

fn dym_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_didyoumean"))
}

// --- init tests ---

#[test]
fn test_init_zsh_output() {
    let output = dym_bin().args(["init", "zsh"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("command_not_found_handler"));
    assert!(stdout.contains("# >>> didyoumean initialize >>>"));
}

#[test]
fn test_init_bash_output() {
    let output = dym_bin().args(["init", "bash"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("command_not_found_handle"));
}

#[test]
fn test_init_no_shell_fails() {
    let output = dym_bin().arg("init").output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_init_unsupported_shell_fails() {
    let output = dym_bin().args(["init", "fish"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unsupported shell"));
}

// --- version / help ---

#[test]
fn test_version_output() {
    let output = dym_bin().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("didyoumean "));
}

#[test]
fn test_help_output() {
    let output = dym_bin().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("USAGE"));
    assert!(stdout.contains("EXAMPLES"));
}

#[test]
fn test_no_args_shows_help() {
    let output = dym_bin().output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("USAGE"));
}

// --- suggest via stdin pipe ---

#[test]
fn test_suggest_confident_correct_via_stdin() {
    let output = dym_bin()
        .arg("gti")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "git").ok();
                writeln!(stdin, "gdb").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "git");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("[dym]"));
}

#[test]
fn test_suggest_with_args_via_stdin() {
    let output = dym_bin()
        .args(["gti", "stash", "pop"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "git").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("stash pop"));
}

#[test]
fn test_suggest_no_match_via_stdin() {
    let output = dym_bin()
        .arg("xyzabc123")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "git").ok();
                writeln!(stdin, "cargo").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("not found"));
}

#[test]
fn test_suggest_multiple_matches_via_stdin() {
    // "gti" matches both "git" (d=1) and "gci" (d=1) → suggestions
    let output = dym_bin()
        .arg("gti")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "git").ok();
                writeln!(stdin, "gci").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Did you mean"));
}

#[test]
fn test_suggest_short_command_auto_correct_disabled() {
    // "sl" → "ls" is AutoCorrect (short), without DYM_AUTO_CORRECT → exit 1
    let output = dym_bin()
        .arg("sl")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "ls").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_suggest_auto_correct_enabled() {
    // With DYM_AUTO_CORRECT=on, short command auto-corrects → exit 0
    let output = dym_bin()
        .arg("sl")
        .env("DYM_AUTO_CORRECT", "on")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "ls").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "ls");
}

#[test]
fn test_suggest_no_color() {
    let output = dym_bin()
        .arg("gti")
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "git").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("\x1b["));
}

#[test]
fn test_suggest_custom_max_distance() {
    // With DYM_MAX_DISTANCE=1, "dcokre" (distance 2 from "docker") → no match
    let output = dym_bin()
        .arg("dcokre")
        .env("DYM_MAX_DISTANCE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "docker").ok();
            }
            child.wait_with_output()
        })
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}

// --- update tests ---

#[test]
fn test_update_check_flag_recognized() {
    // --check should be recognized (may fail due to network, but should not panic)
    let output = dym_bin().args(["update", "--check"]).output().unwrap();
    // exit 0 = up to date or update available, exit 1 = network error
    assert!(output.status.code().unwrap() <= 1);
}

#[test]
fn test_help_mentions_update() {
    let output = dym_bin().arg("--help").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("update"));
}

// --- uninstall tests ---

#[test]
fn test_uninstall_non_interactive_without_yes_fails() {
    let output = dym_bin()
        .arg("uninstall")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--yes"));
}

#[test]
fn test_uninstall_with_yes_removes_block_and_binary() {
    let tmp = std::env::temp_dir().join("dym_uninstall_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // Create a fake binary
    let binary_path = tmp.join("didyoumean");
    std::fs::write(&binary_path, "fake binary").unwrap();

    // Create a fake RC file with didyoumean block
    let rc_path = tmp.join(".zshrc");
    std::fs::write(
        &rc_path,
        "export FOO=bar\n# >>> didyoumean initialize >>>\neval \"$(didyoumean init zsh)\"\n# <<< didyoumean initialize <<<\nexport BAZ=qux\n",
    )
    .unwrap();

    // Run uninstall with --yes, overriding HOME and SHELL
    let output = Command::new(env!("CARGO_BIN_EXE_didyoumean"))
        .args(["uninstall", "--yes"])
        .env("HOME", tmp.to_str().unwrap())
        .env("SHELL", "/bin/zsh")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();

    // Verify RC file was cleaned
    let rc_content = std::fs::read_to_string(&rc_path).unwrap();
    assert!(!rc_content.contains("didyoumean"));
    assert!(rc_content.contains("export FOO=bar"));
    assert!(rc_content.contains("export BAZ=qux"));

    // Verify backup was created
    let backup_path = tmp.join(".zshrc.dym.bak");
    assert!(backup_path.exists());

    // Verify stderr mentions removal
    assert!(stderr.contains("Removed didyoumean block"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_uninstall_no_block_skips() {
    let tmp = std::env::temp_dir().join("dym_uninstall_noblock");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rc_path = tmp.join(".zshrc");
    std::fs::write(&rc_path, "export FOO=bar\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_didyoumean"))
        .args(["uninstall", "--yes"])
        .env("HOME", tmp.to_str().unwrap())
        .env("SHELL", "/bin/zsh")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("skipped"));

    // RC file should be unchanged
    let rc_content = std::fs::read_to_string(&rc_path).unwrap();
    assert_eq!(rc_content, "export FOO=bar\n");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_uninstall_with_y_flag() {
    let tmp = std::env::temp_dir().join("dym_uninstall_yflag");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_didyoumean"))
        .args(["uninstall", "-y"])
        .env("HOME", tmp.to_str().unwrap())
        .env("SHELL", "/bin/fish") // unknown shell → no rc_file
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("uninstalled"));

    let _ = std::fs::remove_dir_all(&tmp);
}
