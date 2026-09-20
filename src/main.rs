use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use comment_warden::config::Config;
use comment_warden::scan::{self, Mode, Outcome};
use comment_warden::strip::FileReport;

#[derive(Parser)]
#[command(
    name = "comment-warden",
    about = "An AST comment warden that strips comments which stop no edit"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct CommonOpts {
    #[arg(long, value_name = "FILE")]
    exclude_from: Option<PathBuf>,
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    Check {
        #[command(flatten)]
        common: CommonOpts,
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    Strip {
        #[command(flatten)]
        common: CommonOpts,
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    Hook {
        #[command(flatten)]
        common: CommonOpts,
    },
    Advice {
        #[command(flatten)]
        common: CommonOpts,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("comment-warden: {e:#}");
            ExitCode::from(3)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::Check { common, paths } => {
            let (config, excludes) = load(&common)?;
            let totals = scan::run(&paths, &excludes, &config, Mode::Check)?;
            report_check(&totals);
            Ok(exit(totals.outcome()))
        }
        Command::Strip { common, paths } => {
            let (config, excludes) = load(&common)?;
            let totals = scan::run(&paths, &excludes, &config, Mode::Strip)?;
            report_strip(&totals);
            Ok(exit(totals.outcome()))
        }
        Command::Hook { common } => run_hook(&common),
        Command::Advice { common } => {
            let config = Config::load(common.config.as_deref())?;
            let advice = config.advice()?;
            print_session_start(&advice);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn load(common: &CommonOpts) -> Result<(Config, HashSet<PathBuf>)> {
    let config = Config::load(common.config.as_deref())?;
    let excludes = match &common.exclude_from {
        Some(p) => scan::read_exclude_from(p)?,
        None => HashSet::new(),
    };
    Ok((config, excludes))
}

fn exit(outcome: Outcome) -> ExitCode {
    ExitCode::from(outcome.exit_code() as u8)
}

fn report_check(totals: &scan::RunTotals) {
    for (path, report) in &totals.reports {
        for (row, line) in &report.untagged_lines {
            println!("{}:{}: untagged: {}", path.display(), row + 1, line);
        }
        for (row, line) in &report.left_lines {
            println!("{}:{}: left in place: {}", path.display(), row + 1, line);
        }
        for (row, line) in &report.unclassified_lines {
            println!("{}:{}: unclassified: {}", path.display(), row + 1, line);
        }
    }
}

fn report_strip(totals: &scan::RunTotals) {
    println!("stripped {} untagged comment(s)", totals.stripped);
    for (path, report) in &totals.reports {
        for (row, line) in &report.left_lines {
            println!("{}:{}: left in place: {}", path.display(), row + 1, line);
        }
        for (row, line) in &report.unclassified_lines {
            println!("{}:{}: unclassified: {}", path.display(), row + 1, line);
        }
    }
}

#[derive(Deserialize)]
struct HookPayload {
    tool_input: Option<ToolInput>,
}

#[derive(Deserialize)]
struct ToolInput {
    file_path: Option<PathBuf>,
}

fn run_hook(common: &CommonOpts) -> Result<ExitCode> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("reading hook payload from stdin")?;

    let payload: HookPayload = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(_) => return Ok(ExitCode::SUCCESS),
    };
    let Some(path) = payload.tool_input.and_then(|t| t.file_path) else {
        return Ok(ExitCode::SUCCESS);
    };

    let config = Config::load(common.config.as_deref()).unwrap_or_else(|_| Config::empty());
    let excludes = match &common.exclude_from {
        Some(p) => scan::read_exclude_from(p).unwrap_or_default(),
        None => HashSet::new(),
    };
    let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
    if excludes.contains(&canonical) || excludes.contains(&path) {
        return Ok(ExitCode::SUCCESS);
    }

    match scan::process_one(&path, &config, Mode::Strip) {
        Ok(Some(report)) => {
            if let Some(msg) = hook_receipt(&path, &report) {
                print_post_tool_use(&msg);
            }
        }
        Ok(None) => {}
        Err(_) => {
            print_post_tool_use(&format!(
                "comment-warden: could not check {} — comments there are unchecked",
                path.display()
            ));
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn hook_receipt(path: &std::path::Path, report: &FileReport) -> Option<String> {
    if report.stripped == 0 && report.left_in_place == 0 && report.unclassified == 0 {
        return None;
    }
    let mut msg = format!(
        "comment-warden: stripped {} untagged comment(s) from {}",
        report.stripped,
        path.display()
    );
    if report.left_in_place > 0 {
        msg.push_str(&format!(", left {} in place", report.left_in_place));
    }
    if report.unclassified > 0 {
        msg.push_str(&format!(", {} unclassified line(s)", report.unclassified));
    }
    Some(msg)
}

#[derive(Serialize)]
struct HookOutput<'a> {
    #[serde(rename = "hookSpecificOutput")]
    hook_specific_output: HookSpecificOutput<'a>,
}

#[derive(Serialize)]
struct HookSpecificOutput<'a> {
    #[serde(rename = "hookEventName")]
    hook_event_name: &'a str,
    #[serde(rename = "additionalContext")]
    additional_context: &'a str,
}

fn print_hook_output(event: &str, context: &str) {
    let out = HookOutput {
        hook_specific_output: HookSpecificOutput {
            hook_event_name: event,
            additional_context: context,
        },
    };
    println!("{}", serde_json::to_string(&out).unwrap());
}

fn print_post_tool_use(context: &str) {
    print_hook_output("PostToolUse", context);
}

fn print_session_start(context: &str) {
    print_hook_output("SessionStart", context);
}
