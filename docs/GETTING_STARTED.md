# Getting Started with Kittine

This walks through installing everything needed to write and run Kittine
(`.kitty`) programs, from a clean machine.

## 1. Prerequisites

| Tool | Why | Install |
|---|---|---|
| Rust (stable) | Compiles the generated Leptos code to WebAssembly | [rustup.rs](https://rustup.rs), or `winget install Rustlang.Rustup` on Windows |
| `wasm32-unknown-unknown` target | The WASM compile target Leptos CSR apps use | `rustup target add wasm32-unknown-unknown` |
| A C/C++ linker | Rust on Windows needs the MSVC linker to produce any binary at all (including WASM host tooling) | [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with the "Desktop development with C++" workload — or `winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools"` |
| `wasm-bindgen-cli` | Post-processes the compiled `.wasm` into browser-ready JS glue | `cargo install wasm-bindgen-cli --version <match your Cargo.lock>` (see below) |
| Node.js 18+ and npm | Runs the Vite dev server / build | [nodejs.org](https://nodejs.org) |
| [Kittine VS Code extension](../vscode-kittine) | Syntax highlighting for `.kitty` files | See [VSCODE_EXTENSION.md](VSCODE_EXTENSION.md) |

### Matching `wasm-bindgen-cli` to your lockfile

The `wasm-bindgen` crate version pulled in by `leptos` **must exactly match**
the `wasm-bindgen-cli` binary version, or `wasm-bindgen` will fail with a
schema-version mismatch. After your first `cargo build` in `example-app/`
(or your own crate), check the pinned version:

```sh
grep -A1 'name = "wasm-bindgen"' example-app/Cargo.lock
# version = "0.2.126"   <- install exactly this
cargo install wasm-bindgen-cli --version 0.2.126
```

## 2. Build the compiler

```sh
cd kittine-compiler
cargo build --release
```

This produces `kittine-compiler/target/release/kittine-compiler`, a
standalone CLI. Run `cargo test` here too — it exercises the full
lex → parse → codegen pipeline against every construct in the language spec.

## 3. Install JS dependencies

From the repo root (an npm workspace covering `vite-plugin-kittine` and
`example-app`):

```sh
npm install
npm run build:plugin
```

## 4. Run the example app

```sh
npm run dev
```

Open the printed `http://localhost:5173/` URL. The first load recompiles
the whole Rust/Leptos/WASM chain from scratch and is slow (tens of
seconds); after that, editing `example-app/src/App.kitty` and saving
triggers an incremental rebuild. You should see a "Clicks: 0" button that
increments on click, and `Welcome Admin` logged to the browser console.

For a production build:

```sh
npm run build
cd example-app && npm run preview
```

## 5. Starting your own Kittine project

The fastest path is to copy `example-app/` as a template:

```sh
cp -r example-app my-app
cd my-app
```

Then:

1. Rename the crate in `Cargo.toml` (`[package] name = "my-app"`).
2. Edit `src/App.kitty` with your own components (add more `func`s and
   reference the generated component names from `main.rs` as needed —
   see [LANGUAGE.md](LANGUAGE.md)).
3. `npm install && npm run dev`.

Every `.kitty` file needs to live inside a directory that has a `Cargo.toml`
above it somewhere (the Vite plugin walks up from the file to find the
crate root) — `example-app/` follows this by putting `App.kitty` next to
`main.rs` inside the crate.

## Troubleshooting

- **`error: linker 'link.exe' not found`** — the MSVC Build Tools aren't
  installed, or aren't the ones rustup is finding. Reinstall with the
  "Desktop development with C++" workload and restart your shell.
- **`wasm-bindgen` schema/version mismatch** — your installed
  `wasm-bindgen-cli` doesn't match the `wasm-bindgen` crate version in
  `Cargo.lock`. Re-run `cargo install wasm-bindgen-cli --version <exact>`.
- **Nothing renders / blank page** — open the browser devtools console
  first; Rust panics inside the WASM module show up there (via
  `console_error_panic_hook`), not in the terminal.
