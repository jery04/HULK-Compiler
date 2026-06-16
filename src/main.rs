//! Main entry point for the HULK compiler.
//!
//! This module orchestrates the full compilation pipeline:
//! 1. **Lexical analysis** – tokenizes the source code.
//! 2. **Syntactic analysis** – parses tokens into an AST ([`parser::Program`]).
//! 3. **Semantic analysis** – performs type checking and validation.
//! 4. **Code generation** – lowers the AST to C and compiles it to an executable.
//!
//! The compiler follows the contract defined by the Matcom course:
//! - Exits with code `1` for lexical errors.
//! - Exits with code `2` for syntactic errors.
//! - Exits with code `3` for semantic errors or backend failures.
//! - Exits with code `0` on successful compilation.
//!
//! # Usage
//! ```bash
//! hulk <file.hulk>
//! ```
//! The generated executable is written to `./output` (or `output.exe` on Windows).



#![allow(dead_code)]

mod lexer; // lexer module
mod parser; // parser module
mod evaluator; // evaluator module (frozen probe)
mod struct_printer; // structure printer module (dev)
mod semantic; // semantic checker module
mod codegen; // C backend

use std::process::{exit, Command};

use lexer::lexer::{Span, TokenStream};
use parser::Parser;

/// Render one diagnostic line to stderr in the contract format
/// `(line,col) TYPE: message`. Positions are 1-based (`Pos` already is).
fn emit_error(span: Span, ty: &str, message: &str) {
    let (line, col) = (span.start.line, span.start.col);
    eprintln!("({},{}) {}: {}", line, col, ty, message);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("(0,0) SYNTACTIC: usage: hulk <file.hulk>");
        exit(2);
    }
    let path = &args[1];

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("(0,0) SYNTACTIC: cannot read '{}': {}", path, e);
            exit(2);
        }
    };

    // ── Phase 1: lexical (highest priority) ──────────────────────────────────
    let (_tokens, lex_errors) = TokenStream::tokenize_all(&source);
    if !lex_errors.is_empty() {
        for e in &lex_errors {
            emit_error(e.span, "LEXICAL", &e.msg);
        }
        exit(1);
    }

    // ── Phase 2: syntactic ───────────────────────────────────────────────────
    let stream = TokenStream::new(&source);
    let mut parser = Parser::new(stream);
    let program = parser.parse_program();
    if !parser.errors.is_empty() || program.is_none() {
        if parser.errors.is_empty() {
            emit_error(Span::default(), "SYNTACTIC", "could not parse program");
        } else {
            for e in &parser.errors {
                emit_error(e.span, "SYNTACTIC", &e.message);
            }
        }
        exit(2);
    }
    let program = program.unwrap();

    // ── Phase 3: semantic ────────────────────────────────────────────────────
    let sem_errors = semantic::check_program(&program);
    if !sem_errors.is_empty() {
        for e in &sem_errors {
            emit_error(e.span, "SEMANTIC", &e.message);
        }
        exit(3);
    }

    // ── Success: lower to C and compile to ./output ──────────────────────────
    let c_src = codegen::emit_c(&program);
    if let Err(msg) = build_output(&c_src) {
        // A backend failure is reported as semantic-level (lowest priority) so the
        // contract still emits a typed diagnostic instead of crashing.
        eprintln!("(0,0) SEMANTIC: backend error: {}", msg);
        exit(3);
    }
    exit(0);
}

/// Write the generated C to `output.c` and compile it into `./output` with the
/// system C compiler. Tries common compiler front-ends in order.
fn build_output(c_src: &str) -> Result<(), String> {
    let c_path = "output.c";
    std::fs::write(c_path, c_src).map_err(|e| format!("writing {}: {}", c_path, e))?;

    let candidates = ["cc", "gcc", "clang"];
    let mut last_err = String::from("no C compiler found (tried cc, gcc, clang)");
    for cc in candidates {
        let result = Command::new(cc)
            .args([c_path, "-o", "output", "-lm", "-O2"])
            .output();
        match result {
            Ok(out) if out.status.success() => return Ok(()),
            Ok(out) => {
                last_err = format!(
                    "{} failed: {}",
                    cc,
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Err(_) => continue, // compiler not present, try next
        }
    }
    Err(last_err)
}
