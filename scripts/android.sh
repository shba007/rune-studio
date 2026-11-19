#!/usr/bin/env bash
set -euo pipefail

studio="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
api=26 # minSdk: audio capture links libaaudio, which exists from Android 8
app=dev.shirsendu_bairagi.rune_studio.debug
native() { if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi; }

sdk="${ANDROID_HOME:-}"
for guess in "$LOCALAPPDATA/Android/Sdk" "$HOME/AppData/Local/Android/Sdk" "$HOME/Library/Android/sdk" "$HOME/Android/Sdk"; do
  [ -n "$sdk" ] && [ -d "$sdk" ] && break
  sdk="$guess"
done
[ -d "$sdk" ] || { echo "Android SDK not found. Set ANDROID_HOME." >&2; exit 1; }
ndk="${NDK_HOME:-$(ls -d "$sdk"/ndk/* 2>/dev/null | sort -V | tail -1)}"
[ -d "$ndk" ] || { echo "No NDK under $sdk/ndk. Install one in Android Studio." >&2; exit 1; }
bin="$(ls -d "$ndk"/toolchains/llvm/prebuilt/*/bin | head -1)"
ext=""; [ -f "$bin/aarch64-linux-android$api-clang.cmd" ] && ext=".cmd"

export ANDROID_HOME="$(native "$sdk")" NDK_HOME="$(native "$ndk")" ANDROID_NDK_HOME="$(native "$ndk")"
export CC_aarch64_linux_android="$(native "$bin")/aarch64-linux-android$api-clang$ext"
export AR_aarch64_linux_android="$(native "$bin")/llvm-ar$([ -f "$bin/llvm-ar.exe" ] && echo .exe)"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"

if command -v cygpath >/dev/null 2>&1 && ! command -v bun.bat >/dev/null 2>&1; then
  shim="$(mktemp -d)"
  printf '@echo off\r\n"%s" %%*\r\n' "$(native "$(command -v bun)")" >"$shim/bun.bat"
  export PATH="$shim:$PATH"
fi

cd "$studio"
bunx tauri android build --debug --apk --target aarch64

[ "${1:-}" = "--no-install" ] && exit 0
adb="$sdk/platform-tools/adb"; [ -x "$adb" ] || [ -x "$adb.exe" ] || adb="$(command -v adb || true)"
if [ -z "$adb" ] || ! "$adb" devices | grep -qw device; then
  echo "Built. No phone connected over adb, so nothing was installed."
  exit 0
fi
apk="$studio/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
"$adb" install -r "$(native "$apk")"
MSYS_NO_PATHCONV=1 "$adb" shell monkey -p "$app" -c android.intent.category.LAUNCHER 1 >/dev/null
echo "Installed and started $app."
