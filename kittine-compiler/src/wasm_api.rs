//! The browser-facing single-file compile path for the Kittine
//! playground: lex -> parse -> codegen, with no filesystem access at all
//! (no `import`s are resolved or compiled) -- which is exactly why this
//! one path was always buildable to WASM, unlike `kittine-compiler
//! build`'s recursive, filesystem-walking import graph. `import`
//! statements in the source still lex and parse fine; they're just
//! inert here, matching a playground's "one snippet" scope.

use wasm_bindgen::prelude::*;

/// Compiles a single `.kitty` source string into Rust targeting Leptos
/// 0.7, or returns a human-readable lex/parse error message on failure.
#[wasm_bindgen]
pub fn compile_kitty_single_file(source: &str) -> Result<String, JsValue> {
    let tokens =
        crate::lexer::tokenize(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let program = crate::parser::parse(tokens).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let signatures = crate::codegen::collect_signatures(&program.items);
    Ok(crate::codegen::generate(&program, &signatures))
}
