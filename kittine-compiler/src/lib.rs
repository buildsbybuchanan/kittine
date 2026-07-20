pub mod ast;
#[cfg(not(target_arch = "wasm32"))]
pub mod build;
pub mod codegen;
pub mod fmt;
pub mod infer;
pub mod lexer;
pub mod lint;
pub mod lockfile;
pub mod manifest;
#[cfg(not(target_arch = "wasm32"))]
pub mod package;
pub mod parser;
#[cfg(not(target_arch = "wasm32"))]
pub mod registry;
#[cfg(test)]
mod tests;
#[cfg(target_arch = "wasm32")]
pub mod wasm_api;
