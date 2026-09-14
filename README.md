# hidenv

Keep your secrets out of the chat.

**hidenv** reads environment files (`.env` and friends) and hides the secret
values before they reach your screen, your AI agent, or a log file. By default
it shows only `###` instead of the real value.

## Why?

AI agents and coding assistants often read `.env` files. Those files contain
passwords, API keys, and tokens. If an agent reads them directly, the secrets
end up in the chat history, where they can leak. `hidenv` makes sure an agent
only sees masked values — unless you explicitly ask to reveal them.

## What it does

| Command                          | Result                                    |
| -------------------------------- | ----------------------------------------- |
| `hidenv`                         | Masks the current shell environment       |
| `hidenv .env`                    | Shows the file with all values as `###`   |
| `hidenv .env --keys`             | Shows only the names, no values           |
| `hidenv .env --show API_KEY`     | Shows the real value of `API_KEY`         |
| `hidenv .env --all`              | Shows every real value (be careful!)      |
| `cat .env \| hidenv -`           | Masks output piped from another command   |
| `hidenv MY_VAR=123`              | Masks a value typed on the command line   |
| `hidenv -S user@host:/path/.env` | Reads a remote `.env` over SSH, masked    |
| `hidenv -S user@host`            | Reads a remote machine's env over SSH      |

Example:

```console
$ hidenv .env
DB_HOST=###
DB_PASSWORD=###
API_KEY=###
```

## Remote files (SSH)

Use the `-S` flag to read a `.env` on a remote machine without copying the
secrets to your local disk. The values are masked on the way out:

```console
$ hidenv -S deploy@server:/etc/app/.env
DB_HOST=###
DB_PASSWORD=###
```

`hidenv` runs `ssh <host> -- cat <path>` under the hood, so it respects your
normal `~/.ssh/config` (aliases, keys, ports). A bare host (no `:path`) reads
the remote machine's environment instead:

```console
$ hidenv -S deploy@server   # remote shell environment, masked
```

Extra SSH options are available:

- `-i key.pem` — use a specific identity file (`ssh -i`).
- `-t` — force a pseudo-terminal (`ssh -tt`, works even without a local TTY),
  needed by hosts that reject sessions without a real PTY (e.g. RunPod).

```console
$ hidenv -S -i key.pem -t deploy@server:/etc/app/.env --keys
```

You can combine these with the other flags, e.g. `--keys` or `--show`.

## Shell environment

With no file given, `hidenv` falls back to the current shell environment:

```console
$ hidenv              # every variable masked
PATH=###
HOME=###
API_KEY=###

$ hidenv --keys       # names only
$ hidenv --show API_KEY
```

## Install

**Requisito obrigatório:** você precisa do `cargo` instalado. Se não tiver,
instale o Rust primeiro em [rustup.rs](https://rustup.rs) (inclui o `cargo`):

- Linux / macOS: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Windows: baixe e execute `rustup-init.exe` de [rustup.rs](https://rustup.rs)

Depois instale a versão:

```bash
cargo install --git https://github.com/1magdev/hidenv --tag v0.1.0
```

Funciona em Linux, macOS e Windows.

To also install the agent-integration files (skills, commands, and
instructions for OpenCode and Claude Code), clone the repo and run the
installer:

```bash
git clone https://github.com/1magdev/hidenv
cd hidenv
./install.sh
```

The installer builds the binary and places it in `~/.local/bin`.

Make sure `~/.local/bin` is on your `PATH`.

## Safety rules

- Masking is the default. Real values are **never** shown unless you ask.
- `--show KEY` reveals a single key — confirm before sharing it anywhere.
- `--all` reveals everything. Use it with extreme care and never paste the
  output into public chats or logs.

## Agent harness

The agent guard is **on by default**: agents must route every `.env*` read and
every shell-environment read through `hidenv`. To disable it for a session,
use the slash command `/hidenv off`; re-enable it with `/hidenv on`.

## Development

```bash
cargo build --release   # compile
cargo test              # run unit + integration tests
cargo fmt               # format
cargo clippy -- -D warnings   # lint
```

The CLI is written in Rust with no external dependencies.
