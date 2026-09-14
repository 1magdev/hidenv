//! Command-line argument parsing.
//!
//! Turns `std::env::args()` into a [`Config`] describing what the user
//! wants hidenv to do.

use std::collections::BTreeSet;
use std::fmt;

/// How the values should be presented.
#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    /// Mask every value (the default).
    Mask,
    /// Print only the key names.
    Keys,
    /// Print the real value of the requested keys.
    Show(BTreeSet<String>),
    /// Print every real value.
    All,
}

/// Fully parsed invocation.
#[derive(Debug)]
pub struct Config {
    pub mode: Mode,
    /// File paths (or `-` for stdin) to process.
    pub files: Vec<String>,
    /// Remote `user@host:/path` targets to fetch over SSH.
    pub ssh_targets: Vec<String>,
    /// Identity file passed to `ssh -i` for SSH targets.
    pub ssh_identity: Option<String>,
    /// Force a pseudo-terminal (`ssh -t`) for SSH targets.
    pub ssh_tty: bool,
    /// Free-form `NAME=VALUE` text from positional arguments.
    pub var_text: Option<String>,
}

/// Reasons argument parsing can fail.
#[derive(Debug)]
pub enum CliError {
    /// Help was requested via `-h`/`--help`.
    HelpRequested,
    /// `--show` was given without a value.
    ShowMissingValue,
    /// `--show` was given an empty key list.
    ShowRequiresKey,
    /// A flag that expects a value (`-i`) was given without one.
    MissingValue(&'static str),
    /// An unrecognized flag was supplied.
    UnknownFlag(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::HelpRequested => write!(f, "help requested"),
            CliError::ShowMissingValue => write!(f, "--show requires at least one key"),
            CliError::ShowRequiresKey => write!(f, "--show requires at least one key"),
            CliError::MissingValue(flag) => write!(f, "{flag} requires a value"),
            CliError::UnknownFlag(flag) => write!(f, "unknown flag: {flag}"),
        }
    }
}

pub const HELP: &str = "\
hidenv - hides values of environment variables.

Usage:
    hidenv                            # masks the current shell environment
    hidenv file.txt                   # prints the file with values masked
    hidenv .env                       # same behavior
    hidenv MY_VAR=123                 # prints MY_VAR=###
    hidenv MY_VAR = 123               # also works
    hidenv VAR1=1 VAR2 = 2            # multiple variables
    cat .env | hidenv -               # read from stdin
    hidenv -S user@host:/path/.env    # read a remote .env over SSH (masked)
    hidenv -S user@host               # read the remote machine's env over SSH
    hidenv -S -i key.pem user@host:/path/.env   # SSH with a specific key
    hidenv -S -t user@host:/path/.env           # force a pseudo-terminal (ssh -tt)

Reveal (on demand):
    hidenv --keys                     # key names only (shell environment)
    hidenv .env --keys                # key names only (from a file)
    hidenv .env --show KEY1,KEY2      # real values of those keys
    hidenv .env --all                 # all real values
";

/// Parses the CLI arguments into a [`Config`].
///
/// `-h`/`--help` surfaces [`CliError::HelpRequested`]; the caller decides
/// the exit code and whether output goes to stdout.
pub fn parse_args(args: &[String]) -> Result<Config, CliError> {
    let mut mode = Mode::Mask;
    let mut ssh_mode = false;
    let mut ssh_identity: Option<String> = None;
    let mut ssh_tty = false;
    let mut positionals: Vec<String> = Vec::new();
    let mut ssh_targets: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-h" | "--help" => return Err(CliError::HelpRequested),
            "--all" => mode = Mode::All,
            "--keys" => mode = Mode::Keys,
            "-S" | "--ssh" => ssh_mode = true,
            "-t" => ssh_tty = true,
            "-i" => {
                let value = args.get(i + 1).ok_or(CliError::MissingValue("-i"))?;
                ssh_identity = Some(value.clone());
                i += 1;
            }
            "--show" => {
                let value = args.get(i + 1).ok_or(CliError::ShowMissingValue)?;
                mode = Mode::Show(parse_show(value)?);
                i += 1;
            }
            _ if arg.starts_with("--show=") => {
                mode = Mode::Show(parse_show(&arg["--show=".len()..])?);
            }
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(CliError::UnknownFlag(arg.to_string()));
            }
            _ => {
                if ssh_mode {
                    ssh_targets.push(arg.to_string());
                } else {
                    positionals.push(arg.to_string());
                }
            }
        }
        i += 1;
    }

    let mut files = Vec::new();
    let mut var_parts = Vec::new();
    for arg in positionals {
        if arg == "-" || is_file(&arg) {
            files.push(arg);
        } else {
            var_parts.push(arg);
        }
    }

    let var_text = if var_parts.is_empty() {
        None
    } else {
        Some(var_parts.join(" "))
    };

    Ok(Config {
        mode,
        files,
        ssh_targets,
        ssh_identity,
        ssh_tty,
        var_text,
    })
}

/// Splits a comma-separated `--show` value into an ordered set of keys.
fn parse_show(value: &str) -> Result<BTreeSet<String>, CliError> {
    let keys: BTreeSet<String> = value
        .split(',')
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
        .collect();
    if keys.is_empty() {
        Err(CliError::ShowRequiresKey)
    } else {
        Ok(keys)
    }
}

fn is_file(path: &str) -> bool {
    std::path::Path::new(path).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn help_flags_request_help() {
        assert!(matches!(
            parse_args(&args(&["-h"])),
            Err(CliError::HelpRequested)
        ));
        assert!(matches!(
            parse_args(&args(&["--help"])),
            Err(CliError::HelpRequested)
        ));
    }

    #[test]
    fn default_mode_is_mask() {
        let cfg = parse_args(&args(&["-"])).unwrap();
        assert_eq!(cfg.mode, Mode::Mask);
    }

    #[test]
    fn keys_mode() {
        let cfg = parse_args(&args(&["-", "--keys"])).unwrap();
        assert_eq!(cfg.mode, Mode::Keys);
    }

    #[test]
    fn all_mode() {
        let cfg = parse_args(&args(&["-", "--all"])).unwrap();
        assert_eq!(cfg.mode, Mode::All);
    }

    #[test]
    fn show_mode_with_space() {
        let cfg = parse_args(&args(&["-", "--show", "A,B"])).unwrap();
        assert_eq!(cfg.mode, Mode::Show(["A".into(), "B".into()].into()));
    }

    #[test]
    fn show_mode_with_equals() {
        let cfg = parse_args(&args(&["-", "--show=A"])).unwrap();
        assert_eq!(cfg.mode, Mode::Show(["A".into()].into()));
    }

    #[test]
    fn show_without_value_is_error() {
        assert!(matches!(
            parse_args(&args(&["-", "--show"])),
            Err(CliError::ShowMissingValue)
        ));
    }

    #[test]
    fn unknown_flag_is_error() {
        assert!(matches!(
            parse_args(&args(&["--nope"])),
            Err(CliError::UnknownFlag(_))
        ));
    }

    #[test]
    fn ssh_flag_collects_targets() {
        let cfg = parse_args(&args(&["-S", "user@host:/a/.env"])).unwrap();
        assert_eq!(cfg.ssh_targets, vec!["user@host:/a/.env"]);
        assert!(cfg.files.is_empty());
    }

    #[test]
    fn ssh_long_flag_collects_targets() {
        let cfg = parse_args(&args(&["--ssh", "host:/a/.env"])).unwrap();
        assert_eq!(cfg.ssh_targets, vec!["host:/a/.env"]);
    }

    #[test]
    fn ssh_targets_after_flag_only() {
        let cfg = parse_args(&args(&["file.txt", "-S", "host:/a/.env"])).unwrap();
        assert_eq!(cfg.ssh_targets, vec!["host:/a/.env"]);
        assert!(cfg.var_text.is_some());
    }

    #[test]
    fn ssh_identity_flag() {
        let cfg = parse_args(&args(&["-S", "-i", "key.pem", "host:/a/.env"])).unwrap();
        assert_eq!(cfg.ssh_identity.as_deref(), Some("key.pem"));
    }

    #[test]
    fn ssh_identity_without_value_is_error() {
        assert!(matches!(
            parse_args(&args(&["-S", "-i"])),
            Err(CliError::MissingValue("-i"))
        ));
    }

    #[test]
    fn ssh_tty_flag() {
        let cfg = parse_args(&args(&["-S", "-t", "host:/a/.env"])).unwrap();
        assert!(cfg.ssh_tty);
    }

    #[test]
    fn collects_files_and_var_text() {
        let cfg = parse_args(&args(&["-", "A=1"])).unwrap();
        assert_eq!(cfg.files, vec!["-"]);
        assert_eq!(cfg.var_text.as_deref(), Some("A=1"));
    }

    #[test]
    fn joins_multiple_var_parts() {
        let cfg = parse_args(&args(&["A=1", "B = 2"])).unwrap();
        assert_eq!(cfg.files.len(), 0);
        assert_eq!(cfg.var_text.as_deref(), Some("A=1 B = 2"));
    }
}
