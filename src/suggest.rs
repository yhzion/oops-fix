use std::collections::HashSet;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use strsim::damerau_levenshtein;

#[derive(Debug, PartialEq)]
pub enum SuggestResult {
    ConfidentCorrect(String),
    AutoCorrect(String),
    Suggestions(Vec<String>),
    NoMatch,
}

pub fn suggest(
    cmd: &str,
    candidates: &[String],
    max_distance: usize,
    max_suggestions: usize,
) -> SuggestResult {
    let cmd_len = cmd.len();

    let mut matches: Vec<(String, usize)> = candidates
        .iter()
        .filter(|c| cmd_len.abs_diff(c.len()) <= max_distance)
        .filter_map(|c| {
            let dist = damerau_levenshtein(cmd, c);
            if dist > 0 && dist <= max_distance {
                Some((c.clone(), dist))
            } else {
                None
            }
        })
        .collect();

    if matches.is_empty() {
        return SuggestResult::NoMatch;
    }

    matches.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let best_distance = matches[0].1;
    let best_count = matches.iter().filter(|(_, d)| *d == best_distance).count();

    if best_distance == 1 && best_count == 1 {
        if cmd_len >= 3 {
            SuggestResult::ConfidentCorrect(matches[0].0.clone())
        } else {
            SuggestResult::AutoCorrect(matches[0].0.clone())
        }
    } else {
        let suggestions: Vec<String> = matches
            .into_iter()
            .take(max_suggestions)
            .map(|(name, _)| name)
            .collect();
        SuggestResult::Suggestions(suggestions)
    }
}

pub fn scan_path() -> Vec<String> {
    let path = env::var("PATH").unwrap_or_default();
    scan_path_from(&path)
}

fn scan_path_from(path: &str) -> Vec<String> {
    let mut seen = HashSet::new();

    for dir in path.split(':') {
        if dir.is_empty() {
            continue;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }

            #[cfg(unix)]
            {
                if let Ok(meta) = file_path.metadata() {
                    if meta.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
                seen.insert(name.to_string());
            }
        }
    }

    let mut result: Vec<String> = seen.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_excluded() {
        let candidates = vec!["git".to_string()];
        let result = suggest("git", &candidates, 2, 5);
        assert_eq!(result, SuggestResult::NoMatch);
    }

    #[test]
    fn test_distance_1_single_match_confident() {
        // len >= 3, unique distance-1 match, no close 2nd → ConfidentCorrect
        let candidates = vec!["git".to_string()];
        let result = suggest("gti", &candidates, 2, 5);
        assert_eq!(result, SuggestResult::ConfidentCorrect("git".to_string()));
    }

    #[test]
    fn test_confident_correct_long_command() {
        // "claued" → "claude": transposition, len 6, no close 2nd
        let candidates = vec!["claude".to_string(), "clang".to_string()];
        let result = suggest("claued", &candidates, 2, 5);
        assert_eq!(result, SuggestResult::ConfidentCorrect("claude".to_string()));
    }

    #[test]
    fn test_not_confident_when_short() {
        // len 2 < 3 → AutoCorrect (not ConfidentCorrect)
        let candidates = vec!["ls".to_string()];
        let result = suggest("sl", &candidates, 2, 5);
        assert_eq!(result, SuggestResult::AutoCorrect("ls".to_string()));
    }

    #[test]
    fn test_confident_with_close_second() {
        // distance-1 unique match is confident even with distance-2 nearby
        let candidates = vec!["abcd".to_string(), "abxy".to_string()];
        let result = suggest("abce", &candidates, 2, 5);
        assert_eq!(result, SuggestResult::ConfidentCorrect("abcd".to_string()));
    }

    #[test]
    fn test_distance_1_multiple_matches() {
        // "gti" vs "git" = 1 (transposition), "gti" vs "gci" = 1 (substitution)
        let candidates = vec!["git".to_string(), "gci".to_string()];
        let result = suggest("gti", &candidates, 2, 5);
        match result {
            SuggestResult::Suggestions(s) => {
                assert!(s.contains(&"git".to_string()));
                assert!(s.contains(&"gci".to_string()));
            }
            _ => panic!("Expected Suggestions, got {:?}", result),
        }
    }

    #[test]
    fn test_no_match() {
        let candidates = vec!["git".to_string(), "cargo".to_string()];
        let result = suggest("xyzabc", &candidates, 2, 5);
        assert_eq!(result, SuggestResult::NoMatch);
    }

    #[test]
    fn test_length_filter() {
        let candidates = vec!["abcdefghij".to_string()];
        let result = suggest("abc", &candidates, 2, 5);
        assert_eq!(result, SuggestResult::NoMatch);
    }

    #[test]
    fn test_max_suggestions() {
        let candidates = vec![
            "abc".to_string(),
            "abd".to_string(),
            "aec".to_string(),
            "axc".to_string(),
        ];
        let result = suggest("abc", &candidates, 2, 2);
        match result {
            SuggestResult::Suggestions(s) => assert_eq!(s.len(), 2),
            _ => panic!("Expected Suggestions, got {:?}", result),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_scan_path_returns_executables() {
        use std::fs::File;

        let dir = std::env::temp_dir().join("dym_test_executables");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        File::create(dir.join("myexec")).unwrap();
        fs::set_permissions(dir.join("myexec"), fs::Permissions::from_mode(0o755)).unwrap();

        File::create(dir.join("noexec")).unwrap();
        fs::set_permissions(dir.join("noexec"), fs::Permissions::from_mode(0o644)).unwrap();

        let result = scan_path_from(dir.to_str().unwrap());
        assert!(result.contains(&"myexec".to_string()));
        assert!(!result.contains(&"noexec".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_path_skips_nonexistent_dirs() {
        let result = scan_path_from("/nonexistent/path/dym_test_12345");
        assert!(result.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_scan_path_deduplicates() {
        use std::fs::File;

        let dir = std::env::temp_dir().join("dym_test_dedup");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        File::create(dir.join("dupcmd")).unwrap();
        fs::set_permissions(dir.join("dupcmd"), fs::Permissions::from_mode(0o755)).unwrap();

        let path = format!("{}:{}", dir.display(), dir.display());
        let result = scan_path_from(&path);
        let count = result.iter().filter(|c| *c == "dupcmd").count();
        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
