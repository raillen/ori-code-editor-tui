//! Binário `oride` — editor TUI (P0.2).

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use oride_core::DocumentStore;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("oride {VERSION}");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--demo") {
        return run_demo();
    }

    let mut path: Option<PathBuf> = None;
    let mut stat = false;
    let mut force_headless = false;

    for arg in &args {
        match arg.as_str() {
            "--stat" => stat = true,
            "--headless" => force_headless = true,
            other if other.starts_with('-') => {
                eprintln!("unknown argument: {other}");
                print_help();
                return ExitCode::from(2);
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    if stat {
        let Some(path) = path else {
            eprintln!("error: --stat requires a file path");
            return ExitCode::from(2);
        };
        return match print_file_stat(&path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err}");
                ExitCode::from(1)
            }
        };
    }

    if force_headless {
        eprintln!("--headless without --stat is a no-op; use TUI or --stat");
        return ExitCode::from(2);
    }

    // TUI: path opcional
    match oride_app::run(path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn print_help() {
    println!(
        "oride {VERSION} — TUI code editor\n\n\
         USAGE:\n\
           oride [file]           open file (or empty buffer)\n\
           oride --version\n\
           oride --demo           headless core smoke\n\
           oride <file> --stat    print line/byte stats\n\n\
         KEYS (P0.2):\n\
           Ctrl+S save · Ctrl+Z undo · Ctrl+Y redo\n\
           arrows / Home / End · Esc or Ctrl+Q quit"
    );
}

fn run_demo() -> ExitCode {
    let mut store = DocumentStore::new();
    let id = store.open_empty();
    let doc = store.get_mut(id).expect("doc just opened");
    if let Err(err) = doc.insert_text("module demo\n\nfn main() {\n  print(\"hi\")\n}\n") {
        eprintln!("demo insert failed: {err}");
        return ExitCode::from(1);
    }
    doc.commit_edit_group();
    let lines = doc.buffer().line_count();
    let bytes = doc.buffer().len_bytes();
    println!(
        "oride demo ok — {lines} lines, {bytes} bytes, dirty={}",
        doc.is_dirty()
    );
    println!("---");
    print!("{}", doc.buffer().as_string());
    ExitCode::SUCCESS
}

fn print_file_stat(path: &std::path::Path) -> Result<(), oride_core::DocumentError> {
    let mut store = DocumentStore::new();
    let id = store.open_path(path)?;
    let doc = store.get(id).expect("opened");
    println!(
        "{}: {} lines, {} bytes, dirty={}",
        doc.tab_title(),
        doc.buffer().line_count(),
        doc.buffer().len_bytes(),
        doc.is_dirty()
    );
    Ok(())
}
