<p align="center">
  <img src="kittinelogo.png" alt="Kittine" width="160" />
</p>

# Kittine

*Created by [Sivario Buchanan](https://buildsbybuchanan.com).*

Kittine (`.kitty`) is a small custom language with its own variable/state
syntax and an embedded JSX-like view syntax, compiled to idiomatic
[Leptos 0.7](https://leptos.dev) Rust and run in the browser via
WebAssembly. It's deliberately not a from-scratch ecosystem: plain HTML
elements, and Leptos/Rust items like `leptos_router`'s components, are
first-class inside a `.kitty` file right alongside Kittine's own syntax —
lean on the real framework underneath instead of reinventing it.

Part of the [BuildsByBuchanan](https://buildsbybuchanan.com) code
portfolio.

📖 **[Full documentation](docs/)** — [language reference](docs/LANGUAGE.md) ·
[getting started](docs/GETTING_STARTED.md) · [CLI reference](docs/CLI.md) ·
[VS Code extension](docs/VSCODE_EXTENSION.md) ·
[architecture](docs/ARCHITECTURE.md) · [roadmap](docs/ROADMAP.md)

This repository is a monorepo with four parts:

- **`kittine-compiler/`** — a Rust CLI that lexes, parses, and lowers
  `.kitty` source into a Leptos 0.7 `.rs` file.
- **`vite-plugin-kittine/`** — a Vite plugin that runs `kittine-compiler` on
  `.kitty` files, builds the host Rust crate to `wasm32-unknown-unknown`,
  post-processes it with `wasm-bindgen`, and serves the result to the
  browser.
- **`example-app/`** — a real multi-page Leptos CSR app: `App.kitty` wraps
  `Home`/`About`/`NotFound` pages in a `leptos_router` `<Router>`, with
  client-side `<A>` navigation and a 404 fallback. `Home.kitty`
  demonstrates a signal-backed counter, an `if>`/`orif>`/`else>` block, a
  `spin` loop (both imperative and list-rendering forms), a `purr`
  function, and composed `Nav`/`Card` components — all wired up through
  the Vite plugin.
- **`vscode-kittine/`** — a VS Code extension providing syntax highlighting
  and a file icon for `.kitty` files. See
  [docs/VSCODE_EXTENSION.md](docs/VSCODE_EXTENSION.md) to install it.

## Language quick reference

```kitty
import { Nav } from './Nav.kitty'

purr doubled(<<Num>> n) <<Num>> {
    return (n * 2)
}

func Counter() {
    <{count}> >> <<Num>> 0
    <{username}> >> 'Admin'
    <{ready}> >> yes>

    if><{username}> >> 'Admin'
        craft<'Welcome Admin'>
    orif><{username}> >> "User"
        craft<"Welcome User">
    else>
        craft<'no output'>

    spin<{n}> in [1, 2, 3] }{
        craft<n>
    }{

    return (
        <div>
            <Nav active='home' />
            <button onClick={<{count}> >> count + 1}>
                "Clicks: "
                <{count}>
                " (doubled: "
                { doubled(count) }
                ")"
            </button>
        </div>
    )
}
```

- `<{name}> >> value` declares a signal the first time it's seen in a
  component, and mutates it (`set_name.update(..)`) every time after.
- `'...'` and `"..."` are fully interchangeable string literals.
- `yes>` / `no>` are boolean literals; `[a, b, c]` is an array literal.
- `<<Num>>` / `<<Word>>` / `<<Flag>>` (or `<<Num[]>>`/`<<Word[]>>`/`<<Flag[]>>`
  for an array of one) are type tags — optional on a value, mandatory on a
  prop or a `purr` return type — checked against literal values at compile
  time and erased in the generated Rust.
- `craft<expr>` logs via `leptos::logging::log!`.
- `if>` / `orif>` / `else>` are indentation-delimited (no braces), with
  `>>`/`<`/`<=`/`>`/`>=`/`!=` as comparisons — usable in conditions and
  generally anywhere an expression is (a `purr` return, `craft<...>`).
- `spin<{item}> in list }{ .. }{` loops over an array, its body fenced by
  `}{` — a closing brace immediately followed by an opening one. As a
  statement it's a plain imperative loop; inside `return ( ... )` it
  renders one element per item via a reactive Leptos `<For>`.
- `func Name(<<Type>> prop) { .. }` takes typed props; `<Name prop='x' />`
  composes it into another view (a capitalized JSX tag is a component
  reference, lowercase is a plain HTML element).
- `func Card(children) { .. { children() } .. }` — an untyped `children`
  param accepts nested JSX from wherever the component is composed:
  `<Card><p>"nested"</p></Card>`, no extra syntax needed at the call site.
- `purr name(<<Type>> param) <<Type>> { .. return (expr) }` is a plain
  function — computes and returns a value, renders no view — called like
  `name(arg)` anywhere an expression is valid.
- `import { Name } from './file.kitty'` brings another file's
  components/functions into scope; `kittine-compiler` resolves and
  compiles the whole import graph recursively.
- **Routing has no Kittine-specific syntax** — `leptos_router` is in scope
  everywhere, and `<Router>`/`<Routes>`/`<Route>`/`<A>` compose exactly
  like any other component: `<Route path={StaticSegment('about')}
  view={About}/>`. See [Routing](docs/LANGUAGE.md#routing).
- `return ( ... )` holds a JSX-like tree that lowers to a Leptos
  `view! { ... }` block; `onX` attributes become Leptos `on:x=` bindings.

## Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- [`wasm-bindgen-cli`](https://crates.io/crates/wasm-bindgen-cli), matching
  the `wasm-bindgen` version pulled in by `leptos` (check
  `example-app/Cargo.lock` after your first build):
  ```sh
  cargo install wasm-bindgen-cli --version <matching-version>
  ```
- Node.js 18+ and npm.

## 1. Build the compiler

```sh
cd kittine-compiler
cargo build --release
```

This produces `kittine-compiler/target/release/kittine-compiler`. You can
run it directly on any `.kitty` file:

```sh
./target/release/kittine-compiler build path/to/App.kitty
# -> writes path/to/App.rs
```

## 2. Install JS dependencies and build the Vite plugin

The repo root is an npm workspace containing `vite-plugin-kittine` and
`example-app`, so a single install hoists and dedupes shared packages
(notably `vite` itself — this matters, since the plugin imports Vite's
`Plugin` type):

```sh
npm install                # from the repo root
npm run build:plugin       # compiles vite-plugin-kittine/src -> dist/
```

## 3. Run the example app

```sh
npm run dev                # from the repo root; equivalent to `cd example-app && npm run dev`
```

Open the printed `http://localhost:5173/` URL. On first request for
`App.kitty`, the plugin will:

1. Run `kittine-compiler build src/App.kitty` to (re)generate `src/App.rs`.
2. Run `cargo build --target wasm32-unknown-unknown` for the crate.
3. Run `wasm-bindgen` to produce browser-ready JS + `.wasm` in
   `example-app/pkg/`.
4. Serve that glue module back to the browser.

The first load is slow (a full Rust/Leptos compile); subsequent edits to
`App.kitty` are incremental. You should see a "Clicks: 0" button that
increments on click, and `Welcome Admin` logged to the browser console.

To produce a production build:

```sh
npm run build               # from the repo root, or `cd example-app && npm run build`
cd example-app && npm run preview
```

## Known behavior

- `craft<...>` calls inside `if>`/`orif>`/`else>` blocks at the top level of
  a component run once, at component setup — they are not wrapped in a
  reactive `Effect`. Leptos will print a dev-mode warning if the condition
  reads a signal outside a tracked context; this is expected given the
  literal `if x.get() == "word" { .. }` translation the language spec
  calls for, and does not affect correctness of the generated app.
- The generated `.rs` files are marked "Generated by kittine-compiler. Do
  not edit by hand." — re-run the compiler (or just re-save the `.kitty`
  file with the dev server running) instead of editing them directly.

## Running the compiler's test suite

```sh
cd kittine-compiler
cargo test
```

## VS Code extension

Syntax highlighting and a dedicated file icon for `.kitty` files live in
[`vscode-kittine/`](vscode-kittine). See
[docs/VSCODE_EXTENSION.md](docs/VSCODE_EXTENSION.md) for install
instructions (including how to hand a `.vsix` to a friend).

## Community

- [CONTRIBUTING.md](CONTRIBUTING.md) — how to propose language/compiler changes.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — community standards.
- [SECURITY.md](SECURITY.md) — reporting a vulnerability.
- [CHANGELOG.md](CHANGELOG.md) — what changed and when.

## Author

Kittine (language, compiler, and tooling) was designed and created by
**Sivario Buchanan**.

## License

[MIT](LICENSE).
