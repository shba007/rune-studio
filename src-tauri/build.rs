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
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
        println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    }
    tauri_build::build()
}
