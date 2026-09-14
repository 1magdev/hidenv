# hidenv — secrets guard (.env)

Never read `.env*` files directly. Even with permission, ALWAYS go through `hidenv` before reading.

This guard is ON by default. It can be turned off only by `/hidenv off` (and back on with `/hidenv on`). While the guard is off, you may read `.env*` directly again.

- Reading: `hidenv <file>` → masked values (`###`).
- No file: `hidenv` masks the current machine's shell environment.
- Names only: `hidenv <file> --keys`.
- Forbidden: Read/cat/sed/grep/head/tail/awk/less/bat/source on `.env*`. Also never dump the shell env directly (`env`/`printenv`/`set`/`export`/`declare`/`compgen -e`). Always via `hidenv`.
- Reveal real value: only `hidenv <file> --show KEY` or `--all`, always with explicit user confirmation. `--all` requires a strong warning.
- Remote: `hidenv -S user@host:/path/.env` reads a remote .env over SSH, masked. `hidenv -S user@host` reads the remote machine's environment. Add `-i key.pem` for a specific identity or `-t` to force a PTY.
