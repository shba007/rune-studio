use semver::Version;
use std::fs;
use studio_xtask::{
    canonical_notes, release_notes, sync_manifests, validate_event, validate_versions,
};
fn v(s: &str) -> Version {
    Version::parse(s).unwrap()
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("src-tauri")).unwrap();
    fs::write(
        root.join("package.json"),
        "{\n  \"name\": \"rune-studio\",\n  \"version\": \"0.0.27\"\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("src-tauri/Cargo.toml"),
        "[package]\nname = \"rune-studio\"\nversion = \"0.0.26\"\n",
    )
    .unwrap();
    fs::write(root.join("src-tauri/Cargo.lock"),"version = 4\n[[package]]\nname = \"rune-studio\"\nversion = \"0.0.26\"\n[[package]]\nname = \"dependency\"\nversion = \"1.0.0\"\n").unwrap();
    fs::write(
        root.join("src-tauri/tauri.conf.json"),
        "{\"version\":\"../package.json\"}",
    )
    .unwrap();
    fs::write(root.join("CHANGELOG.md"),"# Changelog\n\n## v0.0.27\n\n[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.26...v0.0.27)\n\n### 🚀 Enhancements\n\n- Explain a selected window ([abcdef1](https://github.com/shba007/rune-studio/commit/abcdef1))\n\n### ❤️ Contributors\n\n- Author ([@author](https://github.com/author))\n\n## v0.0.26\n\n- Historical Cycle 2 notes.\n").unwrap();
    temp
}
#[test]
fn observed_release_version_mismatch_fails_before_body_validation() {
    let root = fixture();
    assert!(
        validate_versions(root.path())
            .unwrap_err()
            .to_string()
            .contains("versions must agree")
    );
}
#[test]
fn sync_is_small_idempotent_and_never_edits_changelog() {
    let root = fixture();
    let original = fs::read(root.path().join("CHANGELOG.md")).unwrap();
    sync_manifests(root.path(), &v("0.0.27")).unwrap();
    sync_manifests(root.path(), &v("0.0.27")).unwrap();
    validate_versions(root.path()).unwrap();
    assert_eq!(
        fs::read(root.path().join("CHANGELOG.md")).unwrap(),
        original
    );
    assert!(
        fs::read_to_string(root.path().join("src-tauri/Cargo.lock"))
            .unwrap()
            .contains("name = \"dependency\"\nversion = \"1.0.0\"")
    );
}
#[test]
fn publisher_and_every_artifact_platform_use_exact_prepared_notes() {
    let root = fixture();
    sync_manifests(root.path(), &v("0.0.27")).unwrap();
    let notes = release_notes(
        &fs::read_to_string(root.path().join("CHANGELOG.md")).unwrap(),
        &v("0.0.27"),
    )
    .unwrap();
    for _platform in ["windows", "linux", "macos-aarch64", "macos-x64"] {
        assert!(validate_event(root.path(),&serde_json::json!({"release":{"tag_name":"v0.0.27","body":notes.replace('\n',"\r\n")}})).is_ok());
    }
    for changed in [
        notes.replace("([@author](https://github.com/author))", "(@author)"),
        notes.replace("Explain a selected window", "Different unrelated notes"),
        format!("{} ", notes.trim_end()),
    ] {
        assert!(
            validate_event(
                root.path(),
                &serde_json::json!({"release":{"tag_name":"v0.0.27","body":changed}})
            )
            .is_err()
        );
    }
    assert!(
        validate_event(
            root.path(),
            &serde_json::json!({"release":{"tag_name":"v0.0.26","body":notes}})
        )
        .is_err()
    );
}
#[test]
fn empty_missing_duplicate_and_placeholder_notes_fail() {
    for notes in [
        "## v0.0.26\n- Old.",
        "## v0.0.27\n",
        "## v0.0.27\n### Fixes\n",
        "## v0.0.27\n- TODO\n",
        "## v0.0.27\n- Cycle 4 changes\n",
        "## v0.0.27\n- Good.\n## v0.0.27\n- Duplicate.",
    ] {
        assert!(release_notes(notes, &v("0.0.27")).is_err());
    }
}
#[test]
fn normalization_does_not_rewrite_markdown_or_spaces() {
    assert_eq!(canonical_notes("\r\n- Exact.\r\n"), "- Exact.");
    assert_ne!(canonical_notes("- Exact. "), canonical_notes("- Exact."));
    assert_eq!(
        canonical_notes("([@user](https://github.com/user))"),
        "([@user](https://github.com/user))"
    );
}
