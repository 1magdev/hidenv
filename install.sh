#!/usr/bin/env bash
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPC="$HOME/.config/opencode"
CLAUDE="$HOME/.claude"
LOCAL_BIN="$HOME/.local/bin"

echo "==> hidenv install"

# 1. build (Rust)
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "    error: cargo not found. Install Rust: https://rustup.rs" >&2
    exit 1
fi
echo "    build: cargo build --release"
(cd "$REPO_DIR" && cargo build --release)

# 2. tool
mkdir -p "$LOCAL_BIN"
if [ -L "$LOCAL_BIN/hidenv" ]; then
    rm -f "$LOCAL_BIN/hidenv"
    echo "    removed old symlink: $LOCAL_BIN/hidenv"
fi
install -m 0755 "$REPO_DIR/target/release/hidenv" "$LOCAL_BIN/hidenv"
echo "    binary: $LOCAL_BIN/hidenv"

# 3. skills
mkdir -p "$OPC/skills" "$CLAUDE/skills"
rm -rf "$OPC/skills/hidenv" "$CLAUDE/skills/hidenv"
cp -r "$REPO_DIR/skills/hidenv" "$OPC/skills/hidenv"
cp -r "$REPO_DIR/skills/hidenv" "$CLAUDE/skills/hidenv"
echo "    skill: opencode + claude"

# 4. commands
mkdir -p "$OPC/commands" "$CLAUDE/commands"
cp "$REPO_DIR/commands/hidenv.md" "$OPC/commands/hidenv.md"
cp "$REPO_DIR/commands/hidenv.md" "$CLAUDE/commands/hidenv.md"
echo "    command: /hidenv (opencode + claude)"

# 5. instructions (opencode guard)
mkdir -p "$OPC/instructions"
cp "$REPO_DIR/instructions/hidenv.md" "$OPC/instructions/hidenv.md"
echo "    instruction: $OPC/instructions/hidenv.md"

# 6 + 7. claude user memory guard + permissions (opencode.jsonc + claude settings.json)
python3 - "$OPC" "$CLAUDE" "$REPO_DIR" <<'PY'
import json, os, sys
opc, claude, repo_dir = sys.argv[1], sys.argv[2], sys.argv[3]

# claude user memory guard: replace any previous hidenv block with the current one
mem = os.path.join(claude, "CLAUDE.md")
block = open(os.path.join(repo_dir, "claude", "CLAUDE.md"), encoding="utf-8").read().strip("\n")
text = open(mem, encoding="utf-8").read() if os.path.exists(mem) else ""
lines = text.split("\n")
start = None
for i, ln in enumerate(lines):
    if ln.startswith("# hidenv"):
        start = i
        break
if start is not None:
    if start > 0 and lines[start - 1].strip() == "":
        start -= 1
    lines = lines[:start]
text = "\n".join(lines)
if text and not text.endswith("\n"):
    text += "\n"
text += "\n" + block + "\n"
open(mem, "w", encoding="utf-8").write(text)
print(f"    claude memory: synced to {mem}")

# opencode.jsonc
p = os.path.join(opc, "opencode.jsonc")
try:
    data = json.load(open(p, encoding="utf-8"))
except Exception as e:
    print(f"    [warn] opencode.jsonc not editable: {e}")
    data = None
if data is not None:
    inst = data.setdefault("instructions", [])
    tgt = ".config/opencode/instructions/hidenv.md"
    if tgt not in inst:
        inst.append(tgt)
    perm = data.setdefault("permission", {})
    read = perm.setdefault("read", {})
    read.update({"*": "allow", "*.env": "deny", "*.env.*": "deny", "*.env.example": "allow"})
    bash = perm.setdefault("bash", {})
    bash.setdefault("*", "allow")
    bash["hidenv *"] = "allow"
    for pat in ["cat *.env*", "cat .env*", "sed *.env*", "grep *.env*", "head *.env*",
                "tail *.env*", "awk *.env*", "less *.env*", "bat *.env*", "source *.env*",
                "env", "env *", "printenv", "printenv *", "set", "export",
                "declare", "declare *", "compgen *"]:
        bash[pat] = "deny"
    json.dump(data, open(p, "w", encoding="utf-8"), indent=2, ensure_ascii=False)
    print("    permission: opencode.jsonc updated")

# claude settings.json
p = os.path.join(claude, "settings.json")
try:
    data = json.load(open(p, encoding="utf-8"))
except Exception as e:
    print(f"    [warn] settings.json not editable: {e}")
    data = None
if data is not None:
    perm = data.setdefault("permissions", {})
    allow = perm.setdefault("allow", [])
    deny = perm.setdefault("deny", [])
    def add(lst, item):
        if item not in lst:
            lst.append(item)
    add(allow, "Bash(hidenv *)")
    for item in ["Read(//**/.env)", "Read(//**/.env.*)", "Read(//**/*.env)",
                 "Read(//**/*.env.*)", "Bash(cat *.env*)", "Bash(cat .env*)",
                 "Bash(sed *.env*)", "Bash(grep *.env*)", "Bash(head *.env*)",
                 "Bash(tail *.env*)", "Bash(awk *.env*)", "Bash(less *.env*)",
                 "Bash(bat *.env*)", "Bash(source *.env*)",
                 "Bash(env)", "Bash(env *)", "Bash(printenv)", "Bash(printenv *)",
                 "Bash(set)", "Bash(export)", "Bash(declare)", "Bash(declare *)",
                 "Bash(compgen *)"]:
        add(deny, item)
    json.dump(data, open(p, "w", encoding="utf-8"), indent=2, ensure_ascii=False)
    print("    permission: settings.json updated")
PY

echo "==> done"
