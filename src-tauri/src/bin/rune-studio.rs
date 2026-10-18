//! The `rune-studio` command. A console program, unlike the windowed app (`rune-studio-app`).

fn main() {
    std::process::exit(app_lib::cli_main());
}
