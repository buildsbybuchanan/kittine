mod ast;
mod codegen;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;

use clap::{Parser as ClapParser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(ClapParser)]
#[command(
    name = "kittine-compiler",
    version,
    about = "Compiler for the Kittine (.kitty) language"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile a .kitty file into a Rust file targeting Leptos 0.7.
    Build {
        /// Path to the input .kitty file.
        input: PathBuf,
        /// Output .rs path. Defaults to the input file with a .rs extension.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { input, output } => match build(&input, output.as_deref()) {
            Ok((out_path, was_written)) => {
                if was_written {
                    println!("kittine-compiler: wrote {}", out_path.display());
                } else {
                    println!("kittine-compiler: {} is already up to date", out_path.display());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("kittine-compiler: error: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

/// Compiles `input`, and — for every `import { .. } from '<path>'` it
/// contains — recursively compiles that dependency too, into the sibling
/// `.rs` path `codegen::rs_path_for_import` expects (same relative path,
/// `.kitty` swapped for `.rs`), before writing `input`'s own output. Each
/// distinct file is compiled at most once per invocation; an import cycle
/// is reported as an error instead of recursing forever. The returned
/// `bool` is whether `input`'s own output file was actually (re)written, as
/// opposed to already being byte-identical and left untouched.
fn build(input: &Path, output: Option<&Path>) -> Result<(PathBuf, bool), String> {
    let mut signatures = HashMap::new();
    collect_all_signatures(input, &mut HashSet::new(), &mut signatures)?;
    let mut compiled = HashSet::new();
    let mut stack = Vec::new();
    compile_recursive(input, output, &mut compiled, &mut stack, &signatures)
}

/// Recursively parses (lex + parse only, no codegen) every `.kitty` file
/// reachable from `input` through `import`s, merging each file's `purr`
/// signatures into one whole-graph map — this is what lets a string
/// literal passed to an *imported* `purr` get the same `Word`-parameter
/// `.to_string()` treatment a same-file call already had (see
/// `codegen::collect_function_signatures`). Every file gets parsed twice
/// across a full build (once here, once in `compile_recursive`) — real,
/// but cheap: parsing a `.kitty` file is milliseconds, nowhere near the
/// cost `cargo`/`wasm-bindgen` add downstream, so doing it twice to keep
/// this pass simple (no restructuring `compile_recursive` into a more
/// complex two-phase walk) is a good trade.
///
/// A cycle here just means "stop descending, this file's already been
/// visited" — real cycle *detection* (erroring out) is `compile_recursive`'s
/// job, on the pass that actually matters for output.
fn collect_all_signatures(
    input: &Path,
    visited: &mut HashSet<PathBuf>,
    signatures: &mut HashMap<String, Vec<String>>,
) -> Result<(), String> {
    let canonical = input
        .canonicalize()
        .map_err(|e| format!("failed to resolve '{}': {e}", input.display()))?;
    if !visited.insert(canonical) {
        return Ok(());
    }

    let source = std::fs::read_to_string(input)
        .map_err(|e| format!("failed to read '{}': {e}", input.display()))?;
    let tokens = lexer::tokenize(&source).map_err(|e| e.to_string())?;
    let program = parser::parse(tokens).map_err(|e| e.to_string())?;
    signatures.extend(codegen::collect_function_signatures(&program.items));

    let base_dir = input.parent().unwrap_or_else(|| Path::new("."));
    for import in &program.imports {
        let import_path = base_dir.join(&import.path);
        if import_path.exists() {
            collect_all_signatures(&import_path, visited, signatures)?;
        }
        // A missing import is reported properly by `compile_recursive`;
        // this pass just skips it rather than duplicating that error.
    }
    Ok(())
}

fn compile_recursive(
    input: &Path,
    output: Option<&Path>,
    compiled: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    signatures: &HashMap<String, Vec<String>>,
) -> Result<(PathBuf, bool), String> {
    let canonical = input
        .canonicalize()
        .map_err(|e| format!("failed to resolve '{}': {e}", input.display()))?;

    if stack.contains(&canonical) {
        let cycle = stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(format!(
            "import cycle detected: {cycle} -> {}",
            input.display()
        ));
    }

    let out_path = match output {
        Some(p) => p.to_path_buf(),
        None => input.with_extension("rs"),
    };

    if compiled.contains(&canonical) {
        return Ok((out_path, false));
    }

    stack.push(canonical.clone());

    let source = std::fs::read_to_string(input)
        .map_err(|e| format!("failed to read '{}': {e}", input.display()))?;
    let tokens = lexer::tokenize(&source).map_err(|e| e.to_string())?;
    let program = parser::parse(tokens).map_err(|e| e.to_string())?;

    let base_dir = input.parent().unwrap_or_else(|| Path::new("."));
    for import in &program.imports {
        let import_path = base_dir.join(&import.path);
        if !import_path.exists() {
            return Err(format!(
                "'{}' imports '{}', but '{}' does not exist",
                input.display(),
                import.path,
                import_path.display()
            ));
        }
        compile_recursive(&import_path, None, compiled, stack, signatures)?;
    }

    let rust_code = codegen::generate(&program, signatures);
    // Only touch the output file's mtime if its content actually changed.
    // `kittine-compiler build` recompiles the whole reachable import graph
    // on every invocation (simple, always-correct), but downstream tools
    // (cargo, wasm-bindgen, Vite's own file watcher) decide whether *they*
    // need to redo work by looking at file mtimes — rewriting every `.rs`
    // file unconditionally, even byte-for-byte identical ones, made every
    // reachable dependency look freshly modified on every single build and
    // defeated that caching entirely.
    let already_up_to_date = std::fs::read_to_string(&out_path)
        .is_ok_and(|existing| existing == rust_code);
    if !already_up_to_date {
        std::fs::write(&out_path, rust_code)
            .map_err(|e| format!("failed to write '{}': {e}", out_path.display()))?;
    }

    stack.pop();
    compiled.insert(canonical);
    Ok((out_path, !already_up_to_date))
}
