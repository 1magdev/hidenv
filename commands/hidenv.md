---
description: Reads environment values via hidenv (masks by default); /hidenv off disables the guard, /hidenv on re-enables it
---

Arguments: `$ARGUMENTS`

- If `$ARGUMENTS` is `off`: the user disabled the hidenv guard for this session.
  Acknowledge briefly. From now on you may read `.env*` directly (cat/sed/grep/Read)
  and dump the shell env (env/printenv/set) without routing through `hidenv`.
- If `$ARGUMENTS` is `on`: the guard is back. Route every `.env*` read and every
  shell-env read through `hidenv` again, and confirm before any reveal.
- Otherwise: run `hidenv $ARGUMENTS` via bash and show the output in chat.

Default behavior (when the guard is on):
- No source: masks the current machine's shell environment (`hidenv`, or `--keys`/`--show`/`--all` without a file).
- No flag: masked output (`###`). Show it as-is.
- With `--show KEY` or `--all`: before running, confirm with the user that they want to reveal the real value.
- Remote: `-S user@host:/path/.env` reads a remote .env over SSH; `-S user@host` reads the remote machine's environment; add `-i key.pem` for an identity or `-t` to force a PTY.
- Forbidden: `cat`/`sed`/`grep`/`head`/`tail`/`awk`/`less`/`bat`/`source` or direct reading of `.env*`. Also never `env`/`printenv`/`set`/`export`/`declare`/`compgen -e`. Always via `hidenv`.
