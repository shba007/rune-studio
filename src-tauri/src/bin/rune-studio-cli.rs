//! The `rune-studio-cli` command. A console program, unlike the windowed app (`rune-studio`).

fn main() {
    std::process::exit(app_lib::cli_main());
}
