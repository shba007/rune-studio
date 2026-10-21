fn main() {
    println!("cargo:rerun-if-changed=../package.json");
    let package = std::fs::read_to_string("../package.json").unwrap_or_default();
    let version = package
        .lines()
        .find_map(|line| {
            line.trim().strip_prefix("\"version\":").map(|rest| {
                rest.trim()
                    .trim_matches(|c| c == '"' || c == ',')
                    .to_string()
            })
        })
        .filter(|version| !version.is_empty())
        .unwrap_or_else(|| "0.0.0".to_string());
    println!("cargo:rustc-env=RUNE_VERSION={version}");
    tauri_build::build()
}
