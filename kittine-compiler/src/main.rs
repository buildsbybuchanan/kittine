mod ast;
mod codegen;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;

use clap::{Parser as ClapParser, Subcommand};
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

fn build(input: &Path, output: Option<&Path>) -> Result<PathBuf, String> {
    let source = std::fs::read_to_string(input)
        .map_err(|e| format!("failed to read '{}': {e}", input.display()))?;

    let tokens = lexer::tokenize(&source).map_err(|e| e.to_string())?;
    let program = parser::parse(tokens).map_err(|e| e.to_string())?;
    let rust_code = codegen::generate(&program);

    let out_path = match output {
        Some(p) => p.to_path_buf(),
        None => input.with_extension("rs"),
    };
    std::fs::write(&out_path, rust_code)
        .map_err(|e| format!("failed to write '{}': {e}", out_path.display()))?;

    Ok(out_path)
}
