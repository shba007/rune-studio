#!/usr/bin/env bash
# Local cross-repo testing for Rune: Studio -> Kit / Infer -> plugins, without pushing or pinning.
#
# Local development only. It needs code-kit, code-tools (and code-infer for `on infer`)
# checked out beside code-studio. Run it from anywhere:  bash scripts/rune-link.sh <command>
# Works in Git Bash on Windows and in bash on Linux. Needs cargo, bun and node.
#
# What it writes:
#   .cargo/config.toml   Cargo redirect of Studio's pinned Kit / Infer to the sibling
#                        folders. Git-ignored here, so it cannot be committed.
#   ../.dev/             beside code-studio, outside every repository: the isolated Kit
#                        plugin store and saved state.
#
# The only tracked files that change while linked are code-studio's two Cargo.lock
# files (Cargo rewrites them). `off` puts both back.
# Builds that pass --locked in code-studio fail while linked; that is intended.
#
#   rune-link on [kit] [infer]   redirect Studio to local code-kit and/or code-infer (default: kit)
#   rune-link plugin <name>      build a plugin from code-tools and install it into the dev store
#   rune-link studio             run Studio (tauri:dev) against the dev store
#   rune-link kit <args...>      run the local Kit CLI against the dev store
#   rune-link status             show what is linked
#   rune-link off                remove the redirect and restore the lock files
#   rune-link reset              off, and delete the dev store
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
studio="$(dirname "$here")"
# The folder that holds code-studio and its sibling checkouts.
root="$(dirname "$studio")"
for sibling in code-kit code-tools; do
  [ -d "$root/$sibling" ] || {
    echo "Expected $sibling beside code-studio in $root. This script needs the sibling checkouts." >&2
    exit 1
  }
done

cargo_config="$studio/.cargo/config.toml"
dev="$root/.dev"
store="$dev/kit-data"
marker='# managed by scripts/rune-link.sh - do not edit; run "rune-link off" to remove'
# Every tracked lock file in code-studio: Cargo rewrites each one whose workspace is built
# (or merely opened by an editor) while the redirect is on.
locks=("src-tauri/Cargo.lock" "xtask/Cargo.lock")

# Native programs on Windows need D:/... paths, not Git Bash's /d/... form.
native() { if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi; }
backup_of() { printf '%s/studio-%s' "$dev" "${1//\//-}"; }
ours() { [ -f "$cargo_config" ] && [ "$(head -n 1 "$cargo_config")" = "$marker" ]; }

crate_of() {
  case "$1" in
    kit) echo "rune-kit https://github.com/shba007/rune-kit.git code-kit" ;;
    infer) echo "rune-infer https://github.com/shba007/rune-infer.git code-infer" ;;
    *) echo "Unknown target '$1'. Use kit and/or infer." >&2; exit 1 ;;
  esac
}

seed_store() {
  [ -d "$store" ] && return 0
  mkdir -p "$store"
  if [ -d "$HOME/.rune/tools" ]; then
    # Start from what is installed, so only the plugins you rebuild differ.
    cp -R "$HOME/.rune/tools/." "$store/"
    echo "Seeded the dev store from $HOME/.rune/tools"
  fi
}

kit() {
  seed_store
  RUNE_KIT_DATA_DIR="$(native "$store")" \
    cargo run --quiet --manifest-path "$(native "$root/code-kit/Cargo.toml")" --bin rune-kit -- "$@"
}

cmd_on() {
  [ $# -gt 0 ] || set -- kit
  if [ -f "$cargo_config" ] && ! ours; then
    echo "$cargo_config exists and was not written by this script. Move it away first." >&2
    exit 1
  fi
  local body="$marker"$'\n'
  for target in $(printf '%s\n' "$@" | sort -u); do
    read -r name url dir <<<"$(crate_of "$target")"
    [ -d "$root/$dir" ] || { echo "No $dir beside code-studio." >&2; exit 1; }
    body+=$'\n'"[patch.\"$url\"]"$'\n'"$name = { path = \"$(native "$root/$dir")\" }"$'\n'
  done
  mkdir -p "$dev" "$(dirname "$cargo_config")"
  for lock in "${locks[@]}"; do
    [ -f "$(backup_of "$lock")" ] || cp "$studio/$lock" "$(backup_of "$lock")"
  done
  printf '%s' "$body" >"$cargo_config"
  echo "Linked: $(printf '%s\n' "$@" | sort -u | paste -sd, - | sed 's/,/, /g'). Studio now builds against the local folder(s)."
  echo 'The Cargo.lock files in code-studio will change while linked. Do not commit them; "rune-link off" restores them.'
}

cmd_off() {
  if [ -f "$cargo_config" ]; then
    ours || { echo "$cargo_config was not written by this script; leaving it." >&2; exit 1; }
    rm "$cargo_config"
  fi
  for lock in "${locks[@]}"; do
    if [ -f "$(backup_of "$lock")" ]; then
      cp "$(backup_of "$lock")" "$studio/$lock"
      rm "$(backup_of "$lock")"
      echo "Restored $lock"
    fi
    # An editor can rewrite a lock file before the backup is taken, leaving a
    # leftover "unused patch" entry. Strip it so the file matches the repo again.
    node -e '
      const fs = require("fs"), file = process.argv[1], text = fs.readFileSync(file, "utf8")
      const clean = text.replace(/\r?\n\[\[patch\.unused\]\]\r?\nname = "rune-(kit|infer)"\r?\nversion = "[^"]*"\r?\n/g, "")
      if (clean !== text) fs.writeFileSync(file, clean)
    ' "$(native "$studio/$lock")"
  done
  echo 'Unlinked. Studio builds against its pinned Kit and Infer again.'
}

cmd_plugin() {
  [ $# -gt 0 ] || { echo 'Usage: rune-link plugin <name>   (for example rune-time)' >&2; exit 1; }
  local name="$1" source="$root/code-tools/mcps/$1"
  [ -f "$source/plugin.toml" ] || { echo "No plugin at $source" >&2; exit 1; }
  CARGO_HUSKY_DONT_INSTALL_HOOKS=true cargo build \
    --manifest-path "$(native "$root/code-tools/Cargo.toml")" -p "$name" --release --target wasm32-wasip1
  local built="$root/code-tools/target/wasm32-wasip1/release/${name//-/_}.wasm"
  [ -f "$built" ] || { echo "Build produced no $built (is this a WASM plugin?)" >&2; exit 1; }
  local stage="$dev/stage/$name"
  rm -rf "$stage"
  mkdir -p "$stage"
  cp "$built" "$stage/$name.wasm"
  cp "$source/plugin.toml" "$stage/"
  kit uninstall "$name" >/dev/null 2>&1 || true
  kit install "$(native "$stage/$name.wasm")"
  echo "Installed the local build of $name into the dev store ($store)."
}

cmd_status() {
  local linked=""
  if ours; then
    grep -q 'rune-infer.git' "$cargo_config" && linked="infer"
    grep -q 'rune-kit.git' "$cargo_config" && linked="${linked:+$linked, }kit"
  fi
  echo "Cargo redirect : ${linked:-off (Studio uses its pinned versions)}"
  if [ -d "$store" ]; then echo "Dev store      : $store"; else echo 'Dev store      : not created yet'; fi
  if [ -f "$store/installed.json" ]; then
    node -e '
      const mcps = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).mcps ?? {}
      for (const [name, entry] of Object.entries(mcps))
        console.log("  " + name.padEnd(22) + " " + String(entry.version).padEnd(8) + " " + entry.source)
    ' "$(native "$store/installed.json")"
  fi
  if [ -f "$(backup_of "${locks[0]}")" ]; then
    echo 'Cargo.lock     : changed by linking; do not commit it. "rune-link off" restores it.'
  fi
}

command="${1:-status}"
[ $# -gt 0 ] && shift
case "$command" in
  on) cmd_on "$@" ;;
  off) cmd_off ;;
  plugin) cmd_plugin "$@" ;;
  kit) kit "$@" ;;
  studio)
    seed_store
    cd "$studio"
    RUNE_KIT_DATA_DIR="$(native "$store")" bun run tauri:dev
    ;;
  status) cmd_status ;;
  reset)
    cmd_off
    if [ -d "$dev" ]; then rm -rf "$dev"; echo 'Deleted the dev store.'; fi
    ;;
  *) sed -n '2,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; echo "Unknown command '$command'" >&2; exit 1 ;;
esac
