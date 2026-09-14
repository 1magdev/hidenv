//! hidenv — masks secret values in environment files.
//!
//! Thin entry point: parse arguments, dispatch to the right behavior, and
//! map the result to a process exit code. All parsing/masking logic lives
//! in [`cli`] and [`mask`]; remote fetching lives in [`ssh`].

mod cli;
mod mask;
mod ssh;

use std::collections::BTreeSet;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

use cli::{CliError, Config, Mode, HELP};
use mask::{extract_pairs, mask_line, parse_assignment};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let config = match cli::parse_args(&args) {
        Ok(config) => config,
        Err(CliError::HelpRequested) => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("hidenv: {err}");
            return ExitCode::FAILURE;
        }
    };

    if run(config) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Executes the requested behavior. Returns `false` on error.
fn run(config: Config) -> bool {
    match &config.mode {
        Mode::Mask => run_mask(&config),
        Mode::Keys | Mode::Show(_) | Mode::All => run_reveal(&config),
    }
}

/// Streams masked output for files, SSH targets, and free-form variable text.
/// When no source is given, masks the current shell environment.
fn run_mask(config: &Config) -> bool {
    let mut had_source = false;

    for path in &config.files {
        if let Err(err) = with_file(path, &mut mask_reader) {
            eprintln!("hidenv: {err}");
            return false;
        }
        had_source = true;
    }
    for target in &config.ssh_targets {
        if let Err(err) = with_ssh(target, config, &mut mask_reader) {
            eprintln!("hidenv: {err}");
            return false;
        }
        had_source = true;
    }
    if let Some(text) = &config.var_text {
        for (name, _) in extract_pairs(text) {
            println!("{name}=###");
        }
        had_source = true;
    }

    if !had_source {
        mask_env();
    }
    true
}

/// Handles `--keys`, `--show`, and `--all`.
/// When no source is given, reveals from the current shell environment.
fn run_reveal(config: &Config) -> bool {
    match &config.mode {
        Mode::Show(keys) => warn_show(keys),
        Mode::All => warn_all(),
        _ => {}
    }

    if config.files.is_empty() && config.ssh_targets.is_empty() {
        return reveal_env(&config.mode);
    }

    let mut all_ok = true;
    for path in &config.files {
        match with_file(path, &mut |r| reveal_reader(r, &config.mode)) {
            Ok(found_all) => all_ok &= found_all,
            Err(err) => {
                eprintln!("hidenv: {err}");
                all_ok = false;
            }
        }
    }
    for target in &config.ssh_targets {
        match with_ssh(target, config, &mut |r| reveal_reader(r, &config.mode)) {
            Ok(found_all) => all_ok &= found_all,
            Err(err) => {
                eprintln!("hidenv: {err}");
                all_ok = false;
            }
        }
    }
    all_ok
}

/// Runs `f` against a buffered reader over a local file (or stdin for `-`).
fn with_file<R>(path: &str, f: &mut dyn FnMut(&mut dyn BufRead) -> io::Result<R>) -> io::Result<R> {
    if path == "-" {
        let stdin = io::stdin();
        f(&mut BufReader::new(stdin.lock()))
    } else {
        let file = std::fs::File::open(path)?;
        f(&mut BufReader::new(file))
    }
}

/// Runs `f` against a buffered reader over a remote file fetched via SSH.
fn with_ssh<R>(
    target: &str,
    config: &Config,
    f: &mut dyn FnMut(&mut dyn BufRead) -> io::Result<R>,
) -> io::Result<R> {
    let remote = ssh::parse_target(target)?;
    let data = ssh::fetch(&remote, config.ssh_identity.as_deref(), config.ssh_tty)?;
    f(&mut BufReader::new(io::Cursor::new(data)))
}

/// Streams masked output from a reader to stdout.
fn mask_reader(reader: &mut dyn BufRead) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        write!(out, "{}", mask_line(&line))?;
    }
    out.flush()
}

/// Returns the shell environment sorted by variable name.
fn env_vars() -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    vars
}

/// Prints every environment variable with its value masked.
fn mask_env() {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (name, _) in env_vars() {
        let _ = writeln!(out, "{name}=###");
    }
    let _ = out.flush();
}

/// Reveals values from the shell environment according to the active mode.
fn reveal_env(mode: &Mode) -> bool {
    let mut seen = BTreeSet::new();
    for (name, value) in env_vars() {
        reveal_entry(&name, &value, mode, &mut seen);
    }
    check_missing(mode, &seen)
}

/// Reveals values from a reader according to the active mode.
///
/// Returns `Ok(true)` when every requested `--show` key was found,
/// `Ok(false)` otherwise. Missing keys are reported on stderr.
fn reveal_reader(reader: &mut dyn BufRead, mode: &Mode) -> io::Result<bool> {
    let mut seen = BTreeSet::new();
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let Some((name, value)) = parse_assignment(&line) else {
            continue;
        };
        reveal_entry(name, value, mode, &mut seen);
    }

    Ok(check_missing(mode, &seen))
}

/// Prints a single name/value pair according to the active mode, recording
/// any requested key that was found.
fn reveal_entry(name: &str, value: &str, mode: &Mode, seen: &mut BTreeSet<String>) {
    match mode {
        Mode::Keys => println!("{name}"),
        Mode::All => println!("{name}={value}"),
        Mode::Show(keys) if keys.contains(name) => {
            println!("{name}={value}");
            seen.insert(name.to_string());
        }
        _ => {}
    }
}

/// Reports any requested `--show` key that was not found. Returns `true` when
/// every key was found.
fn check_missing(mode: &Mode, seen: &BTreeSet<String>) -> bool {
    match mode {
        Mode::Show(keys) => {
            let missing: Vec<&String> = keys.difference(seen).collect();
            for name in &missing {
                eprintln!("hidenv: key not found: {name}");
            }
            missing.is_empty()
        }
        _ => true,
    }
}

fn warn_show(keys: &BTreeSet<String>) {
    let list: Vec<&str> = keys.iter().map(String::as_str).collect();
    eprintln!(
        "⚠ hidenv: revealing real value of: {}. Be careful when pasting into chat.",
        list.join(", ")
    );
}

fn warn_all() {
    eprintln!(
        "⚠ hidenv: revealing ALL real values. Do not paste secrets into public chat or logs."
    );
}
