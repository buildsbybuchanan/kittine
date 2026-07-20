use clap::{Parser as ClapParser, Subcommand};
use kittine_compiler::{fmt, lexer, lint, parser};
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
    /// Add a dependency to this directory's kittine.toml, creating one if
    /// it doesn't exist yet. Doesn't download anything -- run `install`
    /// afterward to actually fetch it.
    Add {
        /// Package name, as published to the registry.
        name: String,
        /// Exact version to pin. Defaults to the latest published version.
        #[arg(long)]
        version: Option<String>,
    },
    /// Resolve and download every dependency in kittine.toml into
    /// kitten_modules/, sha256-verifying each tarball against the
    /// registry index, and write the exact resolved set to kittine.lock.
    Install,
    /// Publish this directory as a package: packs it into a tarball and
    /// uploads it as a new version of kittine.toml's [package] name.
    /// Requires the `gh` CLI, authenticated with write access to the
    /// registry repo.
    Publish,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { input, output } => match kittine_compiler::build::build(&input, output.as_deref()) {
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
        Command::Add { name, version } => cmd_add(&name, version.as_deref()),
        Command::Install => cmd_install(),
        Command::Publish => cmd_publish(),
    }
}

fn cmd_add(name: &str, version: Option<&str>) -> ExitCode {
    let dir = std::env::current_dir().expect("failed to read current directory");
    match kittine_compiler::package::add(&dir, name, version) {
        Ok(resolved) => {
            println!("kittine-compiler: added '{name}' {resolved} to kittine.toml");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("kittine-compiler: error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_install() -> ExitCode {
    let dir = std::env::current_dir().expect("failed to read current directory");
    match kittine_compiler::package::install(&dir) {
        Ok(locked) => {
            println!(
                "kittine-compiler: installed {} package(s) into {}/",
                locked.len(),
                kittine_compiler::package::MODULES_DIR
            );
            for pkg in &locked {
                println!("  {} {}", pkg.name, pkg.version);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("kittine-compiler: error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_publish() -> ExitCode {
    let dir = std::env::current_dir().expect("failed to read current directory");
    match kittine_compiler::package::publish(&dir) {
        Ok(url) => {
            println!("kittine-compiler: published; tarball at {url}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("kittine-compiler: error: {e}");
            ExitCode::FAILURE
        }
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

