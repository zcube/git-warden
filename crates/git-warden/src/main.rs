//! git-warden CLI entry point. Corresponds to Go `cmd/root.go` + `Execute`.
//! Uses clap builder API instead of cobra/fang, injecting i18n help text at runtime.

mod commands;
mod extra;
mod output;
mod steps;

use clap::{Arg, ArgAction, ArgMatches, Command};

/// Global flags. Corresponds to Go's globalQuiet/globalNoColor/globalNoGuide/globalRequireConfig/configFile.
pub struct Globals {
    pub config_file: String,
    pub quiet: bool,
    pub no_color: bool,
    pub no_guide: bool,
    pub require_config: bool,
}

/// Command execution error. Corresponds to Go's errSilentExit / general error.
pub enum CmdError {
    /// Already reported to stderr; exit with code 1 without additional output.
    Silent,
    /// Print message to stderr and exit with code 1.
    Msg(String),
}

fn main() {
    // Detect locale from environment variables and initialize i18n.
    git_warden_i18n::init("");

    let matches = build_cli().get_matches();
    let code = run(&matches);
    std::process::exit(code);
}

fn read_globals(m: &ArgMatches) -> Globals {
    Globals {
        config_file: m
            .get_one::<String>("config")
            .cloned()
            .unwrap_or_else(|| ".git-warden.yaml".to_string()),
        quiet: m.get_flag("quiet"),
        no_color: m.get_flag("no-color"),
        no_guide: m.get_flag("no-guide"),
        require_config: m.get_flag("require-config"),
    }
}

fn run(matches: &ArgMatches) -> i32 {
    let Some((name, sub)) = matches.subcommand() else {
        // No subcommand: print help and exit normally (cobra root behavior).
        let _ = build_cli().print_help();
        println!();
        return 0;
    };

    let g = read_globals(sub);
    // Configure logger (corresponds to PersistentPreRunE).
    git_warden_logger::set_quiet(g.quiet);
    if g.no_color {
        git_warden_logger::set_no_color(true);
    }

    let result: Result<(), CmdError> = match name {
        "run" => commands::cmd_run(&g, fmt(sub), &only(sub)),
        "diff" => {
            let args: Vec<String> = sub
                .get_many::<String>("args")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            commands::cmd_diff(&g, fmt(sub), sub.get_flag("staged"), &only(sub), &args)
        }
        "msg" => commands::cmd_msg(
            &g,
            sub.get_one::<String>("file").unwrap(),
            sub.get_flag("fix"),
        ),
        "push" => commands::cmd_push(
            &g,
            sub.get_one::<String>("range")
                .map(|s| s.as_str())
                .unwrap_or(""),
        ),
        "prepare-msg" => {
            let args: Vec<&String> = sub
                .get_many::<String>("args")
                .map(|v| v.collect())
                .unwrap_or_default();
            let file = args.first().map(|s| s.as_str()).unwrap_or("");
            let source = args.get(1).map(|s| s.as_str());
            commands::cmd_prepare_msg(&g, file, source)
        }
        "fix" => commands::cmd_fix(&g, sub.get_flag("dry-run")),
        "clean" => extra::cmd_clean(&g, sub.get_flag("yes")),
        "analyze" => extra::cmd_analyze(&g),
        "init" => extra::cmd_init(
            &g,
            sub.get_flag("force"),
            sub.get_one::<String>("lang")
                .map(|s| s.as_str())
                .unwrap_or(""),
        ),
        "migrate" => commands::cmd_migrate(&g, sub.get_flag("dry-run")),
        "validate" => commands::cmd_validate(&g),
        "version" => {
            commands::cmd_version();
            Ok(())
        }
        _ => Ok(()),
    };

    match result {
        Ok(()) => 0,
        Err(CmdError::Silent) => 1,
        Err(CmdError::Msg(m)) => {
            eprintln!("Error: {m}");
            1
        }
    }
}

fn fmt(m: &ArgMatches) -> &str {
    m.get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("text")
}

// --only: cobra StringSliceVar allows comma-separated values and repeated flags — handled the same way.
fn only(m: &ArgMatches) -> Vec<String> {
    m.get_many::<String>("only")
        .map(|vals| {
            vals.flat_map(|v| v.split(',').map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn t(key: &str) -> String {
    git_warden_i18n::t!(key)
}

fn build_cli() -> Command {
    let format_arg = || {
        Arg::new("format")
            .long("format")
            .default_value("text")
            .help(t("flag.format"))
    };
    let only_arg = || {
        Arg::new("only")
            .long("only")
            .action(ArgAction::Append)
            .help(t("flag.only"))
    };

    Command::new("git-warden")
        .version(git_warden_version::version())
        .about(t("cmd.root.short"))
        .long_about(t("cmd.root.long"))
        .subcommand_required(false)
        .arg_required_else_help(false)
        .arg(
            Arg::new("config")
                .long("config")
                .global(true)
                .default_value(".git-warden.yaml")
                .help(t("flag.config")),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .global(true)
                .action(ArgAction::SetTrue)
                .help(t("flag.quiet")),
        )
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .global(true)
                .action(ArgAction::SetTrue)
                .help(t("flag.no_color")),
        )
        .arg(
            Arg::new("no-guide")
                .long("no-guide")
                .global(true)
                .action(ArgAction::SetTrue)
                .help(t("flag.no_guide")),
        )
        .arg(
            Arg::new("require-config")
                .long("require-config")
                .global(true)
                .action(ArgAction::SetTrue)
                .help(t("flag.require_config")),
        )
        .subcommand(
            Command::new("run")
                .about(t("cmd.run.short"))
                .long_about(t("cmd.run.long"))
                .arg(format_arg())
                .arg(only_arg()),
        )
        .subcommand(
            Command::new("diff")
                .about(t("cmd.diff.short"))
                .long_about(t("cmd.diff.long"))
                .arg(format_arg())
                .arg(
                    Arg::new("staged")
                        .long("staged")
                        .alias("cached")
                        .action(ArgAction::SetTrue)
                        .help(t("flag.diff_staged")),
                )
                .arg(only_arg())
                .arg(Arg::new("args").num_args(0..=2).action(ArgAction::Append)),
        )
        .subcommand(
            Command::new("msg")
                .about(t("cmd.msg.short"))
                .long_about(t("cmd.msg.long"))
                .arg(
                    Arg::new("fix")
                        .long("fix")
                        .action(ArgAction::SetTrue)
                        .help(t("flag.msg_fix")),
                )
                .arg(Arg::new("file").required(true).num_args(1)),
        )
        .subcommand(
            Command::new("push")
                .about(t("cmd.push.short"))
                .long_about(t("cmd.push.long"))
                .arg(Arg::new("range").long("range").help(t("flag.push_range"))),
        )
        .subcommand(
            Command::new("prepare-msg")
                .about(t("cmd.prepare_msg.short"))
                .long_about(t("cmd.prepare_msg.long"))
                .arg(
                    Arg::new("args")
                        .num_args(1..=3)
                        .action(ArgAction::Append)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("fix")
                .about(t("cmd.fix.short"))
                .long_about(t("cmd.fix.long"))
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help(t("flag.fix_dry_run")),
                ),
        )
        .subcommand(
            Command::new("clean")
                .about(t("cmd.clean.short"))
                .long_about(t("cmd.clean.long"))
                .arg(
                    Arg::new("yes")
                        .long("yes")
                        .action(ArgAction::SetTrue)
                        .help("delete untracked files (without this flag, runs in dry-run)"),
                ),
        )
        .subcommand(
            Command::new("analyze")
                .about(t("cmd.analyze.short"))
                .long_about(t("cmd.analyze.long")),
        )
        .subcommand(
            Command::new("init")
                .about(t("cmd.init.short"))
                .long_about(t("cmd.init.long"))
                .arg(
                    Arg::new("force")
                        .long("force")
                        .action(ArgAction::SetTrue)
                        .help(t("flag.init_force")),
                )
                .arg(Arg::new("lang").long("lang").help(t("flag.init_lang"))),
        )
        .subcommand(
            Command::new("migrate")
                .about(t("cmd.migrate.short"))
                .long_about(t("cmd.migrate.long"))
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help(t("flag.migrate_dry_run")),
                ),
        )
        .subcommand(
            Command::new("validate")
                .about(t("cmd.validate.short"))
                .long_about(t("cmd.validate.long")),
        )
        .subcommand(Command::new("version").about(t("cmd.version.short")))
}
