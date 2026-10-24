fn main() {
    if std::env::args()
        .nth(1)
        .is_some_and(|arg| matches!(arg.as_str(), "--version" | "-V"))
    {
        println!("rune-studio-cli {}", app_lib::VERSION);
        return;
    }
    std::process::exit(app_lib::cli_main());
}
