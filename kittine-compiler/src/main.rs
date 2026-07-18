mod ast;
mod codegen;
mod lexer;
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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { input, output } => match build(&input, output.as_deref()) {
            Ok(out_path) => {
                println!("kittine-compiler: wrote {}", out_path.display());
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
/// is reported as an error instead of recursing forever.
fn build(input: &Path, output: Option<&Path>) -> Result<PathBuf, String> {
    let mut compiled = HashSet::new();
    let mut stack = Vec::new();
    compile_recursive(input, output, &mut compiled, &mut stack)
}

fn compile_recursive(
    input: &Path,
    output: Option<&Path>,
    compiled: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
) -> Result<PathBuf, String> {
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
        return Ok(out_path);
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
        compile_recursive(&import_path, None, compiled, stack)?;
    }

    let rust_code = codegen::generate(&program);
    std::fs::write(&out_path, rust_code)
        .map_err(|e| format!("failed to write '{}': {e}", out_path.display()))?;

    stack.pop();
    compiled.insert(canonical);
    Ok(out_path)
}
