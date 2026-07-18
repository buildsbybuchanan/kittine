# Kittine Documentation

- **[LANGUAGE.md](LANGUAGE.md)** — the full language reference: syntax,
  semantics, the compilation-to-Rust mapping for every construct, and
  known limitations.
- **[GETTING_STARTED.md](GETTING_STARTED.md)** — install the toolchain,
  build the compiler, run the example app, and start your own project.
- **[CLI.md](CLI.md)** — `kittine-compiler` command-line reference.
- **[VSCODE_EXTENSION.md](VSCODE_EXTENSION.md)** — installing and using the
  `.kitty` syntax highlighting extension, including how to share it.
- **[ARCHITECTURE.md](ARCHITECTURE.md)** — how the compiler, Vite plugin,
  example app, and VS Code extension fit together.
- **[SSR.md](SSR.md)** — server-side rendering via `cargo-leptos` + Axum
  (a separate toolchain from Vite), for when a public/SEO-sensitive site
  needs real first paint instead of `example-app`'s CSR default.
- **[ROADMAP.md](ROADMAP.md)** — what works today, what's next, and the
  full long-term vision. Update this whenever you find a gap.

New to Kittine? Start with [GETTING_STARTED.md](GETTING_STARTED.md), then
skim [LANGUAGE.md](LANGUAGE.md) alongside `example-app/src/App.kitty`.
