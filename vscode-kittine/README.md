# Kittine for VS Code

Syntax highlighting and file-type support for [Kittine](https://github.com/siv-the-programmer/BuildsByBuchanan/tree/main/Valquroz_holding/kittine) (`.kitty`), a small language that compiles to [Leptos 0.7](https://leptos.dev) Rust and runs in the browser via WebAssembly.

![Kittine](images/kittine-icon.png)

## Features

- Syntax highlighting for every Kittine-specific construct:
  - Variable brackets: `<{name}>`
  - The assign/equality operator: `>>`
  - Control flow: `if>`, `orif>`, `else>`
  - Logging: `craft<...>`
  - Both string forms: `¨...¨` and `"..."`
  - The embedded JSX-like `return ( ... )` view syntax
- A distinct file icon for `.kitty` files (works alongside whatever icon theme you already have — no need to switch themes).
- Bracket matching and auto-closing for `<{ }>`, `¨ ¨`, `( )`, `{ }`.

## Getting started

Open any `.kitty` file and the `Kittine` language mode activates automatically. See the [language reference](../docs/LANGUAGE.md) in the main Kittine project for the full syntax spec.

## Requirements

This extension only provides editor syntax highlighting — it does not compile Kittine itself. To build `.kitty` files you need the `kittine-compiler` CLI (or the Vite plugin), both in the parent [`kittine`](..) project.

## Author

Kittine was created by **Sivario Buchanan**. Licensed under [MIT](LICENSE).
