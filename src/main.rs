mod shell;
mod suggest;

use std::env;
use std::io::{self, BufRead, IsTerminal};

use suggest::SuggestResult;

fn main() {
    let args: Vec<String> = env::args().collect();
    let exit_code = match args.get(1).map(|s| s.as_str()) {
        Some("init") => cmd_init(args.get(2).map(|s| s.as_str())),
        Some("uninstall") => {
            let skip_confirm = args.iter().any(|a| a == "--yes" || a == "-y");
            cmd_uninstall(skip_confirm)
        }
        Some("--version") => cmd_version(),
        Some("--help") | None => cmd_help(),
        Some(cmd) => cmd_suggest(cmd, &args[2..]),
    };
    std::process::exit(exit_code);
}

fn cmd_init(shell_arg: Option<&str>) -> i32 {
    match shell_arg {
        Some("zsh") => print!("{}", shell::init_zsh()),
        Some("bash") => print!("{}", shell::init_bash()),
        Some(s) => {
            eprintln!("Error: unsupported shell '{}'. Supported: zsh, bash", s);
            return 1;
        }
        None => {
            eprintln!("Error: specify shell type (zsh or bash)");
            eprintln!("Usage: didyoumean init <zsh|bash>");
            return 1;
        }
    }
    0
}

fn cmd_uninstall(skip_confirm: bool) -> i32 {
    let is_korean = env::var("LANG")
        .map(|v| v.starts_with("ko"))
        .unwrap_or(false);

    let home = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Error: HOME not set");
            return 1;
        }
    };

    let shell_name = env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|s| s.to_string()))
        .unwrap_or_default();

    let rc_file = match shell_name.as_str() {
        "zsh" => Some(format!("{}/.zshrc", home)),
        "bash" => Some(format!("{}/.bashrc", home)),
        _ => None,
    };

    let binary_path = env::current_exe()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| format!("{}/.local/bin/didyoumean", home));

    // Show what will be removed
    if is_korean {
        eprintln!("제거 대상:");
        eprintln!("  - 바이너리: {}", binary_path);
        if let Some(ref rc) = rc_file {
            eprintln!("  - 쉘 설정: {} (didyoumean 블록)", rc);
        }
    } else {
        eprintln!("Will remove:");
        eprintln!("  - Binary: {}", binary_path);
        if let Some(ref rc) = rc_file {
            eprintln!("  - Shell config: {} (didyoumean block)", rc);
        }
    }

    if !skip_confirm {
        if !io::stdin().is_terminal() {
            if is_korean {
                eprintln!("비대화형 모드에서는 --yes 플래그가 필요합니다.");
            } else {
                eprintln!("Use --yes to confirm in non-interactive mode.");
            }
            return 1;
        }
        if is_korean {
            eprint!("\n계속하시겠습니까? [y/N] ");
        } else {
            eprint!("\nProceed? [y/N] ");
        }
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            if is_korean {
                eprintln!("취소되었습니다.");
            } else {
                eprintln!("Cancelled.");
            }
            return 1;
        }
    }

    // Remove init block from RC file
    if let Some(ref rc) = rc_file {
        if let Ok(content) = std::fs::read_to_string(rc) {
            let start = "# >>> didyoumean initialize >>>";
            let end = "# <<< didyoumean initialize <<<";

            if content.contains(start) {
                let backup = format!("{}.dym.bak", rc);
                let _ = std::fs::copy(rc, &backup);

                let mut new_lines: Vec<&str> = Vec::new();
                let mut skip = false;
                for line in content.lines() {
                    if line.contains(start) {
                        skip = true;
                        continue;
                    }
                    if line.contains(end) {
                        skip = false;
                        continue;
                    }
                    if !skip {
                        new_lines.push(line);
                    }
                }

                // Trim trailing empty lines left by block removal
                while new_lines.last() == Some(&"") {
                    new_lines.pop();
                }
                let mut final_content = new_lines.join("\n");
                final_content.push('\n');

                if let Err(e) = std::fs::write(rc, final_content) {
                    eprintln!("Error writing {}: {}", rc, e);
                    return 1;
                }
                if is_korean {
                    eprintln!("  {} 에서 didyoumean 블록 제거 (백업: {})", rc, backup);
                } else {
                    eprintln!("  Removed didyoumean block from {} (backup: {})", rc, backup);
                }
            } else if is_korean {
                eprintln!("  {} 에 didyoumean 블록 없음 (건너뜀)", rc);
            } else {
                eprintln!("  No didyoumean block in {} (skipped)", rc);
            }
        }
    }

    // Remove binary
    if std::path::Path::new(&binary_path).exists() {
        if let Err(e) = std::fs::remove_file(&binary_path) {
            eprintln!("Error removing {}: {}", binary_path, e);
            return 1;
        }
        if is_korean {
            eprintln!("  {} 삭제 완료", binary_path);
        } else {
            eprintln!("  Removed {}", binary_path);
        }
    }

    eprintln!();
    if is_korean {
        eprintln!("didyoumean이 제거되었습니다. 'exec $SHELL'로 쉘을 재시작하세요.");
    } else {
        eprintln!("didyoumean uninstalled. Run 'exec $SHELL' to restart your shell.");
    }
    0
}

fn cmd_version() -> i32 {
    println!("didyoumean {}", env!("CARGO_PKG_VERSION"));
    0
}

fn cmd_help() -> i32 {
    println!(
        "\
didyoumean - Shell command typo correction tool

Usage:
  didyoumean <command>          Suggest corrections for a mistyped command
  didyoumean init <shell>       Output shell integration code (zsh, bash)
  didyoumean uninstall [-y]     Remove binary and shell config
  didyoumean --version          Show version
  didyoumean --help             Show this help

Environment variables:
  DYM_AUTO_CORRECT      Enable auto-correction (on/1/true, default: off)
  DYM_MAX_DISTANCE      Maximum edit distance (default: 2)
  DYM_MAX_SUGGESTIONS   Maximum suggestions to show (default: 5)
  NO_COLOR              Disable color output"
    );
    0
}

fn cmd_suggest(cmd: &str, extra_args: &[String]) -> i32 {
    let max_distance: usize = env::var("DYM_MAX_DISTANCE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let max_suggestions: usize = env::var("DYM_MAX_SUGGESTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let auto_correct_enabled = env::var("DYM_AUTO_CORRECT")
        .map(|v| matches!(v.to_lowercase().as_str(), "on" | "1" | "true"))
        .unwrap_or(false);
    let is_root = env::var("EUID").map(|v| v == "0").unwrap_or(false);
    let is_korean = env::var("LANG")
        .map(|v| v.starts_with("ko"))
        .unwrap_or(false);
    let use_color = color_enabled();

    let candidates = if !io::stdin().is_terminal() {
        read_candidates_from_stdin()
    } else {
        suggest::scan_path()
    };
    let result = suggest::suggest(cmd, &candidates, max_distance, max_suggestions);

    let args_suffix = if extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", extra_args.join(" "))
    };

    match result {
        SuggestResult::ConfidentCorrect(ref corrected) if !is_root => {
            eprintln!(
                "[dym] '{}{}' → '{}{}'",
                cmd,
                args_suffix,
                colorize(corrected, Color::YellowBold, use_color),
                args_suffix
            );
            println!("{}", corrected);
            0
        }
        SuggestResult::AutoCorrect(ref corrected) if auto_correct_enabled && !is_root => {
            if is_korean {
                eprintln!(
                    "[dym] 자동 수정: '{}{}' → '{}{}'",
                    cmd,
                    args_suffix,
                    colorize(corrected, Color::YellowBold, use_color),
                    args_suffix
                );
            } else {
                eprintln!(
                    "[dym] Correcting '{}{}' to '{}{}'",
                    cmd,
                    args_suffix,
                    colorize(corrected, Color::YellowBold, use_color),
                    args_suffix
                );
            }
            println!("{}", corrected);
            0
        }
        SuggestResult::ConfidentCorrect(corrected) | SuggestResult::AutoCorrect(corrected) => {
            print_suggestions(cmd, std::slice::from_ref(&corrected), is_korean, use_color);
            println!("{}", corrected);
            1
        }
        SuggestResult::Suggestions(ref suggestions) => {
            print_suggestions(cmd, suggestions, is_korean, use_color);
            for s in suggestions {
                println!("{}", s);
            }
            1
        }
        SuggestResult::NoMatch => {
            if is_korean {
                eprintln!("[dym] '{}': 유사한 명령어를 찾을 수 없습니다", cmd);
            } else {
                eprintln!("[dym] Command '{}' not found, no similar commands", cmd);
            }
            2
        }
    }
}

fn print_suggestions(cmd: &str, suggestions: &[String], is_korean: bool, use_color: bool) {
    if suggestions.len() == 1 {
        if is_korean {
            eprintln!(
                "[dym] 혹시 '{}'을(를) 찾으셨나요?",
                colorize(&suggestions[0], Color::Green, use_color)
            );
        } else {
            eprintln!(
                "[dym] Did you mean '{}'?",
                colorize(&suggestions[0], Color::Green, use_color)
            );
        }
    } else {
        if is_korean {
            eprintln!("[dym] '{}' 대신 이 명령어를 찾으셨나요?", cmd);
        } else {
            eprintln!("[dym] Did you mean one of these?");
        }
        for s in suggestions {
            eprintln!("  {}", colorize(s, Color::Green, use_color));
        }
    }
}

enum Color {
    YellowBold,
    Green,
}

fn colorize(text: &str, color: Color, enabled: bool) -> String {
    if !enabled {
        return text.to_string();
    }
    match color {
        Color::YellowBold => format!("\x1b[1;33m{}\x1b[0m", text),
        Color::Green => format!("\x1b[32m{}\x1b[0m", text),
    }
}

fn read_candidates_from_stdin() -> Vec<String> {
    let stdin = io::stdin();
    let mut seen = std::collections::HashSet::new();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let trimmed = line.trim().to_string();
        if !trimmed.is_empty() {
            seen.insert(trimmed);
        }
    }
    let mut result: Vec<String> = seen.into_iter().collect();
    result.sort();
    result
}

fn color_enabled() -> bool {
    if env::var("NO_COLOR").is_ok() {
        return false;
    }
    std::io::stderr().is_terminal()
}
