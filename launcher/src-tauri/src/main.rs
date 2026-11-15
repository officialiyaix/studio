// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Elevated helper mode for the PC Optimiser: performs one machine-wide change and exits without opening a window.
    #[cfg(windows)]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--stabled-optimiser-elevated") {
            std::process::exit(stabled_lib::run_elevated_helper(&args[2..]));
        }
    }
    #[cfg(feature = "dev-tools")]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--dev") {
            std::process::exit(stabled_lib::devtools::main(&args[2..]));
        }
    }
    stabled_lib::run()
}
