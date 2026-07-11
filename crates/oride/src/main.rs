//! Binário `oride` — scaffold Fase 0 (sem TUI ainda).
//!
//! Uso:
//!   oride --version
//!   oride --demo              # exercita oride-core em memória
//!   oride <arquivo> --stat    # abre e imprime stats (headless)

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use oride_core::DocumentStore;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
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
    for arg in &args {
        match arg.as_str() {
            "--stat" => stat = true,
            other if !other.starts_with('-') => path = Some(PathBuf::from(other)),
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    let Some(path) = path else {
        eprintln!("error: path required (TUI still WIP — use --demo or --stat <file>)");
        print_help();
        return ExitCode::from(2);
    };

    if !stat {
        eprintln!(
            "oride {VERSION}: TUI not implemented yet (Phase 0 scaffold).\n\
             Use: oride --stat {}   or   oride --demo",
            path.display()
        );
        return ExitCode::from(1);
    }

    match print_file_stat(&path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn print_help() {
    println!(
        "oride {VERSION} — TUI code editor (scaffold)\n\n\
         USAGE:\n\
           oride --version\n\
           oride --demo\n\
           oride <file> --stat\n\n\
         Phase 0: buffer/document core only. TUI lands in P0.2."
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
