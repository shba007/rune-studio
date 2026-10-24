use std::{env, fs, path::PathBuf, process::Command};
use studio_xtask::{
    Result, release_notes, sync_manifests, validate_event, validate_versions, version,
};
fn run() -> Result<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    if let Some(at) = args.iter().position(|s| s == "--root") {
        if at + 1 >= args.len() {
            return Err("--root requires a path".into());
        }
        root = PathBuf::from(args.remove(at + 1)).canonicalize()?;
        args.remove(at);
    }
    match args.first().map(String::as_str) {
  Some("prepare"|"example")=>{let script=PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("scripts/release.mjs");let status=Command::new(if cfg!(windows){"bun.exe"}else{"bun"}).arg(script).args(&args).arg("--root").arg(&root).status()?;if !status.success(){return Err("Pinned changelogen integration failed".into());}}
  Some("sync") if args.len()==2=>sync_manifests(&root,&semver::Version::parse(&args[1])?)?,
  Some("extract") if args.len()==3=>print!("{}",release_notes(&fs::read_to_string(&args[1])?,&semver::Version::parse(&args[2])?)?),
  Some("notes")=>{validate_versions(&root)?;let notes=release_notes(&fs::read_to_string(root.join("CHANGELOG.md"))?,&version(&root)?)?;if args.len()==3 && args[1]=="--out"{fs::write(&args[2],notes)?;}else if args.len()==1{print!("{notes}");}else{return Err("Usage: notes [--out FILE]".into());}}
  Some("version") if args.len()==1=>{validate_versions(&root)?;println!("{}",version(&root)?);}
  Some("verify-event") if args.len()==1=>{let event=serde_json::from_slice(&fs::read(env::var_os("GITHUB_EVENT_PATH").ok_or("GITHUB_EVENT_PATH is required")?)?)?;validate_event(&root,&event)?;}
  _=>return Err("Usage: xtask {prepare [--apply]|example|sync VERSION|extract FILE VERSION|notes [--out FILE]|version|verify-event} [--root DIR]".into()),
 }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Release validation failed: {e}");
        std::process::exit(1);
    }
}
