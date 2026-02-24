mod shell;
mod suggest;
mod update;

use std::env;
use std::io::{self, BufRead, IsTerminal};

use suggest::SuggestResult;

fn main() {
    let args: Vec<String> = env::args().collect();
    let exit_code = run(&args);
    std::process::exit(exit_code);
}

fn run(args: &[String]) -> i32 {
    match args.get(1).map(|s| s.as_str()) {
        Some("init") => cmd_init(args.get(2).map(|s| s.as_str())),
        Some("update") => {
            let check_only = args.iter().any(|a| a == "--check");
            update::update(check_only)
        }
        Some("uninstall") => {
            let skip_confirm = args.iter().any(|a| a == "--yes" || a == "-y");
            cmd_uninstall(skip_confirm)
        }
        Some("--version") => cmd_version(),
        Some("--help") | None => cmd_help(),
        Some("--check-update-bg") => update::background_check(),
        Some(cmd) => cmd_suggest(cmd, &args[2..]),
    }
}

// --- Pure logic: init script routing ---

fn get_init_script(shell: Option<&str>) -> Result<String, String> {
    match shell {
        Some("zsh") => Ok(shell::init_zsh()),
        Some("bash") => Ok(shell::init_bash()),
        Some(s) => Err(format!(
            "Error: unsupported shell '{}'. Supported: zsh, bash",
            s
        )),
        None => {
            Err("Error: specify shell type (zsh or bash)\nUsage: oops init <zsh|bash>".to_string())
        }
    }
}

fn cmd_init(shell_arg: Option<&str>) -> i32 {
    match get_init_script(shell_arg) {
        Ok(script) => {
            print!("{}", script);
            0
        }
        Err(msg) => {
            eprint!("{}", msg);
            1
        }
    }
}

// --- Pure logic: remove init block from RC content ---

fn remove_init_block(content: &str) -> String {
    let start = "# >>> oops-fix initialize >>>";
    let end = "# <<< oops-fix initialize <<<";

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

    while new_lines.last() == Some(&"") {
        new_lines.pop();
    }
    let mut final_content = new_lines.join("\n");
    final_content.push('\n');
    final_content
}

fn extract_shell_name(shell_path: &str) -> &str {
    shell_path.rsplit('/').next().unwrap_or("")
}

fn rc_file_for_shell(shell_name: &str, home: &str) -> Option<String> {
    match shell_name {
        "zsh" => Some(format!("{}/.zshrc", home)),
        "bash" => Some(format!("{}/.bashrc", home)),
        _ => None,
    }
}

struct UninstallPlan {
    binary_path: String,
    rc_file: Option<String>,
}

fn build_uninstall_plan(home: &str, shell_path: &str, exe_path: Option<String>) -> UninstallPlan {
    let shell_name = extract_shell_name(shell_path);
    let rc_file = rc_file_for_shell(shell_name, home);
    let binary_path = exe_path.unwrap_or_else(|| format!("{}/.local/bin/oops", home));
    UninstallPlan {
        binary_path,
        rc_file,
    }
}

fn confirm_action(input: &str) -> bool {
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn cmd_uninstall(skip_confirm: bool) -> i32 {
    let home = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Error: HOME not set");
            return 1;
        }
    };

    let plan = build_uninstall_plan(
        &home,
        &env::var("SHELL").unwrap_or_default(),
        env::current_exe().ok().map(|p| p.display().to_string()),
    );

    eprintln!("Will remove:");
    eprintln!("  - Binary: {}", plan.binary_path);
    if let Some(ref rc) = plan.rc_file {
        eprintln!("  - Shell config: {} (oops-fix block)", rc);
    }

    if !skip_confirm {
        if !io::stdin().is_terminal() {
            eprintln!("Use --yes to confirm in non-interactive mode.");
            return 1;
        }
        eprint!("\nProceed? [y/N] ");
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if !confirm_action(&input) {
            eprintln!("Cancelled.");
            return 1;
        }
    }

    if let Some(ref rc) = plan.rc_file {
        if let Ok(content) = std::fs::read_to_string(rc) {
            let start = "# >>> oops-fix initialize >>>";

            if content.contains(start) {
                let backup = format!("{}.oops.bak", rc);
                let _ = std::fs::copy(rc, &backup);

                let new_content = remove_init_block(&content);

                if let Err(e) = std::fs::write(rc, new_content) {
                    eprintln!("Error writing {}: {}", rc, e);
                    return 1;
                }
                eprintln!("  Removed oops-fix block from {} (backup: {})", rc, backup);
            } else {
                eprintln!("  No oops-fix block in {} (skipped)", rc);
            }
        }
    }

    if std::path::Path::new(&plan.binary_path).exists() {
        if let Err(e) = std::fs::remove_file(&plan.binary_path) {
            eprintln!("Error removing {}: {}", plan.binary_path, e);
            return 1;
        }
        eprintln!("  Removed {}", plan.binary_path);
    }

    eprintln!();
    eprintln!("oops uninstalled. Run 'exec $SHELL' to restart your shell.");
    0
}

fn cmd_version() -> i32 {
    println!("oops {}", env!("CARGO_PKG_VERSION"));
    0
}

fn help_text() -> &'static str {
    "\
oops - Typo → Fix → Run. Instantly.

USAGE
  oops <command> [args...]     Correct a mistyped command
  oops init <zsh|bash>         Output shell integration code
  oops update [--check]        Check for / install updates
  oops uninstall [-y]          Remove oops from your system
  oops --version               Show version
  oops --help                  Show this help

HOW IT WORKS
  When you mistype a command, the shell's command_not_found hook sends
  it to oops along with all known commands (builtins + PATH).
  Damerau-Levenshtein distance is computed for each candidate.

  Exit 0 - Auto-correct & execute
    Confident match: distance 1, unique best match, command length >= 3.
    The corrected command runs immediately with your original arguments.

  Exit 1 - Suggest
    Multiple close matches, or low confidence (short command, distance > 1).
    Suggestions are displayed. Nothing is executed.

  Exit 2 - No match
    No similar command found within the maximum edit distance.

EXAMPLES
  $ gti stash pop
  [oops] 'gti stash pop' -> 'git stash pop'        # auto-executed

  $ dcoker compose up -d
  [oops] 'dcoker compose up -d' -> 'docker compose up -d'

  $ gt
  [oops] Did you mean one of these? (gt)            # too short to auto-correct
    git
    gd

  $ xyzabc123
  [oops] Command 'xyzabc123' not found              # no similar command

ENVIRONMENT
  OOPS_AUTO_CORRECT=on     Also auto-execute lower-confidence corrections
  OOPS_MAX_DISTANCE=2      Maximum edit distance (default: 2)
  OOPS_MAX_SUGGESTIONS=5   Maximum suggestions to show (default: 5)
  NO_COLOR                Disable colored output
"
}

fn cmd_help() -> i32 {
    print!("{}", help_text());
    0
}

// --- Pure logic: suggestion formatting ---

struct SuggestOutput {
    stdout_lines: Vec<String>,
    stderr_lines: Vec<String>,
    exit_code: i32,
}

struct SuggestConfig {
    auto_correct_enabled: bool,
    is_root: bool,
    use_color: bool,
}

fn format_suggest_result(
    cmd: &str,
    extra_args: &[String],
    result: SuggestResult,
    config: &SuggestConfig,
) -> SuggestOutput {
    let args_suffix = build_args_suffix(extra_args);

    match result {
        SuggestResult::ConfidentCorrect(ref corrected) if !config.is_root => SuggestOutput {
            stderr_lines: vec![format!(
                "[oops] '{}{}' \u{2192} '{}{}'",
                cmd,
                args_suffix,
                colorize(corrected, Color::YellowBold, config.use_color),
                args_suffix
            )],
            stdout_lines: vec![corrected.clone()],
            exit_code: 0,
        },
        SuggestResult::AutoCorrect(ref corrected)
            if config.auto_correct_enabled && !config.is_root =>
        {
            SuggestOutput {
                stderr_lines: vec![format!(
                    "[oops] Correcting '{}{}' to '{}{}'",
                    cmd,
                    args_suffix,
                    colorize(corrected, Color::YellowBold, config.use_color),
                    args_suffix
                )],
                stdout_lines: vec![corrected.clone()],
                exit_code: 0,
            }
        }
        SuggestResult::ConfidentCorrect(corrected) | SuggestResult::AutoCorrect(corrected) => {
            SuggestOutput {
                stderr_lines: format_suggestions(
                    cmd,
                    std::slice::from_ref(&corrected),
                    config.use_color,
                ),
                stdout_lines: vec![corrected],
                exit_code: 1,
            }
        }
        SuggestResult::Suggestions(ref suggestions) => SuggestOutput {
            stderr_lines: format_suggestions(cmd, suggestions, config.use_color),
            stdout_lines: suggestions.clone(),
            exit_code: 1,
        },
        SuggestResult::NoMatch => SuggestOutput {
            stderr_lines: vec![format!(
                "[oops] Command '{}' not found, no similar commands",
                cmd
            )],
            stdout_lines: vec![],
            exit_code: 2,
        },
    }
}

fn parse_env_usize(val: Option<String>, default: usize) -> usize {
    val.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_auto_correct(val: Option<String>) -> bool {
    val.map(|v| matches!(v.to_lowercase().as_str(), "on" | "1" | "true"))
        .unwrap_or(false)
}

fn parse_is_root(val: Option<String>) -> bool {
    val.map(|v| v == "0").unwrap_or(false)
}

fn cmd_suggest(cmd: &str, extra_args: &[String]) -> i32 {
    let max_distance = parse_env_usize(env::var("OOPS_MAX_DISTANCE").ok(), 2);
    let max_suggestions = parse_env_usize(env::var("OOPS_MAX_SUGGESTIONS").ok(), 5);
    let config = SuggestConfig {
        auto_correct_enabled: parse_auto_correct(env::var("OOPS_AUTO_CORRECT").ok()),
        is_root: parse_is_root(env::var("EUID").ok()),
        use_color: color_enabled(),
    };

    let candidates = if !io::stdin().is_terminal() {
        parse_candidates(io::stdin().lock().lines().map_while(Result::ok))
    } else {
        suggest::scan_path()
    };
    let result = suggest::suggest(cmd, &candidates, max_distance, max_suggestions);

    let output = format_suggest_result(cmd, extra_args, result, &config);

    for line in &output.stderr_lines {
        eprintln!("{}", line);
    }
    for line in &output.stdout_lines {
        println!("{}", line);
    }

    // Non-blocking update notification
    update::maybe_notify_update();

    output.exit_code
}

// --- Pure helpers ---

fn build_args_suffix(extra_args: &[String]) -> String {
    if extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", extra_args.join(" "))
    }
}

fn format_suggestions(cmd: &str, suggestions: &[String], use_color: bool) -> Vec<String> {
    if suggestions.len() == 1 {
        vec![format!(
            "[oops] Did you mean '{}'?",
            colorize(&suggestions[0], Color::Green, use_color)
        )]
    } else {
        let mut lines = vec![format!("[oops] Did you mean one of these? ({})", cmd)];
        for s in suggestions {
            lines.push(format!("  {}", colorize(s, Color::Green, use_color)));
        }
        lines
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

fn parse_candidates(lines: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    for line in lines {
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

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- get_init_script tests ---

    #[test]
    fn test_get_init_script_zsh() {
        let result = get_init_script(Some("zsh"));
        assert!(result.is_ok());
        assert!(result.unwrap().contains("command_not_found_handler"));
    }

    #[test]
    fn test_get_init_script_bash() {
        let result = get_init_script(Some("bash"));
        assert!(result.is_ok());
        assert!(result.unwrap().contains("command_not_found_handle"));
    }

    #[test]
    fn test_get_init_script_unsupported() {
        let result = get_init_script(Some("fish"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported shell 'fish'"));
    }

    #[test]
    fn test_get_init_script_none() {
        let result = get_init_script(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("specify shell type"));
    }

    // --- remove_init_block tests ---

    #[test]
    fn test_remove_init_block_basic() {
        let content = "before\n# >>> oops-fix initialize >>>\neval line\n# <<< oops-fix initialize <<<\nafter\n";
        let result = remove_init_block(content);
        assert_eq!(result, "before\nafter\n");
    }

    #[test]
    fn test_remove_init_block_no_block() {
        let content = "line1\nline2\n";
        let result = remove_init_block(content);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn test_remove_init_block_trailing_empty_lines() {
        let content =
            "before\n# >>> oops-fix initialize >>>\neval\n# <<< oops-fix initialize <<<\n\n\n";
        let result = remove_init_block(content);
        assert_eq!(result, "before\n");
    }

    #[test]
    fn test_remove_init_block_at_start() {
        let content = "# >>> oops-fix initialize >>>\neval\n# <<< oops-fix initialize <<<\nafter\n";
        let result = remove_init_block(content);
        assert_eq!(result, "after\n");
    }

    #[test]
    fn test_remove_init_block_only_block() {
        let content = "# >>> oops-fix initialize >>>\neval\n# <<< oops-fix initialize <<<\n";
        let result = remove_init_block(content);
        assert_eq!(result, "\n");
    }

    // --- colorize tests ---

    #[test]
    fn test_colorize_disabled() {
        assert_eq!(colorize("hello", Color::Green, false), "hello");
        assert_eq!(colorize("hello", Color::YellowBold, false), "hello");
    }

    #[test]
    fn test_colorize_green() {
        let result = colorize("hello", Color::Green, true);
        assert_eq!(result, "\x1b[32mhello\x1b[0m");
    }

    #[test]
    fn test_colorize_yellow_bold() {
        let result = colorize("hello", Color::YellowBold, true);
        assert_eq!(result, "\x1b[1;33mhello\x1b[0m");
    }

    // --- build_args_suffix tests ---

    #[test]
    fn test_build_args_suffix_empty() {
        assert_eq!(build_args_suffix(&[]), "");
    }

    #[test]
    fn test_build_args_suffix_single() {
        let args = vec!["--continue".to_string()];
        assert_eq!(build_args_suffix(&args), " --continue");
    }

    #[test]
    fn test_build_args_suffix_multiple() {
        let args = vec!["stash".to_string(), "pop".to_string()];
        assert_eq!(build_args_suffix(&args), " stash pop");
    }

    // --- format_suggestions tests ---

    #[test]
    fn test_format_suggestions_single() {
        let suggestions = vec!["git".to_string()];
        let lines = format_suggestions("gti", &suggestions, false);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Did you mean 'git'?"));
    }

    #[test]
    fn test_format_suggestions_multiple() {
        let suggestions = vec!["git".to_string(), "gci".to_string()];
        let lines = format_suggestions("gti", &suggestions, false);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Did you mean one of these?"));
        assert!(lines[1].contains("git"));
        assert!(lines[2].contains("gci"));
    }

    #[test]
    fn test_format_suggestions_with_color() {
        let suggestions = vec!["git".to_string()];
        let lines = format_suggestions("gti", &suggestions, true);
        assert!(lines[0].contains("\x1b[32m"));
    }

    // --- parse_candidates tests ---

    #[test]
    fn test_parse_candidates_basic() {
        let lines = vec!["git".to_string(), "cargo".to_string(), "node".to_string()];
        let result = parse_candidates(lines.into_iter());
        assert_eq!(result, vec!["cargo", "git", "node"]);
    }

    #[test]
    fn test_parse_candidates_dedup() {
        let lines = vec!["git".to_string(), "git".to_string(), "cargo".to_string()];
        let result = parse_candidates(lines.into_iter());
        assert_eq!(result, vec!["cargo", "git"]);
    }

    #[test]
    fn test_parse_candidates_trims_whitespace() {
        let lines = vec!["  git  ".to_string(), "cargo".to_string()];
        let result = parse_candidates(lines.into_iter());
        assert_eq!(result, vec!["cargo", "git"]);
    }

    #[test]
    fn test_parse_candidates_skips_empty() {
        let lines = vec!["git".to_string(), "".to_string(), "  ".to_string()];
        let result = parse_candidates(lines.into_iter());
        assert_eq!(result, vec!["git"]);
    }

    #[test]
    fn test_parse_candidates_empty_input() {
        let lines: Vec<String> = vec![];
        let result = parse_candidates(lines.into_iter());
        assert!(result.is_empty());
    }

    // --- format_suggest_result tests ---

    fn no_color_config() -> SuggestConfig {
        SuggestConfig {
            auto_correct_enabled: false,
            is_root: false,
            use_color: false,
        }
    }

    #[test]
    fn test_format_confident_correct() {
        let result = SuggestResult::ConfidentCorrect("git".to_string());
        let output = format_suggest_result("gti", &[], result, &no_color_config());
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout_lines, vec!["git"]);
        assert!(output.stderr_lines[0].contains("'gti' \u{2192} 'git'"));
    }

    #[test]
    fn test_format_confident_correct_with_args() {
        let args = vec!["stash".to_string(), "pop".to_string()];
        let result = SuggestResult::ConfidentCorrect("git".to_string());
        let output = format_suggest_result("gti", &args, result, &no_color_config());
        assert_eq!(output.exit_code, 0);
        assert!(output.stderr_lines[0].contains("'gti stash pop'"));
        assert!(output.stderr_lines[0].contains("'git stash pop'"));
    }

    #[test]
    fn test_format_confident_correct_as_root() {
        let config = SuggestConfig {
            is_root: true,
            ..no_color_config()
        };
        let result = SuggestResult::ConfidentCorrect("git".to_string());
        let output = format_suggest_result("gti", &[], result, &config);
        assert_eq!(output.exit_code, 1);
        assert!(output.stderr_lines[0].contains("Did you mean"));
    }

    #[test]
    fn test_format_auto_correct_enabled() {
        let config = SuggestConfig {
            auto_correct_enabled: true,
            ..no_color_config()
        };
        let result = SuggestResult::AutoCorrect("ls".to_string());
        let output = format_suggest_result("sl", &[], result, &config);
        assert_eq!(output.exit_code, 0);
        assert!(output.stderr_lines[0].contains("Correcting"));
    }

    #[test]
    fn test_format_auto_correct_disabled() {
        let result = SuggestResult::AutoCorrect("ls".to_string());
        let output = format_suggest_result("sl", &[], result, &no_color_config());
        assert_eq!(output.exit_code, 1);
        assert!(output.stderr_lines[0].contains("Did you mean"));
    }

    #[test]
    fn test_format_auto_correct_as_root() {
        let config = SuggestConfig {
            auto_correct_enabled: true,
            is_root: true,
            use_color: false,
        };
        let result = SuggestResult::AutoCorrect("ls".to_string());
        let output = format_suggest_result("sl", &[], result, &config);
        assert_eq!(output.exit_code, 1);
    }

    #[test]
    fn test_format_suggestions_result() {
        let result = SuggestResult::Suggestions(vec!["git".to_string(), "gci".to_string()]);
        let output = format_suggest_result("gti", &[], result, &no_color_config());
        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout_lines, vec!["git", "gci"]);
        assert!(output.stderr_lines[0].contains("Did you mean one of these?"));
    }

    #[test]
    fn test_format_no_match() {
        let result = SuggestResult::NoMatch;
        let output = format_suggest_result("xyzabc", &[], result, &no_color_config());
        assert_eq!(output.exit_code, 2);
        assert!(output.stdout_lines.is_empty());
        assert!(output.stderr_lines[0].contains("not found"));
    }

    // --- help_text test ---

    #[test]
    fn test_help_text_contains_sections() {
        let text = help_text();
        assert!(text.contains("USAGE"));
        assert!(text.contains("HOW IT WORKS"));
        assert!(text.contains("EXAMPLES"));
        assert!(text.contains("ENVIRONMENT"));
        assert!(text.contains("OOPS_AUTO_CORRECT"));
        assert!(text.contains("OOPS_MAX_DISTANCE"));
        assert!(text.contains("NO_COLOR"));
    }

    // --- color_enabled test (NO_COLOR env) ---

    #[test]
    fn test_color_disabled_by_no_color_env() {
        env::set_var("NO_COLOR", "1");
        assert!(!color_enabled());
        env::remove_var("NO_COLOR");
    }

    // --- run (routing) tests ---

    #[test]
    fn test_run_help_no_args() {
        let args = vec!["oops".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn test_run_help_flag() {
        let args = vec!["oops".to_string(), "--help".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn test_run_version() {
        let args = vec!["oops".to_string(), "--version".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn test_run_init_no_shell() {
        let args = vec!["oops".to_string(), "init".to_string()];
        assert_eq!(run(&args), 1);
    }

    #[test]
    fn test_run_init_zsh() {
        let args = vec!["oops".to_string(), "init".to_string(), "zsh".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn test_run_init_bash() {
        let args = vec!["oops".to_string(), "init".to_string(), "bash".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn test_run_init_unsupported() {
        let args = vec!["oops".to_string(), "init".to_string(), "fish".to_string()];
        assert_eq!(run(&args), 1);
    }

    // --- extract_shell_name tests ---

    #[test]
    fn test_extract_shell_name_zsh() {
        assert_eq!(extract_shell_name("/bin/zsh"), "zsh");
    }

    #[test]
    fn test_extract_shell_name_bash() {
        assert_eq!(extract_shell_name("/usr/bin/bash"), "bash");
    }

    #[test]
    fn test_extract_shell_name_no_slash() {
        assert_eq!(extract_shell_name("zsh"), "zsh");
    }

    #[test]
    fn test_extract_shell_name_empty() {
        assert_eq!(extract_shell_name(""), "");
    }

    // --- rc_file_for_shell tests ---

    #[test]
    fn test_rc_file_zsh() {
        assert_eq!(
            rc_file_for_shell("zsh", "/home/user"),
            Some("/home/user/.zshrc".to_string())
        );
    }

    #[test]
    fn test_rc_file_bash() {
        assert_eq!(
            rc_file_for_shell("bash", "/home/user"),
            Some("/home/user/.bashrc".to_string())
        );
    }

    #[test]
    fn test_rc_file_unknown() {
        assert_eq!(rc_file_for_shell("fish", "/home/user"), None);
    }

    #[test]
    fn test_rc_file_empty() {
        assert_eq!(rc_file_for_shell("", "/home/user"), None);
    }

    // --- parse_env_usize tests ---

    #[test]
    fn test_parse_env_usize_valid() {
        assert_eq!(parse_env_usize(Some("3".to_string()), 2), 3);
    }

    #[test]
    fn test_parse_env_usize_invalid() {
        assert_eq!(parse_env_usize(Some("abc".to_string()), 2), 2);
    }

    #[test]
    fn test_parse_env_usize_none() {
        assert_eq!(parse_env_usize(None, 5), 5);
    }

    #[test]
    fn test_parse_env_usize_empty() {
        assert_eq!(parse_env_usize(Some("".to_string()), 2), 2);
    }

    // --- parse_auto_correct tests ---

    #[test]
    fn test_parse_auto_correct_on() {
        assert!(parse_auto_correct(Some("on".to_string())));
    }

    #[test]
    fn test_parse_auto_correct_one() {
        assert!(parse_auto_correct(Some("1".to_string())));
    }

    #[test]
    fn test_parse_auto_correct_true() {
        assert!(parse_auto_correct(Some("true".to_string())));
    }

    #[test]
    fn test_parse_auto_correct_true_uppercase() {
        assert!(parse_auto_correct(Some("TRUE".to_string())));
    }

    #[test]
    fn test_parse_auto_correct_off() {
        assert!(!parse_auto_correct(Some("off".to_string())));
    }

    #[test]
    fn test_parse_auto_correct_none() {
        assert!(!parse_auto_correct(None));
    }

    // --- parse_is_root tests ---

    #[test]
    fn test_parse_is_root_zero() {
        assert!(parse_is_root(Some("0".to_string())));
    }

    #[test]
    fn test_parse_is_root_nonzero() {
        assert!(!parse_is_root(Some("1000".to_string())));
    }

    #[test]
    fn test_parse_is_root_none() {
        assert!(!parse_is_root(None));
    }

    // --- build_uninstall_plan tests ---

    #[test]
    fn test_build_uninstall_plan_zsh() {
        let plan = build_uninstall_plan("/home/user", "/bin/zsh", None);
        assert_eq!(plan.binary_path, "/home/user/.local/bin/oops");
        assert_eq!(plan.rc_file, Some("/home/user/.zshrc".to_string()));
    }

    #[test]
    fn test_build_uninstall_plan_bash() {
        let plan = build_uninstall_plan("/home/user", "/usr/bin/bash", None);
        assert_eq!(plan.rc_file, Some("/home/user/.bashrc".to_string()));
    }

    #[test]
    fn test_build_uninstall_plan_unknown_shell() {
        let plan = build_uninstall_plan("/home/user", "/bin/fish", None);
        assert_eq!(plan.rc_file, None);
    }

    #[test]
    fn test_build_uninstall_plan_with_exe_path() {
        let plan = build_uninstall_plan(
            "/home/user",
            "/bin/zsh",
            Some("/usr/local/bin/oops".to_string()),
        );
        assert_eq!(plan.binary_path, "/usr/local/bin/oops");
    }

    #[test]
    fn test_build_uninstall_plan_empty_shell() {
        let plan = build_uninstall_plan("/home/user", "", None);
        assert_eq!(plan.rc_file, None);
    }

    // --- confirm_action tests ---

    #[test]
    fn test_confirm_action_yes() {
        assert!(confirm_action("y"));
        assert!(confirm_action("Y"));
        assert!(confirm_action("yes"));
        assert!(confirm_action("YES"));
        assert!(confirm_action("  y  "));
    }

    #[test]
    fn test_confirm_action_no() {
        assert!(!confirm_action("n"));
        assert!(!confirm_action("no"));
        assert!(!confirm_action(""));
        assert!(!confirm_action("yep"));
        assert!(!confirm_action("nope"));
    }

    // --- run routing: update ---

    #[test]
    fn test_help_text_contains_update() {
        let text = help_text();
        assert!(text.contains("update"));
    }

    // --- format_suggest_result with color ---

    #[test]
    fn test_format_confident_correct_with_color() {
        let config = SuggestConfig {
            auto_correct_enabled: false,
            is_root: false,
            use_color: true,
        };
        let result = SuggestResult::ConfidentCorrect("git".to_string());
        let output = format_suggest_result("gti", &[], result, &config);
        assert_eq!(output.exit_code, 0);
        assert!(output.stderr_lines[0].contains("\x1b[1;33m"));
    }

    #[test]
    fn test_format_auto_correct_with_args() {
        let config = SuggestConfig {
            auto_correct_enabled: true,
            is_root: false,
            use_color: false,
        };
        let args = vec!["--version".to_string()];
        let result = SuggestResult::AutoCorrect("ls".to_string());
        let output = format_suggest_result("sl", &args, result, &config);
        assert_eq!(output.exit_code, 0);
        assert!(output.stderr_lines[0].contains("'sl --version'"));
        assert!(output.stderr_lines[0].contains("'ls --version'"));
    }
}
