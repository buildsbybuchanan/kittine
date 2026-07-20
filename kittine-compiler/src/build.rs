//! The `kittine-compiler build` orchestration: recursively resolving and
//! compiling a `.kitty` file's whole `import` graph. Lives in the library
//! (not `main.rs`) so `tests.rs`'s real end-to-end recursive-build tests
//! can call it directly, the same way `codegen`/`lexer`/`parser` already do.

use crate::{codegen, lexer, parser};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Compiles `input`, and — for every `import { .. } from '<path>'` it
/// contains — recursively compiles that dependency too, into the sibling
/// `.rs` path `codegen::rs_path_for_import` expects (same relative path,
/// `.kitty` swapped for `.rs`), before writing `input`'s own output. Each
/// distinct file is compiled at most once per invocation; an import cycle
/// is reported as an error instead of recursing forever. The returned
/// `bool` is whether `input`'s own output file was actually (re)written, as
/// opposed to already being byte-identical and left untouched.
pub fn build(input: &Path, output: Option<&Path>) -> Result<(PathBuf, bool), String> {
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
        if let Some(import_path) = resolve_import_path(base_dir, &import.path) {
            collect_all_signatures(&import_path, visited, signatures)?;
        }
        // A missing import is reported properly by `compile_recursive`;
        // this pass just skips it rather than duplicating that error.
    }
    Ok(())
}

/// Resolves an `import`'s `path` string against the file that declared
/// it. A path containing `/` or ending in `.kitty` is a relative file
/// path (`./components.kitty`, `../shared/util.kitty`) and resolves
/// exactly as it always has: joined onto the importing file's own
/// directory. A bare name with neither (`'kittine-http'`) is a *package*
/// import instead: resolved by walking upward from the importing file's
/// directory looking for `kitten_modules/<name>/lib.kitty` -- the same
/// upward-search `node_modules` resolution uses, so an import behaves the
/// same from any file in the project, not only ones sitting right next
/// to `kitten_modules`.
pub fn resolve_import_path(base_dir: &Path, import_path: &str) -> Option<PathBuf> {
    let is_relative_file = import_path.contains('/') || import_path.ends_with(".kitty");
    if is_relative_file {
        let direct = base_dir.join(import_path);
        return direct.exists().then_some(direct);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut dir = Some(base_dir);
        while let Some(d) = dir {
            let candidate = d
                .join(crate::package::MODULES_DIR)
                .join(import_path)
                .join(crate::package::ENTRY_FILE);
            if candidate.exists() {
                return Some(candidate);
            }
            dir = d.parent();
        }
    }
    None
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
    let mut program = parser::parse(tokens).map_err(|e| e.to_string())?;

    let base_dir = input.parent().unwrap_or_else(|| Path::new("."));
    for import in &mut program.imports {
        let import_path = resolve_import_path(base_dir, &import.path).ok_or_else(|| {
            format!(
                "'{}' imports '{}', but it could not be resolved (checked as a relative \
                 path, and as a package under '{}/{}/{}')",
                input.display(),
                import.path,
                base_dir.display(),
                crate::package::MODULES_DIR,
                import.path
            )
        })?;
        // A bare package specifier (`'kittine-strings'`) resolves to
        // `kitten_modules/<name>/lib.kitty`, not a same-directory file --
        // rewrite it to that relative path *before* codegen sees it, so
        // the `#[path = ".."]` attribute it emits points at the sibling
        // `.rs` file actually written below (via `with_extension("rs")`
        // on this exact resolved path), not a nonexistent
        // `<name>.rs` next to the importer. A relative-file import
        // (`'./util.kitty'`) round-trips through this unchanged, since
        // `base_dir.join(import.path)` stripped of `base_dir` is just
        // `import.path` again.
        if let Ok(rel) = import_path.strip_prefix(base_dir) {
            import.path = rel.to_string_lossy().replace('\\', "/");
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
    let already_up_to_date =
        std::fs::read_to_string(&out_path).is_ok_and(|existing| existing == rust_code);
    if !already_up_to_date {
        std::fs::write(&out_path, rust_code)
            .map_err(|e| format!("failed to write '{}': {e}", out_path.display()))?;
    }

    stack.pop();
    compiled.insert(canonical);
    Ok((out_path, !already_up_to_date))
}
