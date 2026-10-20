//! The app's version has one source: `package.json` (tauri.conf.json points at it too, which is the
//! number Settings shows). Cargo.toml has no version of its own, so `CARGO_PKG_VERSION` read 0.0.0 in
//! the command line, the eval's env.json and the trace. This hands the real one to the code as
//! `RUNE_VERSION`.

fn main() {
    println!("cargo:rerun-if-changed=../package.json");
    let package = std::fs::read_to_string("../package.json").unwrap_or_default();
    let version = package
        .lines()
        .find_map(|line| line.trim().strip_prefix("\"version\":").map(|rest| rest.trim().trim_matches(|c| c == '"' || c == ',').to_string()))
        .filter(|version| !version.is_empty())
        .unwrap_or_else(|| "0.0.0".to_string());
    println!("cargo:rustc-env=RUNE_VERSION={version}");
    tauri_build::build()
}
