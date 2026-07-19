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
[architecture](docs/ARCHITECTURE.md) · [SSR](docs/SSR.md) ·
[roadmap](docs/ROADMAP.md)

This repository is a monorepo with five parts:

- **`kittine-compiler/`** — a Rust CLI that lexes, parses, and lowers
  `.kitty` source into a Leptos 0.7 `.rs` file. Identical output whether
  the result ends up client-side rendered or server-rendered — see
  `example-app` vs. `example-ssr` below.
- **`vite-plugin-kittine/`** — a Vite plugin that runs `kittine-compiler` on
  `.kitty` files, builds the host Rust crate to `wasm32-unknown-unknown`,
  post-processes it with `wasm-bindgen`, and serves the result to the
  browser.
- **`example-app/`** — a real multi-page Leptos **client-side rendered**
  (CSR) app: `App.kitty` wraps `Home`/`About`/`User`/`NotFound` pages in a
  `leptos_router` `<Router>`, with client-side `<A>` navigation, a dynamic
  `/user/:id` route, and a 404 fallback. `Home.kitty` demonstrates a
  signal-backed counter, an `if>`/`orif>`/`else>` block (including
  `&&`/`||`-combined conditions), a `spin` loop (both imperative and
  list-rendering forms), `purr` functions, and composed
  `Nav`/`Card`/`NavList` components; `User.kitty` reads its dynamic segment
  via a method-call chain on `use_params_map()` — all wired up through the
  Vite plugin.
- **`example-ssr/`** — the same idea, **server-side rendered** via
  `cargo-leptos` + Axum instead of Vite: real HTML content in the first
  response (verified with `curl`, no JavaScript needed), hydrated
  client-side for interactivity. `kittine-compiler` needs zero changes for
  this — see [docs/SSR.md](docs/SSR.md) for the toolchain, the two real
  gotchas found while wiring it up, and when to reach for this over
  `example-app`.
- **`vscode-kittine/`** — a VS Code extension providing syntax highlighting
  and a file icon for `.kitty` files. See
  [docs/VSCODE_EXTENSION.md](docs/VSCODE_EXTENSION.md) to install it.

## Language quick reference

```kitty
import { Nav } from './Nav.kitty'

purr doubled(n) {
    return (n * 2)
}

func Counter() {
    <{count}> >> #n 0
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
  `hold name >> expr` is a plain, non-reactive local binding instead
  (`let name = expr;`) — for calling a Leptos hook that needs to run
  eagerly at component setup, like `use_navigate()`.
- `'...'` and `"..."` are fully interchangeable string literals.
- `yes>` / `no>` are boolean literals; `[a, b, c]` is an array literal.
- `#n` / `#w` / `#f` (or `#n[]`/`#w[]`/`#f[]`
  for an array of one) are type tags — optional everywhere, including a
  prop or `purr` param/return type, where an omitted one is
  [inferred](docs/LANGUAGE.md#type-inference) from how the name is used —
  checked against literal values at compile time and erased in the
  generated Rust. Array tags are the one spot still mandatory.
- `craft<expr>` logs via `leptos::logging::log!`.
- `if>` / `orif>` / `else>` are indentation-delimited (no braces), with
  `>>`/`<`/`<=`/`>`/`>=`/`!=` as comparisons — usable in conditions and
  generally anywhere an expression is (a `purr` return, `craft<...>`).
  `&&`/`||` combine two or more comparisons into one condition (`<{age}>
  >= 18 && <{status}> >> 'active'`), `&&` binding tighter than `||`.
- `spin<{item}> in list }{ .. }{` loops over an array, its body fenced by
  `}{` — a closing brace immediately followed by an opening one. As a
  statement it's a plain imperative loop; inside `return ( ... )` it
  renders one element per item via a reactive Leptos `<For>`, keyed by
  `format!("{item}")` by default or by an optional `key(expr)` clause.
- `func Name((#t)? prop) { .. }` takes props, typed or inferred;
  `<Name prop='x' />` composes it into another view (a capitalized JSX tag
  is a component reference, lowercase is a plain HTML element).
- `func Card(children) { .. { children() } .. }` — the special untyped
  `children` param accepts nested JSX from wherever the component is
  composed: `<Card><p>"nested"</p></Card>`, no extra syntax needed at the
  call site.
- `purr name((#t)? param) (#t)? { .. return (expr) }` is a plain
  function — computes and returns a value, renders no view — called like
  `name(arg)` anywhere an expression is valid.
- `import { Name } from './file.kitty'` brings another file's
  components/functions into scope; `kittine-compiler` resolves and
  compiles the whole import graph recursively. `private func`/`purr`
  opts out of being importable at all — enforced by Rust's own privacy
  rules (E0603), not by Kittine re-checking it. `export import { Name }
  from './file.kitty'` re-exports it, so a third file can import through
  this one — see [Re-exports](docs/LANGUAGE.md#re-exports).
- **Routing has no Kittine-specific syntax** — `leptos_router` is in scope
  everywhere, and `<Router>`/`<Routes>`/`<Route>`/`<A>` compose exactly
  like any other component: `<Route path={StaticSegment('about')}
  view={About}/>`. A dynamic segment combines a [tuple](docs/LANGUAGE.md#tuples)
  and a [method-call chain](docs/LANGUAGE.md#method-calls):
  `<Route path={(StaticSegment('user'), ParamSegment('id'))} view={User}/>`,
  read back via `use_params_map().get().get('id')`. See
  [Routing](docs/LANGUAGE.md#routing).
- `receiver.method(arg, ..)` calls a method on any expression's result
  (chains work); `(a, b)` is a tuple literal; `Type::method()` /
  `Type::CONST` is a path-qualified expression — all three exist mainly
  for interop with real Rust/Leptos APIs (this is what makes programmatic
  navigation via `use_navigate()` fully expressible). See [Method
  calls](docs/LANGUAGE.md#method-calls),
  [Tuples](docs/LANGUAGE.md#tuples), and [Path-qualified
  expressions](docs/LANGUAGE.md#path-qualified-expressions).
- `return ( ... )` holds a JSX-like tree that lowers to a Leptos
  `view! { ... }` block; `onX` attributes become Leptos `on:x=` bindings.

## Shorter than the Rust it generates

Kittine's syntax is deliberately compact and unusual — every construct is
built to be quicker to type and read than the Rust it lowers to, not just
different from it:

| Kittine | Generated Rust |
| --- | --- |
| `purr greet(name) { return ('Hello, ' + name) }` | `pub fn greet(name: String) -> String { format!("Hello, {name}") }` |
| `greet('World')` (no tag anywhere — [inferred](docs/LANGUAGE.md#type-inference) from `'Hello, ' + name`) | `greet("World".to_string())` |
| `<{count}> >> #n 0` | `let (count, set_count) = signal(0f64);` |
| `<{ready}> >> yes>` | `let (ready, set_ready) = signal(true);` |
| `craft<'Welcome Admin'>` | `leptos::logging::log!("Welcome Admin");` |
| `spin<{n}> in [1, 2, 3] }{ craft<n> }{` | `for n in (vec![1, 2, 3]).into_iter() { leptos::logging::log!("{}", n); }` |

See [docs/LANGUAGE.md § Brevity by design](docs/LANGUAGE.md#brevity-by-design)
for the two rules that keep this true as the language grows, and
[docs/LANGUAGE.md § Type inference](docs/LANGUAGE.md#type-inference) for
how a `purr`/prop signature can drop its type tags entirely.

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
