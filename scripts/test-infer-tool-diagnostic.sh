#!/usr/bin/env bash
# Runs Studio against the sibling code-infer checkout with Infer's tool diagnostics
# written to a temporary file, then restores Cargo's configuration and lock file.
#
#   bash scripts/test-infer-tool-diagnostic.sh [--prepare-only]
set -uo pipefail

prepare_only=false
[ "${1:-}" = "--prepare-only" ] && prepare_only=true

native() { if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi; }
hash_of() { sha256sum "$1" | cut -d' ' -f1; }

studio_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
infer_root="$(cd "$studio_root/.." && pwd)/code-infer"
cargo_dir="$studio_root/.cargo"
cargo_config="$cargo_dir/config.toml"
lock_file="$studio_root/src-tauri/Cargo.lock"

if [ ! -f "$infer_root/Cargo.toml" ]; then
  echo "Rune Infer checkout not found at $infer_root" >&2
  exit 1
fi
if [ -e "$cargo_config" ]; then
  echo "Refusing to overwrite existing Cargo configuration: $cargo_config" >&2
  exit 1
fi

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/rune-tool-diagnostic-XXXXXXXX")"
backup_lock="$temp_dir/Cargo.lock.original"
diagnostic_file="$temp_dir/tool-diagnostic.jsonl"
mkdir -p "$cargo_dir"
cp "$lock_file" "$backup_lock"

config="[patch.'https://github.com/shba007/rune-infer.git']
rune-infer = { path = \"$(native "$infer_root")\" }
"
generated_lock_hash=""

cleanup() {
  if [ -f "$cargo_config" ]; then
    if [ "$(cat "$cargo_config"; printf x)" = "${config}x" ]; then
      rm "$cargo_config"
    else
      echo "warning: Cargo configuration changed during the test; left it at $cargo_config" >&2
    fi
  fi
  if [ -n "$generated_lock_hash" ] && [ -f "$lock_file" ]; then
    if [ "$(hash_of "$lock_file")" = "$generated_lock_hash" ]; then
      cp "$backup_lock" "$lock_file"
    else
      echo "warning: Cargo.lock changed during the test; original backup is at $backup_lock" >&2
    fi
  elif [ -f "$backup_lock" ]; then
    echo "warning: Cargo.lock was not restored automatically; original backup is at $backup_lock" >&2
  fi
  echo "Diagnostic metadata: $diagnostic_file"
  if [ -f "$diagnostic_file" ]; then
    echo 'Copy the following diagnostic lines:'
    cat "$diagnostic_file"
  else
    echo 'No diagnostic lines were recorded.'
  fi
}
trap cleanup EXIT

# Fail if the file appeared since the check above, as the original create-new did.
set -o noclobber
printf '%s' "$config" >"$cargo_config" || exit 1
set +o noclobber

cd "$studio_root" || exit 1
if ! cargo update --offline --manifest-path src-tauri/Cargo.toml -p rune-infer; then
  echo 'Could not prepare the temporary local Infer dependency.' >&2
  exit 1
fi
generated_lock_hash="$(hash_of "$lock_file")"
echo "Diagnostic metadata will be saved to: $diagnostic_file"
echo 'Run the Notepad task, then close Studio to finish.'
if [ "$prepare_only" = false ]; then
  RUNE_INFER_TOOL_DIAGNOSTICS="$(native "$diagnostic_file")" bun run tauri:dev
fi
