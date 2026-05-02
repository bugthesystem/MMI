mod cleaner;
mod cli;
mod history;
mod hook;
mod patterns;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("mmi: {e:#}");
        std::process::exit(1);
    }
}
