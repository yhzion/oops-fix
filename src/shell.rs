pub fn init_zsh() -> String {
    r#"# >>> didyoumean initialize >>>
command_not_found_handler() {
    if [[ -n "$__DYM_RUNNING" ]]; then
        echo "zsh: command not found: $1" >&2
        return 127
    fi
    export __DYM_RUNNING=1

    local output exit_code
    output=$(didyoumean "$1")
    exit_code=$?

    case $exit_code in
        0)
            "$output" "${@:2}"
            local ret=$?
            unset __DYM_RUNNING
            return $ret
            ;;
        1|2)
            unset __DYM_RUNNING
            return 127
            ;;
        *)
            unset __DYM_RUNNING
            echo "zsh: command not found: $1" >&2
            return 127
            ;;
    esac
}
# <<< didyoumean initialize <<<"#
        .to_string()
}

pub fn init_bash() -> String {
    r#"# >>> didyoumean initialize >>>
command_not_found_handle() {
    if [[ -n "$__DYM_RUNNING" ]]; then
        echo "bash: $1: command not found" >&2
        return 127
    fi
    export __DYM_RUNNING=1

    local output exit_code
    output=$(didyoumean "$1")
    exit_code=$?

    case $exit_code in
        0)
            "$output" "${@:2}"
            local ret=$?
            unset __DYM_RUNNING
            return $ret
            ;;
        1|2)
            unset __DYM_RUNNING
            return 127
            ;;
        *)
            unset __DYM_RUNNING
            echo "bash: $1: command not found" >&2
            return 127
            ;;
    esac
}
# <<< didyoumean initialize <<<"#
        .to_string()
}

pub fn uninstall_instructions() -> String {
    r#"To uninstall didyoumean:

1. Remove the didyoumean binary:
   rm ~/.local/bin/didyoumean

2. Remove the initialization block from your shell config file
   (.zshrc or .bashrc). Delete the lines between:
   # >>> didyoumean initialize >>>
   ...
   # <<< didyoumean initialize <<<

3. Restart your shell or run: exec $SHELL
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_zsh_contains_handler() {
        let output = init_zsh();
        assert!(output.contains("command_not_found_handler"));
    }

    #[test]
    fn test_init_bash_contains_handle() {
        let output = init_bash();
        assert!(output.contains("command_not_found_handle()"));
    }

    #[test]
    fn test_init_zsh_has_markers() {
        let output = init_zsh();
        assert!(output.contains("# >>> didyoumean initialize >>>"));
        assert!(output.contains("# <<< didyoumean initialize <<<"));
    }

    #[test]
    fn test_init_has_reentry_guard() {
        let zsh = init_zsh();
        let bash = init_bash();
        assert!(zsh.contains("__DYM_RUNNING"));
        assert!(bash.contains("__DYM_RUNNING"));
    }

    #[test]
    fn test_uninstall_outputs_instructions() {
        let output = uninstall_instructions();
        assert!(!output.is_empty());
        assert!(output.contains("didyoumean"));
    }
}
