use clap::{Args, Parser, Subcommand};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    process::ExitCode,
};
#[derive(Parser)]
#[command(
    name = "homecheck",
    version,
    about = "Offline bounded automation verification"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Args)]
struct Input {
    #[arg(long)]
    rules: PathBuf,
    #[arg(long)]
    scenario: PathBuf,
    #[arg(long,default_value="json",value_parser=["json","markdown","junit"])]
    format: String,
    #[arg(long)]
    out: Option<PathBuf>,
}
#[derive(Subcommand)]
enum Command {
    Lint(Input),
    Simulate(Input),
    Verify(Input),
    Minimize(Input),
    Report(Input),
    Diff {
        #[command(flatten)]
        input: Input,
        #[arg(long)]
        before: PathBuf,
    },
}
fn read(path: &PathBuf, limit: u64) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("input must be a regular file".into());
    }
    let mut s = String::new();
    file.take(limit + 1)
        .read_to_string(&mut s)
        .map_err(|e| e.to_string())?;
    if s.len() as u64 > limit {
        return Err(format!("{} exceeds byte limit", path.display()));
    }
    Ok(s)
}
fn run() -> Result<u8, String> {
    let cli = Cli::parse();
    let (op, input, before) = match cli.command {
        Command::Lint(i) => ("lint", i, None),
        Command::Simulate(i) => ("simulate", i, None),
        Command::Verify(i) => ("verify", i, None),
        Command::Minimize(i) => ("minimize", i, None),
        Command::Report(i) => ("report", i, None),
        Command::Diff { input, before } => ("diff", input, Some(before)),
    };
    let yaml = read(&input.rules, 131072)?;
    let scenario = read(&input.scenario, 262144)?;
    let v = if let Some(path) = before {
        home_rule_ha::diff(&read(&path, 131072)?, &yaml, &scenario)?
    } else {
        home_rule_ha::analyze(op, &yaml, &scenario)?
    };
    let text = home_rule_ha::render(&v, &input.format)?;
    if text.len() > 33554432 {
        return Err("output exceeds 32 MiB".into());
    }
    if let Some(path) = input.out {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}; outputs never overwrite", path.display()))?;
        f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    } else {
        std::io::stdout()
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(home_rule_ha::exit_code(&v))
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("homecheck: {e}");
            ExitCode::from(2)
        }
    }
}
