# Changelog

All notable changes to Kittine are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Kittine does not yet follow Semantic Versioning tags/releases — entries are
grouped by date until the first tagged release.

## [Unreleased] - 2026-07-18

### Changed

- **Breaking:** removed the `¨...¨` diaeresis-quoted string form. Strings are
  now written with `'...'` or `"..."` — fully interchangeable, pick whichever
  avoids escaping. Existing `.kitty` source using `¨...¨` must be updated.

### Added

- **Boolean literals**: `yes>` / `no>`, lowering to Rust's `true` / `false`.
- **Array literals**: `[expr, expr, ..]`, lowering to `vec![..]`.
  `craft<[..]>` formats arrays with `{:?}` instead of `{}`.
- **Type tags**: `<<Num>>`, `<<Word>>`, `<<Flag>>` — optional explicit type
  annotations on a value. Checked against literal values at parse time
  (`<<Num>> 'oops'` is now a parse error) and erased during code generation.
- **`spin` loops**: `spin<{item}> in list }{ .. }{` iterates an array,
  binding each element to `item`. Lowers to a plain Rust `for` loop. The
  `}{` fence — a closing brace immediately followed by an opening one — is
  the loop-body delimiter.
- `vercel.json` and `.vercelignore` for one-step Vercel deployment of
  `example-app`: installs a minimal Rust toolchain and `wasm-bindgen-cli`
  (version read from `Cargo.lock`, not hardcoded) during the Vercel build,
  then runs the existing `npm run build:plugin && npm run build` pipeline.
- Immutable long-term caching headers for Vite's content-hashed
  `example-app/dist/assets/*` output.
- **Component props**: `func Name(<<Type>> prop, ..) { .. }`. Props are
  plain typed values (not signals) — a `Word` prop is cloned at read sites
  to avoid Rust move-checker conflicts across multiple reactive closures.
- **`purr` functions**: `purr name(<<Type>> param, ..) <<ReturnType>> { ..
  return (expr) }` — a plain, non-view-rendering function.
- **Function calls**: `name(arg, ..)`, valid anywhere an expression is.
- **Component composition**: a PascalCase JSX tag (`<Nav .. />`) is a
  reference to another component, matching Leptos's own `view!` macro
  convention — its attributes are passed as plain prop values instead of
  the reactive `move || ..` closures a real HTML attribute gets.
- **Modules**: `import { A, B } from './file.kitty'`. `kittine-compiler
  build` resolves and compiles the whole import graph recursively (cycle
  detection included), emitting a `#[path] mod .. use ..;` per import.
- `docs/ROADMAP.md`: a living plan (status / next-up / full vision) for
  where Kittine is headed, replacing ad-hoc scope discussions.
- **List rendering in views**: `spin<{item}> in list }{ .. }{` can now
  appear inside `return ( ... )` (not just as a statement), lowering to a
  reactive Leptos `<For each=.. key=|item| format!("{item}") let:item>`
  instead of a plain imperative `for` loop. The key is always
  `format!("{item}")`, which works uniformly across `Num`/`Word`/`Flag`
  since all three implement `Display`.
- **Component children**: an untyped `children` parameter (the one
  exception to every prop needing a type tag) lets a component render
  whatever JSX content it's composed with — `func Card(children) { ..
  { children() } .. }`, called with `<Card><p>"x"</p></Card>`. No
  `children=` attribute needed at the call site; Leptos's `view!` macro
  wires nested JSX into the `children` prop automatically.

### Fixed

- `example-app/Cargo.toml`'s `[profile.release]` now sets `lto = true`,
  `codegen-units = 1`, `strip = true`, and `panic = "abort"` alongside the
  existing `opt-level = "s"`, cutting the shipped `.wasm` from ~227 KB to
  ~76 KB (~54 KB → ~28 KB gzipped) for the example app.
- Whole-number literals are now spelled with an explicit `f64` suffix
  wherever they're an operand next to an already-concrete `f64` value: a
  signal initializer (`signal(0)` → `signal(0f64)`), a compound-assignment
  right-hand side (`*n += 1` → `*n += 1f64`), a general arithmetic operand,
  and a direct function-call argument. Without this, Rust's generic
  inference for `signal(..)`'s type parameter is free to resolve to
  something other than `f64` when nothing *else* pins it down first — and
  it silently fails to compile the moment that value is later required to
  be concretely `f64` (e.g. passed into a `purr` call). Found by actually
  compiling generated output against real Leptos, not just asserting on
  generated-string snapshots.

### Repository

- `main` is now the GitHub default branch; the old auto-generated
  `merged-syntax-branch` branch has been deleted.

## [0.1.0] - 2026-07-17

### Added

- Initial Kittine monorepo: `kittine-compiler` (lexer, parser, codegen
  targeting Leptos 0.7), `vite-plugin-kittine`, and `example-app`.
- `func`, `<{name}> >> value` (declare-or-mutate signals), `craft<...>`
  logging, `if>` / `orif>` / `else>` indentation-delimited control flow,
  and the embedded JSX-like `return ( ... )` view syntax.
- String concatenation via `+`: either operand being a string literal
  lowers the whole expression to `format!("{}{}", ..)`.
- `vscode-kittine`: a VS Code extension with TextMate-grammar syntax
  highlighting, bracket matching, and a dedicated file icon for `.kitty`
  files.
- Full documentation set under `docs/` (language reference, architecture,
  CLI, getting started, VS Code extension).
