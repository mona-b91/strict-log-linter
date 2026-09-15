mod parser;
mod pretty;

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut lenient = false;
    let mut path: Option<String> = None;

    for arg in &args[1..] {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "-h" | "--help" => {
                print_usage();
                return;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown flag: {}", other);
                print_usage();
                process::exit(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("only one input file is supported");
                    process::exit(2);
                }
                path = Some(other.to_string());
            }
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            print_usage();
            process::exit(2);
        }
    };

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("cannot read {}: {}", path, err);
            process::exit(1);
        }
    };

    let mut entries = Vec::new();
    let mut had_error = false;

    for (idx, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match parser::parse_line(line, lenient) {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                had_error = true;
                eprintln!("line {}: {}", idx + 1, err);
            }
        }
    }

    if had_error && !lenient {
        process::exit(1);
    }

    print!("{}", pretty::pretty_print(&entries));

    if had_error {
        process::exit(1);
    }
}

fn print_usage() {
    eprintln!("usage: loglint [--lenient] <file>");
    eprintln!();
    eprintln!("validates a log file and prints it in a normalized, aligned form.");
    eprintln!("by default any malformed line is a hard error and nothing is printed.");
    eprintln!("pass --lenient to recover what can be recovered and keep going.");
}
