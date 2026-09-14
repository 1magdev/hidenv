//! Fetching remote environment data over SSH.
//!
//! Keeps hidenv dependency-free by shelling out to the system `ssh` binary.

use std::io;
use std::process::Command;

/// What to fetch from the remote host.
#[derive(Debug, PartialEq, Eq)]
pub enum Remote {
    /// The remote shell environment (runs `ssh <host> -- env`).
    Env { host: String },
    /// A remote file (runs `ssh <host> -- cat <path>`).
    File { host: String, path: String },
}

/// Splits a `user@host:/path` argument into a file target, or a bare
/// `user@host` argument into an environment target.
pub fn parse_target(arg: &str) -> io::Result<Remote> {
    match arg.find(':') {
        Some(colon) => {
            let (host, rest) = arg.split_at(colon);
            let path = &rest[1..];
            if host.is_empty() || path.is_empty() {
                return Err(invalid(arg));
            }
            Ok(Remote::File {
                host: host.to_string(),
                path: path.to_string(),
            })
        }
        None => {
            if arg.is_empty() {
                return Err(invalid(arg));
            }
            Ok(Remote::Env {
                host: arg.to_string(),
            })
        }
    }
}

fn invalid(arg: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid SSH target: {arg} (expected user@host or user@host:/path)"),
    )
}

/// Builds the argument list for `ssh` given the target and options.
fn ssh_args(host: &str, remote_cmd: &[&str], identity: Option<&str>, tty: bool) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(key) = identity {
        args.push("-i".to_string());
        args.push(key.to_string());
    }
    if tty {
        // `-tt` forces PTY allocation even when there is no local tty,
        // which is what some SSH proxies (e.g. RunPod) require.
        args.push("-tt".to_string());
    }
    args.push(host.to_string());
    args.push("--".to_string());
    args.extend(remote_cmd.iter().map(|s| s.to_string()));
    args
}

/// Runs `ssh <host> -- <remote_cmd>` and returns the captured stdout.
///
/// `identity` and `tty` are passed straight through as `ssh -i` and `ssh -tt`.
pub fn fetch(remote: &Remote, identity: Option<&str>, tty: bool) -> io::Result<Vec<u8>> {
    let (host, remote_cmd) = match remote {
        Remote::File { host, path } => (host.as_str(), vec!["cat", path.as_str()]),
        Remote::Env { host } => (host.as_str(), vec!["env"]),
    };

    let output = Command::new("ssh")
        .args(ssh_args(host, &remote_cmd, identity, tty))
        .output()
        .map_err(|err| io::Error::new(err.kind(), format!("ssh: {err}")))?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.trim();
        let msg = if detail.is_empty() {
            format!("ssh exited with {}", output.status)
        } else {
            format!("ssh: {detail}")
        };
        return Err(io::Error::other(msg));
    }

    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_at_host_file() {
        assert_eq!(
            parse_target("deploy@server:/etc/app/.env").unwrap(),
            Remote::File {
                host: "deploy@server".into(),
                path: "/etc/app/.env".into(),
            }
        );
    }

    #[test]
    fn parses_host_only_file() {
        assert_eq!(
            parse_target("server:/tmp/.env").unwrap(),
            Remote::File {
                host: "server".into(),
                path: "/tmp/.env".into(),
            }
        );
    }

    #[test]
    fn parses_bare_host_as_env() {
        assert_eq!(
            parse_target("server").unwrap(),
            Remote::Env {
                host: "server".into()
            }
        );
    }

    #[test]
    fn parses_user_at_host_as_env() {
        assert_eq!(
            parse_target("deploy@server").unwrap(),
            Remote::Env {
                host: "deploy@server".into()
            }
        );
    }

    #[test]
    fn rejects_empty_host() {
        assert!(parse_target(":/tmp/.env").is_err());
    }

    #[test]
    fn rejects_empty_path() {
        assert!(parse_target("server:").is_err());
    }

    #[test]
    fn builds_plain_file_args() {
        let args = ssh_args("host", &["cat", "/tmp/.env"], None, false);
        assert_eq!(args, vec!["host", "--", "cat", "/tmp/.env"]);
    }

    #[test]
    fn builds_env_args() {
        let args = ssh_args("host", &["env"], None, false);
        assert_eq!(args, vec!["host", "--", "env"]);
    }

    #[test]
    fn builds_args_with_identity_and_tty() {
        let args = ssh_args("host", &["cat", "/tmp/.env"], Some("key.pem"), true);
        assert_eq!(
            args,
            vec!["-i", "key.pem", "-tt", "host", "--", "cat", "/tmp/.env"]
        );
    }
}
