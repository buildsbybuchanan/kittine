# Server-side rendering (SSR) with Kittine

`example-app/` (the default, described everywhere else in these docs) is
**client-side rendered (CSR)**: the browser downloads a blank `index.html`
plus a WebAssembly bundle, and the whole page renders after that bundle
loads. That's simple and works great through `vite-plugin-kittine`, but it
means a crawler (or a slow connection) sees nothing until the WASM finishes
loading — not ideal for a public, SEO-sensitive site.

`example-ssr/` is the second, additional path: a real Kittine app rendered
**server-side** first (genuine HTML content in the very first response,
verified with `curl` — no JavaScript involved), then **hydrated**
client-side for interactivity. This is a separate toolchain from
`vite-plugin-kittine` — see [Why this needs a different
toolchain](#why-this-needs-a-different-toolchain) for why, and use whichever
example matches what you're building.

**`kittine-compiler` itself needs zero changes for this.** The exact same
`.kitty` → `.rs` compiler that produces `example-app`'s output produces
`example-ssr`'s — routing, props, `spin`, `hold`, all of it, unchanged. The
only thing that's different is what *builds and serves* the generated Rust,
which is the whole subject of this document.

## Quickstart

```sh
cargo install cargo-leptos --locked
cd example-ssr
cargo leptos serve
```

Open `http://127.0.0.1:3000/`. View-source (not devtools — the *raw* HTTP
response) to see real, pre-rendered HTML content, no JavaScript required.
Click around — the page hydrates and becomes fully interactive, including
client-side `<A>` navigation between routes, exactly like `example-app`.

### If `cargo install cargo-leptos` fails on Windows

Building `cargo-leptos` from source pulls in `openssl-sys`, which tries to
compile OpenSSL from source and needs a Perl installation with modules
Windows's bundled Perl (via Git Bash / MSYS) often lacks
(`Locale::Maketext::Simple`, surfacing as `perl reported failure with exit
code: 2` deep in the build log). The fix that worked here: skip building
from source entirely and use the prebuilt binary from [cargo-leptos's GitHub
releases](https://github.com/leptos-rs/cargo-leptos/releases) —
`cargo-leptos-x86_64-pc-windows-msvc.tar.gz`, extract it, and copy
`cargo-leptos.exe` into `%USERPROFILE%\.cargo\bin\`.

## Project structure

```
example-ssr/
  Cargo.toml   # [package.metadata.leptos] config; ssr/hydrate features
  src/
    App.kitty  # compiled by kittine-compiler, same as example-app
    Home.kitty
    About.kitty
    lib.rs     # #[wasm_bindgen] hydrate() entry point (feature "hydrate")
    main.rs    # Axum server binary (feature "ssr")
```

Every `.kitty` file compiles exactly the way it does in `example-app` —
`kittine-compiler build src/App.kitty` produces `src/App.rs`, resolving
`import`s recursively the same way. The difference is entirely in
`Cargo.toml`, `lib.rs`, and `main.rs`.

### `Cargo.toml`

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { version = "0.7" }
leptos_router = { version = "0.7" }
leptos_meta = { version = "0.7" }
leptos_axum = { version = "0.7", optional = true }
axum = { version = "0.7", optional = true }
tokio = { version = "1", features = ["rt-multi-thread"], optional = true }
wasm-bindgen = "=0.2.126"   # must match your installed wasm-bindgen-cli exactly
console_error_panic_hook = "0.1"

[features]
hydrate = ["leptos/hydrate"]
ssr = ["dep:axum", "dep:tokio", "dep:leptos_axum", "leptos/ssr", "leptos_router/ssr", "leptos_meta/ssr"]

[package.metadata.leptos]
output-name = "example-ssr"
site-root = "target/site"
site-pkg-dir = "pkg"
site-addr = "127.0.0.1:3000"
bin-features = ["ssr"]
bin-default-features = false
lib-features = ["hydrate"]
lib-default-features = false
```

`cargo-leptos` builds this **one crate twice**: once natively with the
`ssr` feature (the Axum server binary, `main.rs`), once for
`wasm32-unknown-unknown` with the `hydrate` feature (the client bundle,
`lib.rs`). This is the fundamental shape of every Leptos SSR project, not
something Kittine adds — see [How this fits into Leptos's own
model](#how-this-fits-into-leptos-own-model).

**`wasm-bindgen`'s version must exactly match your installed
`wasm-bindgen-cli`** (`wasm-bindgen --version`) — a mismatch fails with a
schema-version error at the `wasm-bindgen` step, not at `cargo build`.

### `lib.rs` — the client entry point

```rust
#[path = "App.rs"]
pub mod app;
pub use app::App;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
```

`hydrate_body`, not `mount_to_body` (which `example-app`'s CSR entry point
uses) — hydration attaches Leptos's reactive system to DOM nodes that
already exist (from the server-rendered HTML), rather than building them
from scratch.

### `main.rs` — the server

The server binary uses `leptos_axum` to turn every route into an Axum
route, rendering `App` (the exact same Kittine-compiled component tree) to
an HTML string per request:

```rust
fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <HydrationScripts options/>
                <MetaTags/>
                <title>"Kittine + SSR example"</title>
            </head>
            <body><App/></body>
        </html>
    }
}

let app = Router::new()
    .leptos_routes(&leptos_options, routes, {
        let leptos_options = leptos_options.clone();
        move || shell(leptos_options.clone())
    })
    .fallback(leptos_axum::file_and_error_handler(shell))
    .with_state(leptos_options);
```

Two real gotchas found by actually running this, not assumed:

- **`<HydrationScripts>` comes from `leptos`'s own prelude, not
  `leptos_meta`** (`MetaTags` does come from `leptos_meta`) — it's the
  component that injects the `<script type="module">` tag loading the
  client wasm bundle. Omit it and the page renders correctly server-side
  but **never becomes interactive** — nothing ever loads the client
  bundle, and there's no error, just a page that looks right and does
  nothing.
- **`.fallback(leptos_axum::file_and_error_handler(shell))` is required**
  to actually serve the wasm/JS bundle `<HydrationScripts>` references —
  `.leptos_routes(..)` alone only handles the page routes themselves, not
  static assets. Without it, the hydration script's own `import(...)` call
  404s, and hydration silently never happens (a real
  `page.on('pageerror')` in a browser, not a server-side compile error).

## Why this needs a different toolchain

`vite-plugin-kittine` drives `example-app`'s entire dev loop: compile
`.kitty` → build one `wasm32` target → serve via Vite's own dev server.
SSR needs a **second, native build of the same crate** (the `ssr`-feature
server binary) running as a real, persistent HTTP server — Vite has no
role to play in that; it doesn't build or run native Rust binaries.
`cargo-leptos` is the tool built specifically to orchestrate both builds
(client wasm + server binary) together, with its own dev server and watch
mode (`cargo leptos watch`).

This was investigated properly before choosing it (see
[ROADMAP.md](ROADMAP.md) history) — `cargo-leptos`'s own docs describe it
as "not designed for parallel use with Vite or similar tools," and there's
no smaller "SSG-only, no server" shortcut in Leptos 0.7: even generating
static files once at build time still needs the same `ssr`-feature native
build and the same `leptos_axum` rendering machinery, just run once
instead of listening on a socket forever. Hand-rolling that machinery
directly (bypassing `leptos_axum`) would mean reimplementing internals
`leptos_axum` already gets right — not a smaller task, a riskier one.

**Both paths are legitimate, for different projects:**

| | `example-app` (CSR) | `example-ssr` (SSR) |
|---|---|---|
| Toolchain | Vite + `vite-plugin-kittine` | `cargo-leptos` |
| First paint | Blank until WASM loads | Real HTML immediately |
| Crawler-visible content | No (needs JS execution) | Yes |
| Best for | Internal tools, apps behind a login | Public, SEO-sensitive sites |
| `kittine-compiler` changes needed | None | None |

## How this fits into Leptos's own model

Leptos 0.7 has no concept of "static site generation" distinct from SSR —
there's no built-in "render once at build time, ship files" mode. What
Leptos calls `ssr` (a Cargo feature) means "render component trees to an
HTML string," and doing that at all requires a native build plus a server
integration crate (`leptos_axum` here; `leptos_actix` and others exist for
other frameworks) to supply the request/response and reactive-context
plumbing around that rendering — regardless of whether you serve that HTML
per-request (real SSR) or generate it once and save it to a file
(SSG-style). `example-ssr` demonstrates the SSR case, since that's what
`leptos_axum`/`cargo-leptos serve` gives you directly; a build-time-only
static-file variant would reuse the exact same rendering path, just driven
once per route instead of per-request — not yet demonstrated here, but not
a different architecture either.

## Deploying

`cargo leptos build --release` produces the server binary
(`target/release/example-ssr` or `.exe`) and the client bundle
(`target/site/pkg/`). Deploying SSR means running that server binary as a
real, persistent process (a container, a VM, a platform that runs
arbitrary binaries) — unlike `example-app`'s CSR output, this isn't static
files you can drop on any static host. This is a real, meaningful
trade-off against CSR's simpler "just static files" deployment story, in
exchange for real first paint and SEO.
