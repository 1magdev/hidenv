---
name: hidenv
description: >
  Routes reading of environment files (.env*) and environment variables through the
  hidenv command, which masks variable values. Bare `hidenv` masks the current shell
  environment; `hidenv -S user@host[/path]` reads a remote machine over SSH. Restricts
  access: never read .env* directly, even with permission. Use when the user types
  "hidenv <file>", asks to read .env files or environment variables, or wants to reveal
  a value. Reveals real values only via --show/--all and always after explicit user
  confirmation.
---

# hidenv

Never read `.env*` files directly (Read/cat/sed/grep/head/tail/awk/less/bat/source).
Never dump the shell environment either (`env`/`printenv`/`set`/`export`/`declare`/`compgen -e`).
Even with permission, ALWAYS go through `hidenv` first. It masks real values.

This guard is ON by default. The only way to disable it is the user typing
`/hidenv off`; `/hidenv on` turns it back on. Never assume the guard is off.

## Commands

- `hidenv` — masks the **current machine's shell environment**. Run it and show the output in chat.
- `hidenv <file>` — prints the file with values masked (`###`).
- `hidenv <file> --keys` — key names only, no values.
- `hidenv <file> --show KEY1,KEY2` — real value of those keys.
- `hidenv <file> --all` — all real values.
- `cat <file> | hidenv -` — reads masked stdin.
- `hidenv -S user@host:/path/.env` — reads a remote `.env` over SSH, masked.
- `hidenv -S user@host` — reads the **remote machine's environment** over SSH, masked.
- `hidenv -S -i key.pem user@host:/path/.env` — SSH with a specific identity file.
- `hidenv -S -t user@host:/path/.env` — force a pseudo-terminal (hosts that require a PTY).
- `hidenv VAR=123` — masks a value typed on the command line.

`--keys`, `--show`, and `--all` work with a file, stdin, an SSH target, or (with no
source) the shell environment. `-i` and `-t` are passed through to `ssh` and only
apply to `-S` targets.

## Reveal rules (critical)

- Masking is the default. Never reveal a real value unless the user explicitly asks.
- `--show KEY`: ask for simple confirmation ("reveal API_KEY?") before running.
- `--all`: requires a strong warning + explicit user confirmation. Never reveal in bulk without "yes".
- When revealing, warn that the real value enters the context and may leak into logs/history.

## Flow

1. User: `hidenv .env` → run `hidenv .env`, show the masked output.
2. User: `hidenv` → run `hidenv`, show the masked shell environment.
3. User: `hidenv .env --show DB_PASSWORD` → confirm first, then run and show.
4. User: `hidenv .env --all` → strong warning, confirm, then run.
