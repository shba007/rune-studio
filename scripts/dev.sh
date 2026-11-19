#!/usr/bin/env bash
set -uo pipefail

studio="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$(dirname "$studio")"
backup="$root/.dev"
store="$backup/kit-data"
config="$studio/.cargo/config.toml"
locks=("src-tauri/Cargo.lock")

native() { if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi; }
saved() { printf '%s/studio-%s' "$backup" "${1//\//-}"; }

unlink() {
  rm -f "$config"
  for lock in "${locks[@]}"; do
    if [ -f "$(saved "$lock")" ]; then
      cp "$(saved "$lock")" "$studio/$lock" && rm "$(saved "$lock")"
    fi
  done
  echo "Dev mode off: Studio uses its pinned Kit and Infer again."
}

link() {
  mkdir -p "$backup" "$(dirname "$config")"
  for lock in "${locks[@]}"; do
    [ -f "$(saved "$lock")" ] || cp "$studio/$lock" "$(saved "$lock")"
  done
  cat >"$config" <<EOF
[patch."https://github.com/shba007/rune-kit.git"]
rune-kit = { path = "$(native "$root/code-kit")" }

[patch."https://github.com/shba007/rune-infer.git"]
rune-infer = { path = "$(native "$root/code-infer")" }
EOF
}

kit() {
  RUNE_KIT_DATA_DIR="$(native "$store")" \
    cargo run --quiet --manifest-path "$(native "$root/code-kit/Cargo.toml")" --bin rune-kit -- "$@"
}

install_plugin() {
  local name="$1" source="$root/code-tools/mcps/$1"
  [ -f "$source/plugin.toml" ] || { echo "No plugin $name in code-tools; skipping." >&2; return 0; }
  CARGO_HUSKY_DONT_INSTALL_HOOKS=true cargo build \
    --manifest-path "$(native "$root/code-tools/Cargo.toml")" -p "$name" --release --target wasm32-wasip1 \
    || { echo "Could not build $name; skipping." >&2; return 0; }
  local wasm="$root/code-tools/target/wasm32-wasip1/release/${name//-/_}.wasm"
  [ -f "$wasm" ] || { echo "No $wasm was built; skipping $name." >&2; return 0; }
  local stage="$backup/stage/$name"
  rm -rf "$stage" && mkdir -p "$stage"
  cp "$wasm" "$stage/$name.wasm" && cp "$source/plugin.toml" "$stage/"
  kit uninstall "$name" >/dev/null 2>&1
  kit install "$(native "$stage/$name.wasm")" || echo "Could not install $name; skipping." >&2
}

run_studio() {
  cd "$studio" || exit 1
  cargo build --manifest-path src-tauri/Cargo.toml --bin rune-studio-cli --features agent-bridge \
    && bunx tauri dev --features agent-bridge,embedded-infer,embedded-kit
}

linked=true
for dir in code-kit code-infer code-tools; do
  [ -d "$root/$dir" ] || linked=false
done
if [ -f "$config" ] && ! grep -q 'rune-kit.git' "$config"; then
  echo "$config exists and is not from this script; running without the link." >&2
  linked=false
fi

if [ "$linked" = false ]; then
  run_studio
  exit $?
fi

trap unlink EXIT
trap 'exit 130' INT TERM
link
echo "Dev mode on: local Kit and Infer linked."

if [ ! -d "$store" ]; then
  mkdir -p "$store"
  [ -d "$HOME/.rune/tools" ] && cp -R "$HOME/.rune/tools/." "$store/"
fi
for plugin in ${RUNE_DEV_PLUGINS-rune-filesystem}; do
  install_plugin "$plugin"
done

RUNE_KIT_DATA_DIR="$(native "$store")" run_studio
