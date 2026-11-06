#!/usr/bin/env bash
set -euo pipefail

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *) echo 'This onboarding test requires Windows and Git Bash.' >&2; exit 1 ;;
esac

for command in bun cargo cygpath; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "Missing required command: $command" >&2
    exit 1
  fi
done

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd -- "$script_dir/.."

test_root="$(mktemp -d "${TMPDIR:-/tmp}/rune-onboarding.XXXXXXXX")"
mkdir -p "$test_root/profile" "$test_root/infer"
profile_native="$(cygpath -w "$test_root/profile")"
config_native="$(cygpath -w "$test_root/tauri-test.json")"
export RUNE_HOME
RUNE_HOME="$(cygpath -w "$test_root/infer")"
token="$(basename "$test_root" | tr -cd '[:alnum:]' | tr '[:upper:]' '[:lower:]')"
identifier="dev.shirsendu-bairagi.rune-studio.onboarding${token}"

bun -e 'const [file, root, identifier] = process.argv.slice(1); await Bun.write(file, JSON.stringify({ identifier, app: { appDirectoriesOverride: root } }, null, 2));' -- "$config_native" "$profile_native" "$identifier"

show_recordings="$(bun -e 'import { join } from "node:path"; try { const path = join(process.env.APPDATA || "", "dev.shirsendu-bairagi.rune-studio", "settings.json"); const settings = await Bun.file(path).json(); console.log(settings.demoMode === true ? "1" : "0"); } catch { console.log("0"); }')"
if [[ "$show_recordings" == '1' ]]; then
  export RUNE_STUDIO_ONBOARDING_TEST_SHOW_RECORDINGS=1
else
  unset RUNE_STUDIO_ONBOARDING_TEST_SHOW_RECORDINGS
fi

echo "Isolated test profile: $profile_native"
echo "Isolated Infer home: $RUNE_HOME"
echo "Test app identifier: $identifier"
if [[ "$show_recordings" == '1' ]]; then
  echo 'Screen-recording visibility is enabled, matching your normal Studio preference.'
else
  echo 'Screen-recording visibility is disabled for this test profile.'
fi
echo 'Close other Studio instances first to avoid conflicts with their local services and hotkeys.'
echo 'The test folder is kept after exit; model downloads can be large.'

if [[ "${1:-}" == '--dry-run' ]]; then
  exit 0
fi
if [[ $# -ne 0 ]]; then
  echo 'Usage: bash scripts/test-onboarding.sh [--dry-run]' >&2
  exit 2
fi

cargo build --locked --manifest-path src-tauri/Cargo.toml --bin rune-studio-cli --features agent-bridge
exec bun x --no-install tauri dev --features agent-bridge,embedded-infer --config "$config_native"
