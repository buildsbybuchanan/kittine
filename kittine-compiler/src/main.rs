mod ast;
mod codegen;
mod fmt;
mod infer;
mod lexer;
mod lint;
mod parser;
#[cfg(test)]
mod tests;

use clap::{Parser as ClapParser, Subcommand};
use std::collections::HashSet;
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
    /// Reformat .kitty source into a canonical style.
    ///
    /// Every reformat is self-verified: the output is reparsed and its AST
    /// compared against the original before anything is written (see
    /// `fmt::format_and_verify`), so a formatter bug fails loudly instead of
    /// silently corrupting a file. A file containing `//` comments is
    /// skipped by default -- the lexer discards them before parsing, so
    /// there's nothing here to preserve them -- pass `--force` to reformat
    /// it anyway and accept losing them.
    Fmt {
        /// A .kitty file, or a directory to format every .kitty file under.
        path: PathBuf,
        /// Check formatting without writing; exits non-zero if anything
        /// would change.
        #[arg(long)]
        check: bool,
        /// Reformat files containing `//` comments too, losing them.
        #[arg(long)]
        force: bool,
    },
    /// Lint .kitty source for genuine issues: unused imports/private
    /// items/params/`hold` bindings, and duplicate field/variant/method/
    /// param names that would fail once generated to Rust.
    Lint {
        /// A .kitty file, or a directory to lint every .kitty file under.
        path: PathBuf,
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
        Command::Fmt { path, check, force } => cmd_fmt(&path, check, force),
        Command::Lint { path } => cmd_lint(&path),
    }
}

/// Recursively collects every `.kitty` file under `path` (or just `path`
/// itself, if it's a file), skipping `target`/`node_modules`/dotfile
/// directories.
fn collect_kitty_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(out);
    }
    walk_dir(path, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read '{}': {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read '{}': {e}", dir.display()))?;
        let p = entry.path();
        if p.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "target" || name == "node_modules" || name.starts_with('.') {
                continue;
            }
            walk_dir(&p, out)?;
        } else if p.extension().is_some_and(|e| e == "kitty") {
            out.push(p);
        }
    }
    Ok(())
}

fn cmd_fmt(path: &Path, check: bool, force: bool) -> ExitCode {
    let files = match collect_kitty_files(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("kittine-compiler: error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if files.is_empty() {
        eprintln!("kittine-compiler: no .kitty files found at '{}'", path.display());
        return ExitCode::FAILURE;
    }

    let mut any_changed = false;
    let mut any_error = false;
    for f in &files {
        let source = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("kittine-compiler: error: failed to read '{}': {e}", f.display());
                any_error = true;
                continue;
            }
        };
        if fmt::has_line_comments(&source) && !force {
            eprintln!(
                "kittine-compiler: skipping '{}': contains '//' comments, which fmt \
                 cannot preserve (the lexer discards them before parsing) -- pass \
                 --force to format anyway and lose them",
                f.display()
            );
            any_error = true;
            continue;
        }
        let formatted = match fmt::format_and_verify(&source) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("kittine-compiler: error formatting '{}': {e}", f.display());
                any_error = true;
                continue;
            }
        };
        if formatted == source {
            continue;
        }
        any_changed = true;
        if check {
            println!("kittine-compiler: '{}' is not formatted", f.display());
        } else if let Err(e) = std::fs::write(f, &formatted) {
            eprintln!("kittine-compiler: error: failed to write '{}': {e}", f.display());
            any_error = true;
        } else {
            println!("kittine-compiler: formatted '{}'", f.display());
        }
    }

    if any_error {
        ExitCode::FAILURE
    } else if check && any_changed {
        ExitCode::FAILURE
    } else {
        if !any_changed {
            println!("kittine-compiler: {} file(s) already formatted", files.len());
        }
        ExitCode::SUCCESS
    }
}

fn cmd_lint(path: &Path) -> ExitCode {
    let files = match collect_kitty_files(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("kittine-compiler: error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if files.is_empty() {
        eprintln!("kittine-compiler: no .kitty files found at '{}'", path.display());
        return ExitCode::FAILURE;
    }

    let mut total_warnings = 0usize;
    let mut any_error = false;
    for f in &files {
        let source = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("kittine-compiler: error: failed to read '{}': {e}", f.display());
                any_error = true;
                continue;
            }
        };
        let tokens = match lexer::tokenize(&source) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("kittine-compiler: error: {}: {e}", f.display());
                any_error = true;
                continue;
            }
        };
        let program = match parser::parse(tokens) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("kittine-compiler: error: {}: {e}", f.display());
                any_error = true;
                continue;
            }
        };
        for w in lint::check(&program) {
            println!("kittine-compiler: {}: {w}", f.display());
            total_warnings += 1;
        }
    }

    if any_error {
        ExitCode::FAILURE
    } else if total_warnings > 0 {
        println!("kittine-compiler: {total_warnings} warning(s)");
        ExitCode::FAILURE
    } else {
        println!("kittine-compiler: no lint warnings across {} file(s)", files.len());
        ExitCode::SUCCESS
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
    let mut signatures = codegen::Signatures::default();
    collect_all_signatures(input, &mut HashSet::new(), &mut signatures)?;
    let mut compiled = HashSet::new();
    let mut stack = Vec::new();
    compile_recursive(input, output, &mut compiled, &mut stack, &signatures)
}

/// Recursively parses (lex + parse only, no codegen) every `.kitty` file
/// reachable from `input` through `import`s, merging each file's
/// `purr`/`litter`/`breed` signatures into one whole-graph map — this is
/// what lets a string literal passed to an *imported* `purr` get the same
/// `Word`-parameter `.to_string()` treatment a same-file call already had
/// (see `codegen::collect_signatures`). Every file gets parsed twice
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
    signatures: &mut codegen::Signatures,
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
    signatures.merge(codegen::collect_signatures(&program.items));

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
    signatures: &codegen::Signatures,
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
