//! The Ajisai Core command line.
//!
//! One binary, one execution path, no optimizer flags, no backend selection,
//! no audit subcommand. Packages ship their own hosts.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use ajisai_core::lint::{self, Severity};
use ajisai_core::{manifest, syntax, Interpreter};

const USAGE: &str = "\
ajisai — the Ajisai Core interpreter

USAGE:
    ajisai run <file>       run a source file and print the resulting flow
    ajisai eval <source>    run a source fragment and print the resulting flow
    ajisai lint <file>      report obvious contract inconsistencies
    ajisai fmt <file>       print the program in canonical form
    ajisai words            print the vocabulary manifest as JSON
    ajisai repl             read, evaluate, print

The flow is printed bottom first, one value per line.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("repl");
    let rest = &args[args.len().min(1)..];

    match (command, rest) {
        ("run", [path]) => run_source(&read(path)),
        ("eval", fragments) if !fragments.is_empty() => run_source(&fragments.join(" ")),
        ("lint", [path]) => run_lint(&read(path)),
        ("fmt", [path]) => run_fmt(&read(path)),
        ("words", []) => {
            print!("{}", manifest::vocabulary_json(&Interpreter::new()));
            ExitCode::SUCCESS
        }
        ("repl", []) => repl(),
        ("help" | "--help" | "-h", _) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("ajisai: {path}: {error}");
            std::process::exit(1);
        }
    }
}

fn run_source(source: &str) -> ExitCode {
    let mut interpreter = Interpreter::new();
    match interpreter.execute(source) {
        Ok(()) => {
            print_flow(&interpreter);
            ExitCode::SUCCESS
        }
        Err(error) => {
            print_flow(&interpreter);
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print_flow(interpreter: &Interpreter) {
    for value in interpreter.stack() {
        println!("{value}");
    }
}

fn run_lint(source: &str) -> ExitCode {
    let interpreter = Interpreter::new();
    match lint::lint(&interpreter, source) {
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
        Ok(findings) => {
            for finding in &findings {
                println!("{finding}");
            }
            // The lint reports; it does not certify. Saying "nothing obviously
            // wrong" is the strongest true statement available here, and the
            // wording is deliberate.
            if findings.is_empty() {
                println!("nothing obviously wrong (this is not a proof of success)");
                return ExitCode::SUCCESS;
            }
            if findings.iter().any(|f| f.severity == Severity::Error) {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}

fn run_fmt(source: &str) -> ExitCode {
    match syntax::parse(source) {
        Ok(program) => {
            println!("{}", syntax::render_program(&program));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn repl() -> ExitCode {
    let mut interpreter = Interpreter::new();
    let stdin = io::stdin();
    let mut out = io::stdout();
    loop {
        print!("ajisai> ");
        let _ = out.flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        }
        let trimmed = line.trim();
        if trimmed == "BYE" {
            break;
        }
        if let Err(error) = interpreter.execute(trimmed) {
            println!("error: {error}");
        }
        let flow: Vec<String> = interpreter
            .stack()
            .iter()
            .map(|value| value.to_string())
            .collect();
        println!("{}", flow.join(" "));
    }
    ExitCode::SUCCESS
}
