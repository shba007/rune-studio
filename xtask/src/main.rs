use std::{env, fs, path::PathBuf};
use studio_xtask::{
    Result, apply, release_notes, repository_plan, validate_event, validate_versions, version,
};

fn run() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    match args.first().map(String::as_str) {
        Some("prepare") => {
            let write = args.iter().any(|s| s == "--apply");
            if args.iter().skip(1).any(|s| s != "--apply") { return Err("Usage: prepare [--apply] (default is a read-only dry run)".into()); }
            let plan = repository_plan(&root)?;
            if let Some(plan) = &plan {
                println!("Proposed version: {}\n\n{}", plan.version, plan.notes);
                if write { apply(&root, plan)?; validate_versions(&root)?; }
            } else { println!("No releasable changes; no release PR needed."); }
            if write && let Some(output) = env::var_os("GITHUB_OUTPUT") {
                use std::io::Write;
                writeln!(fs::OpenOptions::new().append(true).open(output)?, "prepared={}", plan.is_some())?;
            }
        }
        Some("notes") => {
            validate_versions(&root)?;
            let notes = release_notes(&fs::read_to_string(root.join("CHANGELOG.md"))?, &version(&root)?)?;
            if args.len() == 3 && args[1] == "--out" { fs::write(&args[2], notes)?; }
            else if args.len() == 1 { print!("{notes}"); }
            else { return Err("Usage: notes [--out FILE]".into()); }
        }
        Some("version") if args.len() == 1 => { validate_versions(&root)?; println!("{}", version(&root)?); }
        Some("verify-event") if args.len() == 1 => {
            let event: serde_json::Value = serde_json::from_slice(&fs::read(env::var_os("GITHUB_EVENT_PATH").ok_or("GITHUB_EVENT_PATH is required")?)?)?;
            validate_event(&root, &event)?;
        }
        Some("example") if args.len() == 1 => {
            let old = semver::Version::parse("0.0.23")?;
            let messages = vec!["fix(kit): report incompatible tool gateways clearly".into(), "ci(release): update version".into()];
            let plan = studio_xtask::plan(&old, &old, "# Changelog\n\n## v0.0.23\n\n- Previous functionality.\n", "example-only", &studio_xtask::entries(&messages))?.unwrap();
            println!("Dry-run example (no files or Git state changed):\n\n## v{}\n\n{}", plan.version, plan.notes);
        }
        _ => return Err("Usage: cargo run --manifest-path xtask/Cargo.toml -- {prepare [--apply]|notes [--out FILE]|version|verify-event|example}".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Release preparation failed: {error}");
        std::process::exit(1);
    }
}
