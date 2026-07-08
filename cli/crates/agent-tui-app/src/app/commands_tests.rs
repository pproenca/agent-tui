use super::*;
use clap::ColorChoice;
use clap::CommandFactory;
use clap::Parser;
use clap::error::ErrorKind;

#[test]
fn test_cli_defaults() {
    // SAFETY: Test-only cleanup of NO_COLOR to verify default parsing.
    unsafe {
        std::env::remove_var("NO_COLOR");
    }
    let cli = Cli::parse_from(["agent-tui", "sessions"]);
    assert!(cli.session.is_none());
    assert_eq!(cli.format, OutputFormat::Text);
    assert!(!cli.no_color);
    assert!(!cli.no_input);
}

#[test]
fn test_global_args() {
    let cli = Cli::parse_from([
        "agent-tui",
        "--session",
        "my-session",
        "--format",
        "json",
        "--no-color",
        "--no-input",
        "sessions",
    ]);
    assert_eq!(cli.session, Some("my-session".to_string()));
    assert_eq!(cli.format, OutputFormat::Json);
    assert!(cli.no_color);
    assert!(cli.no_input);
}

#[test]
fn test_run_requires_command() {
    let err = Cli::try_parse_from(["agent-tui", "run"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn test_run_defaults() {
    let cli = Cli::parse_from(["agent-tui", "run", "bash"]);
    let Commands::Run {
        command,
        args,
        cwd,
        env,
        cols,
        rows,
    } = cli.command
    else {
        panic!("Expected Run command, got {:?}", cli.command);
    };
    assert_eq!(command, "bash");
    assert!(args.is_empty());
    assert!(cwd.is_none());
    assert!(env.is_empty());

    assert_eq!(cols, 120, "Default cols should be 120");
    assert_eq!(rows, 40, "Default rows should be 40");
}

#[test]
fn test_run_custom_dimensions() {
    let cli = Cli::parse_from(["agent-tui", "run", "--cols", "80", "--rows", "24", "vim"]);
    let Commands::Run {
        cols,
        rows,
        command,
        ..
    } = cli.command
    else {
        panic!("Expected Run command, got {:?}", cli.command);
    };
    assert_eq!(cols, 80);
    assert_eq!(rows, 24);
    assert_eq!(command, "vim");
}

#[test]
fn test_run_rejects_out_of_range_cols() {
    let err = Cli::try_parse_from(["agent-tui", "run", "--cols", "9", "vim"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::ValueValidation);
}

#[test]
fn test_resize_rejects_out_of_range_rows() {
    let err = Cli::try_parse_from(["agent-tui", "resize", "--cols", "80", "--rows", "1"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::ValueValidation);
}

#[test]
fn test_run_with_args() {
    let cli = Cli::parse_from(["agent-tui", "run", "vim", "--", "file.txt", "-n"]);
    let Commands::Run { command, args, .. } = cli.command else {
        panic!("Expected Run command, got {:?}", cli.command);
    };
    assert_eq!(command, "vim");
    assert_eq!(args, vec!["file.txt".to_string(), "-n".to_string()]);
}

#[test]
fn test_run_with_env() {
    let cli = Cli::parse_from([
        "agent-tui",
        "run",
        "--env",
        "FOO=bar",
        "--env",
        "EMPTY=",
        "bash",
    ]);
    let Commands::Run { env, .. } = cli.command else {
        panic!("Expected Run command, got {:?}", cli.command);
    };
    assert_eq!(
        env,
        vec![
            EnvAssignment {
                key: "FOO".to_string(),
                value: "bar".to_string(),
            },
            EnvAssignment {
                key: "EMPTY".to_string(),
                value: String::new(),
            },
        ]
    );
}

#[test]
fn test_run_rejects_invalid_env_assignment() {
    let err = Cli::try_parse_from(["agent-tui", "run", "--env", "MISSING_EQUALS", "bash"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::ValueValidation);
}

#[test]
fn test_screenshot_flags() {
    let cli = Cli::parse_from([
        "agent-tui",
        "screenshot",
        "--region",
        "modal",
        "--retain-ansi",
        "--include-cursor",
    ]);
    let Commands::Screenshot {
        region,
        strip_ansi,
        retain_ansi,
        include_cursor,
        legacy_element,
        legacy_accessibility,
        legacy_interactive_only,
    } = cli.command
    else {
        panic!("Expected Screenshot command, got {:?}", cli.command);
    };
    assert_eq!(region, Some("modal".to_string()));
    assert!(!strip_ansi);
    assert!(retain_ansi);
    assert!(include_cursor);
    assert!(!legacy_element);
    assert!(!legacy_accessibility);
    assert!(!legacy_interactive_only);
}

#[test]
fn test_screenshot_legacy_flags() {
    let cli = Cli::parse_from(["agent-tui", "screenshot", "-e", "-a", "--interactive-only"]);
    let Commands::Screenshot {
        legacy_element,
        legacy_accessibility,
        legacy_interactive_only,
        ..
    } = cli.command
    else {
        panic!("Expected Screenshot command, got {:?}", cli.command);
    };
    assert!(legacy_element);
    assert!(legacy_accessibility);
    assert!(legacy_interactive_only);
}

#[test]
fn test_screenshot_output_mode_flags_conflict() {
    let err = Cli::try_parse_from(["agent-tui", "screenshot", "--strip-ansi", "--retain-ansi"])
        .err()
        .expect("expected parse error");

    assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
}

#[test]
fn test_legacy_action_command() {
    let cli = Cli::parse_from(["agent-tui", "action", "@submit", "click"]);
    let Commands::Action { form } = cli.command else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    assert_eq!(form, vec!["@submit".to_string(), "click".to_string()]);
}

#[test]
fn test_wait_requires_condition() {
    let err = Cli::try_parse_from(["agent-tui", "wait"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn test_wait_defaults() {
    let cli = Cli::parse_from(["agent-tui", "wait", "Loading"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert_eq!(params.text, Some("Loading".to_string()));

    assert_eq!(params.timeout, 30000, "Default timeout should be 30000ms");
}

#[test]
fn test_wait_custom_timeout() {
    let cli = Cli::parse_from(["agent-tui", "wait", "-t", "5000", "Done"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert_eq!(params.text, Some("Done".to_string()));
    assert_eq!(params.timeout, 5000);
}

#[test]
fn test_wait_allows_hyphen_text() {
    let cli = Cli::parse_from(["agent-tui", "wait", "-flaglike"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert_eq!(params.text, Some("-flaglike".to_string()));
}

#[test]
fn test_wait_stable() {
    let cli = Cli::parse_from(["agent-tui", "wait", "--stable"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert!(params.stable);
    assert!(params.text.is_none());
}

#[test]
fn test_wait_text_gone() {
    let cli = Cli::parse_from(["agent-tui", "wait", "Loading...", "--gone"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert_eq!(params.text, Some("Loading...".to_string()));
    assert!(params.gone);
}

#[test]
fn test_wait_assert_flag() {
    let cli = Cli::parse_from(["agent-tui", "wait", "--assert", "Success"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert!(params.assert);
    assert_eq!(params.text, Some("Success".to_string()));
}

#[test]
fn test_missing_required_args() {
    assert!(Cli::try_parse_from(["agent-tui", "run"]).is_err());
}

#[test]
fn test_output_format_values() {
    let cli = Cli::parse_from(["agent-tui", "-f", "text", "sessions"]);
    assert_eq!(cli.format, OutputFormat::Text);

    let cli = Cli::parse_from(["agent-tui", "-f", "json", "sessions"]);
    assert_eq!(cli.format, OutputFormat::Json);

    assert!(Cli::try_parse_from(["agent-tui", "-f", "xml", "sessions"]).is_err());
}

#[test]
fn test_json_shorthand_flag() {
    let cli = Cli::parse_from(["agent-tui", "--json", "sessions"]);
    assert!(cli.json);
}

#[test]
fn test_no_input_flag() {
    let cli = Cli::parse_from(["agent-tui", "--no-input", "sessions"]);
    assert!(cli.no_input);
}

#[test]
fn test_run_with_cwd() {
    let cli = Cli::parse_from(["agent-tui", "run", "-d", "/tmp", "bash"]);
    let Commands::Run { command, cwd, .. } = cli.command else {
        panic!("Expected Run command, got {:?}", cli.command);
    };
    assert_eq!(command, "bash");
    assert_eq!(cwd, Some(PathBuf::from("/tmp")));
}

#[test]
fn test_sessions_list() {
    let cli = Cli::parse_from(["agent-tui", "sessions"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(command.is_none());
}

#[test]
fn test_sessions_all_flag_rejected() {
    let err = Cli::try_parse_from(["agent-tui", "sessions", "--all"])
        .err()
        .expect("expected parse error");
    assert!(matches!(
        err.kind(),
        ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
    ));
}

#[test]
fn test_sessions_list_explicit() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "list"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(command, Some(SessionsCommand::List)));
}

#[test]
fn test_sessions_list_alias_ls() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "ls"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(command, Some(SessionsCommand::List)));
}

#[test]
fn test_sessions_attach_default() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "attach"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(
        command,
        Some(SessionsCommand::Attach {
            no_tty: false,
            detach_keys: None
        })
    ));
}

#[test]
fn test_sessions_attach_with_id_rejected() {
    let err = Cli::try_parse_from(["agent-tui", "sessions", "attach", "my-session"])
        .err()
        .expect("expected parse error");
    assert!(matches!(
        err.kind(),
        ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
    ));
}

#[test]
fn test_sessions_attach_no_tty() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "attach", "-T"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(
        command,
        Some(SessionsCommand::Attach {
            no_tty: true,
            detach_keys: None
        })
    ));
}

#[test]
fn test_sessions_record_subcommand_rejected() {
    let err = Cli::try_parse_from(["agent-tui", "sessions", "record"])
        .err()
        .expect("expected parse error");
    assert!(matches!(
        err.kind(),
        ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
    ));
}

#[test]
fn test_sessions_cleanup() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "cleanup"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(
        command,
        Some(SessionsCommand::Cleanup {
            all: false,
            dry_run: false,
            yes: false
        })
    ));
}

#[test]
fn test_sessions_cleanup_all() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "cleanup", "--all"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(
        command,
        Some(SessionsCommand::Cleanup {
            all: true,
            dry_run: false,
            yes: false
        })
    ));
}

#[test]
fn test_sessions_cleanup_dry_run_yes() {
    let cli = Cli::parse_from([
        "agent-tui",
        "sessions",
        "cleanup",
        "--all",
        "--dry-run",
        "--yes",
    ]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(
        command,
        Some(SessionsCommand::Cleanup {
            all: true,
            dry_run: true,
            yes: true
        })
    ));
}

#[test]
fn test_sessions_show_with_id() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "show", "abc123"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(
        command,
        Some(SessionsCommand::Show { session_id: _ })
    ));
}

#[test]
fn test_sessions_switch_with_id() {
    let cli = Cli::parse_from(["agent-tui", "sessions", "switch", "abc123"]);
    let Commands::Sessions { command } = cli.command else {
        panic!("Expected Sessions command, got {:?}", cli.command);
    };
    assert!(matches!(
        command,
        Some(SessionsCommand::Switch { session_id: _ })
    ));
}

#[test]
fn test_version_command() {
    let cli = Cli::parse_from(["agent-tui", "version"]);
    assert!(matches!(cli.command, Commands::Version));
}

#[test]
fn test_env_command() {
    let cli = Cli::parse_from(["agent-tui", "env"]);
    assert!(matches!(cli.command, Commands::Env));
}

#[test]
fn test_kill_command() {
    let cli = Cli::parse_from(["agent-tui", "kill"]);
    assert!(matches!(
        cli.command,
        Commands::Kill {
            dry_run: false,
            yes: false
        }
    ));
}

#[test]
fn test_kill_dry_run_yes() {
    let cli = Cli::parse_from(["agent-tui", "kill", "--dry-run", "--yes"]);
    assert!(matches!(
        cli.command,
        Commands::Kill {
            dry_run: true,
            yes: true
        }
    ));
}

#[test]
fn test_completions_command() {
    let cli = Cli::parse_from(["agent-tui", "completions", "bash"]);
    let Commands::Completions { shell, .. } = cli.command else {
        panic!("Expected Completions command, got {:?}", cli.command);
    };
    assert!(matches!(shell, Some(CompletionShell::Bash)));
}

#[test]
fn test_completions_fish() {
    let cli = Cli::parse_from(["agent-tui", "completions", "fish"]);
    let Commands::Completions { shell, .. } = cli.command else {
        panic!("Expected Completions command, got {:?}", cli.command);
    };
    assert!(matches!(shell, Some(CompletionShell::Fish)));
}

#[test]
fn test_completions_default_guided() {
    let cli = Cli::parse_from(["agent-tui", "completions"]);
    let Commands::Completions {
        shell,
        print,
        install,
        yes,
        ..
    } = cli.command
    else {
        panic!("Expected Completions command, got {:?}", cli.command);
    };
    assert!(shell.is_none());
    assert!(!print);
    assert!(!install);
    assert!(!yes);
}

#[test]
fn test_live_info_alias() {
    let cli = Cli::parse_from(["agent-tui", "live", "info"]);
    let Commands::Live { command } = cli.command else {
        panic!("Expected Live command, got {:?}", cli.command);
    };
    assert!(matches!(command, Some(LiveCommand::Start(_))));
}

#[test]
fn test_daemon_start_default() {
    let cli = Cli::parse_from(["agent-tui", "daemon", "start"]);
    let Commands::Daemon(DaemonCommand::Start {}) = cli.command else {
        panic!("Expected Daemon Start command, got {:?}", cli.command);
    };
}

#[test]
fn test_daemon_run() {
    let cli = Cli::parse_from(["agent-tui", "daemon", "run"]);
    assert!(matches!(cli.command, Commands::Daemon(DaemonCommand::Run)));
}

#[test]
fn test_daemon_stop_default() {
    let cli = Cli::parse_from(["agent-tui", "daemon", "stop"]);
    let Commands::Daemon(DaemonCommand::Stop {
        force,
        dry_run,
        yes,
    }) = cli.command
    else {
        panic!("Expected Daemon Stop command, got {:?}", cli.command);
    };
    assert!(!force, "Default should be graceful stop");
    assert!(!dry_run);
    assert!(!yes);
}

#[test]
fn test_daemon_stop_force() {
    let cli = Cli::parse_from(["agent-tui", "daemon", "stop", "--force"]);
    let Commands::Daemon(DaemonCommand::Stop { force, .. }) = cli.command else {
        panic!("Expected Daemon Stop command, got {:?}", cli.command);
    };
    assert!(force, "Should be force stop");
}

#[test]
fn test_daemon_stop_dry_run_yes() {
    let cli = Cli::parse_from(["agent-tui", "daemon", "stop", "--dry-run", "--yes"]);
    let Commands::Daemon(DaemonCommand::Stop { dry_run, yes, .. }) = cli.command else {
        panic!("Expected Daemon Stop command, got {:?}", cli.command);
    };
    assert!(dry_run);
    assert!(yes);
}

#[test]
fn test_daemon_restart() {
    let cli = Cli::parse_from(["agent-tui", "daemon", "restart"]);
    assert!(matches!(
        cli.command,
        Commands::Daemon(DaemonCommand::Restart {
            dry_run: false,
            yes: false
        })
    ));
}

#[test]
fn test_daemon_status() {
    let cli = Cli::parse_from(["agent-tui", "daemon", "status"]);
    assert!(matches!(
        cli.command,
        Commands::Daemon(DaemonCommand::Status)
    ));
}

#[test]
fn test_restart_command_parses() {
    let cli = Cli::parse_from(["agent-tui", "restart"]);
    assert!(matches!(
        cli.command,
        Commands::Restart {
            dry_run: false,
            yes: false
        }
    ));
}

#[test]
fn test_restart_command_dry_run_yes() {
    let cli = Cli::parse_from(["agent-tui", "restart", "--dry-run", "--yes"]);
    assert!(matches!(
        cli.command,
        Commands::Restart {
            dry_run: true,
            yes: true
        }
    ));
}

#[test]
fn test_resize_command_parses() {
    let cli = Cli::parse_from(["agent-tui", "resize", "--cols", "80", "--rows", "24"]);
    let Commands::Resize { cols, rows } = cli.command else {
        panic!("Expected Resize command, got {:?}", cli.command);
    };
    assert_eq!(cols, 80);
    assert_eq!(rows, 24);
}

// Phase 1: Press and Type commands
#[test]
fn test_press_enter_command() {
    let cli = Cli::parse_from(["agent-tui", "press", "Enter"]);
    let Commands::Press {
        keys,
        hold,
        release,
    } = cli.command
    else {
        panic!("Expected Press command, got {:?}", cli.command);
    };
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0], "Enter");
    assert!(!hold);
    assert!(!release);
}

#[test]
fn test_press_key_sequence() {
    let cli = Cli::parse_from(["agent-tui", "press", "ArrowDown", "ArrowDown", "Enter"]);
    let Commands::Press {
        keys,
        hold,
        release,
    } = cli.command
    else {
        panic!("Expected Press command, got {:?}", cli.command);
    };
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0], "ArrowDown");
    assert_eq!(keys[1], "ArrowDown");
    assert_eq!(keys[2], "Enter");
    assert!(!hold);
    assert!(!release);
}

#[test]
fn test_press_with_modifier() {
    let cli = Cli::parse_from(["agent-tui", "press", "Ctrl+C"]);
    let Commands::Press {
        keys,
        hold,
        release,
    } = cli.command
    else {
        panic!("Expected Press command, got {:?}", cli.command);
    };
    assert_eq!(keys[0], "Ctrl+C");
    assert!(!hold);
    assert!(!release);
}

#[test]
fn test_press_allows_hyphen_key() {
    let cli = Cli::parse_from(["agent-tui", "press", "-"]);
    let Commands::Press { keys, .. } = cli.command else {
        panic!("Expected Press command, got {:?}", cli.command);
    };
    assert_eq!(keys, vec!["-".to_string()]);
}

#[test]
fn test_press_hold_command() {
    let cli = Cli::parse_from(["agent-tui", "press", "Shift", "--hold"]);
    let Commands::Press {
        keys,
        hold,
        release,
    } = cli.command
    else {
        panic!("Expected Press command, got {:?}", cli.command);
    };
    assert_eq!(keys[0], "Shift");
    assert!(hold);
    assert!(!release);
}

#[test]
fn test_press_release_command() {
    let cli = Cli::parse_from(["agent-tui", "press", "Shift", "--release"]);
    let Commands::Press {
        keys,
        hold,
        release,
    } = cli.command
    else {
        panic!("Expected Press command, got {:?}", cli.command);
    };
    assert_eq!(keys[0], "Shift");
    assert!(!hold);
    assert!(release);
}

#[test]
fn test_press_flag_conflicts() {
    assert!(Cli::try_parse_from(["agent-tui", "press", "Shift", "--hold", "--release"]).is_err());
}

#[test]
fn test_type_command() {
    let cli = Cli::parse_from(["agent-tui", "type", "hello"]);
    let Commands::Type { text } = cli.command else {
        panic!("Expected Type command, got {:?}", cli.command);
    };
    assert_eq!(text, "hello");
}

#[test]
fn test_legacy_input_command() {
    let cli = Cli::parse_from(["agent-tui", "input", "hello"]);
    let Commands::Input { text } = cli.command else {
        panic!("Expected Input command, got {:?}", cli.command);
    };
    assert_eq!(text, "hello");
}

#[test]
fn test_type_allows_hyphen_text() {
    let cli = Cli::parse_from(["agent-tui", "type", "-n"]);
    let Commands::Type { text } = cli.command else {
        panic!("Expected Type command, got {:?}", cli.command);
    };
    assert_eq!(text, "-n");
}

#[test]
fn test_type_command_with_spaces() {
    let cli = Cli::parse_from(["agent-tui", "type", "Hello, World!"]);
    let Commands::Type { text } = cli.command else {
        panic!("Expected Type command, got {:?}", cli.command);
    };
    assert_eq!(text, "Hello, World!");
}

#[test]
fn test_scroll_command_default_amount() {
    let cli = Cli::parse_from(["agent-tui", "scroll", "down"]);
    let Commands::Scroll { direction, amount } = cli.command else {
        panic!("Expected Scroll command, got {:?}", cli.command);
    };
    assert!(matches!(direction, ScrollDirection::Down));
    assert_eq!(amount, 1);
}

#[test]
fn test_scroll_command_custom_amount() {
    let cli = Cli::parse_from(["agent-tui", "scroll", "up", "5"]);
    let Commands::Scroll { direction, amount } = cli.command else {
        panic!("Expected Scroll command, got {:?}", cli.command);
    };
    assert!(matches!(direction, ScrollDirection::Up));
    assert_eq!(amount, 5);
}

#[test]
fn test_cli_long_help_renders_without_color() {
    let mut cmd = Cli::command();
    cmd = cmd.color(ColorChoice::Never);
    let _ = cmd.render_long_help().to_string();
}

#[test]
fn test_cli_long_help_avoids_press_any_key_phrasing() {
    let mut cmd = Cli::command();
    cmd = cmd.color(ColorChoice::Never);
    let help = cmd.render_long_help().to_string().to_ascii_lowercase();

    assert!(!help.contains("press enter"));
    assert!(!help.contains("press any"));
}
