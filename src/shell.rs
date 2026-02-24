pub fn init_zsh() -> String {
    r#"# >>> oops-fix initialize >>>
command_not_found_handler() {
    if [[ -n "$__OOPS_RUNNING" ]]; then
        echo "zsh: command not found: $1" >&2
        return 127
    fi
    export __OOPS_RUNNING=1

    {
        local output exit_code
        output=$(print -l ${(ko)commands} | oops "$@")
        exit_code=$?

        case $exit_code in
            0)
                "$output" "${@:2}"
                return $?
                ;;
            1|2)
                return 127
                ;;
            *)
                echo "zsh: command not found: $1" >&2
                return 127
                ;;
        esac
    } always {
        unset __OOPS_RUNNING
    }
}
# <<< oops-fix initialize <<<"#
        .to_string()
}

pub fn init_bash() -> String {
    r#"# >>> oops-fix initialize >>>
command_not_found_handle() {
    if [[ -n "$__OOPS_RUNNING" ]]; then
        echo "bash: $1: command not found" >&2
        return 127
    fi
    export __OOPS_RUNNING=1
    trap 'unset __OOPS_RUNNING; trap - INT TERM' INT TERM

    local output exit_code
    output=$(compgen -c | sort -u | oops "$@")
    exit_code=$?

    case $exit_code in
        0)
            "$output" "${@:2}"
            local ret=$?
            unset __OOPS_RUNNING
            trap - INT TERM
            return $ret
            ;;
        1|2)
            unset __OOPS_RUNNING
            trap - INT TERM
            return 127
            ;;
        *)
            unset __OOPS_RUNNING
            trap - INT TERM
            echo "bash: $1: command not found" >&2
            return 127
            ;;
    esac
}
# <<< oops-fix initialize <<<"#
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
        assert!(output.contains("# >>> oops-fix initialize >>>"));
        assert!(output.contains("# <<< oops-fix initialize <<<"));
    }

    #[test]
    fn test_init_has_reentry_guard() {
        let zsh = init_zsh();
        let bash = init_bash();
        assert!(zsh.contains("__OOPS_RUNNING"));
        assert!(bash.contains("__OOPS_RUNNING"));
    }

    #[test]
    fn test_init_zsh_has_always_cleanup() {
        let zsh = init_zsh();
        assert!(zsh.contains("} always {"));
        assert!(zsh.contains("unset __OOPS_RUNNING"));
    }

    #[test]
    fn test_init_bash_has_signal_trap() {
        let bash = init_bash();
        assert!(bash.contains("trap 'unset __OOPS_RUNNING; trap - INT TERM' INT TERM"));
    }
}
