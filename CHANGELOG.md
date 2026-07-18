# Changelog

All notable changes to Kittine are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Kittine does not yet follow Semantic Versioning tags/releases — entries are
grouped by date until the first tagged release.

## [Unreleased] - 2026-07-18

### Investigated

- **SSR/SSG feasibility.** Checked Leptos 0.7's actual SSR requirements
  directly (the Leptos book, `cargo-leptos`'s own docs) rather than
  assuming: it needs a Rust HTTP server (`leptos_axum`/`leptos_actix`),
  a dual `hydrate`/`ssr`-feature-gated build of the same crate, and a
  `hydrate()` client entry point instead of `mount_to_body()`.
  `cargo-leptos`, the standard tool for this, explicitly isn't designed
  to run alongside Vite — it replaces the dev-server/build-orchestration
  role `vite-plugin-kittine` currently fills. Conclusion: this is a real
  architecture decision (retire Vite for SSR-mode projects and adopt
  `cargo-leptos`, or hand-roll Axum + a dual build alongside Vite), not
  a same-shape increment like the items below — staying CSR-only for
  now rather than force-fitting a partial answer. See
  [ROADMAP.md § Next up](docs/ROADMAP.md#next-up).

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
- **Routing**, with no dedicated Kittine syntax: `leptos_router` is now in
  scope in every generated file, and `Router`/`Routes`/`Route`/`A` are
  just ordinary components composed the existing way — `<Route
  path={StaticSegment('about')} view={About}/>` needed zero new language
  features, since `StaticSegment(..)` is a plain function call and
  `view={About}` is a bare component reference. `example-app` now has
  real `Home`/`About`/`NotFound` pages behind client-side routing,
  verified end-to-end with Playwright against the running dev server
  (navigation, the 404 fallback, and continued reactivity all checked, not
  just a successful compile).
- **Comparison operators**: `<`, `<=`, `>`, `>=`, `!=` alongside the
  existing `>>` (equality) — usable in `if>`/`orif>` conditions and
  generally anywhere an expression is (`purr` returns, `craft<...>`
  arguments, JSX interpolations), not just inside conditions. A bare `>`
  at the top level of `craft<expr>` is ambiguous with `craft<...>`'s own
  closing `>`; wrap it in parens (`craft<(age > 18)>`) — the only new
  caveat this introduces, everything else composes freely.
- **Array-typed props/returns**: `<<Num[]>>`/`<<Word[]>>`/`<<Flag[]>>` —
  `func NavList(<<Word[]>> items) { .. }` lowers to `items: Vec<String>`,
  a `purr` can return an array type the same way. Array type tags also
  check literal elements against the declared element type at parse time.
- **Method calls (`receiver.method(arg, ..)`)**: calls a method on the
  result of any expression, for interop with real Rust/Leptos APIs that
  aren't a Kittine `purr` — chains work (`a.b().c(1).d()`), since the
  receiver of a method call is itself an arbitrary expression. Kittine
  tracks no receiver/argument types here, so arguments render plain (no
  forced `f64` the way a same-file `purr` call gets), and Rust's own type
  checker validates the call.
- **Calling the result of an expression (`callee(arg, ..)` where `callee`
  isn't a bare name)**: most useful right after a call that returns a
  closure — `use_navigate()('/home')` calls the closure `use_navigate()`
  itself returns.
- **Tuple literals (`(expr, expr, ..)`)**: needed to combine multiple
  `leptos_router` path segments into one dynamic route. A single
  parenthesized expression with no comma is still just grouping, not a
  1-tuple.
- **Dynamic route segments, demonstrated end-to-end**: `example-app` now
  has a real `/user/:id` route (`User.kitty`), reached via a tuple path
  (`(StaticSegment('user'), ParamSegment('id'))`) and reading the segment
  back out via a method-call chain
  (`use_params_map().get().get('id').unwrap_or_default()`) — no dedicated
  Kittine syntax needed for either half, once method calls and tuples
  existed. Verified against real Leptos 0.7 and with Playwright driving
  the actual dev server (clicking the nav link, and navigating directly
  to `/user/999`, both showed the correct id). `leptos_router::hooks`
  (`use_params_map`, `use_navigate`, ..) is now in every generated file's
  fixed preamble alongside `leptos_router::components`/`leptos_router`
  itself — it wasn't re-exported at the crate root the way those are.
  Programmatic navigation (`use_navigate()`) remains a documented gap:
  its second argument needs `NavigateOptions::default()`, which needs a
  path-qualified expression (`Type::method()`) Kittine's grammar doesn't
  have yet — discovered while actually trying to wire up a working
  example, not assumed; see
  [LANGUAGE.md § Known limitations](docs/LANGUAGE.md#known-limitations).
- **A string literal passed to a same-file `purr`'s `<<Word>>` parameter
  now renders as an owned `String`.** `greet('World')`, where `greet` is
  defined as `purr greet(<<Word>> name) <<Word>> { .. }` in the same
  file, now lowers to `greet("World".to_string())` instead of
  `greet("World")` — the compiler already knows `greet`'s parameter
  types from its own definition, so it can render the argument correctly
  without guessing. A call through an `import`, or to a function Kittine
  has no signature for, is unaffected (still renders the literal bare) —
  real cross-file type information isn't available yet; see
  [LANGUAGE.md § Known limitations](docs/LANGUAGE.md#known-limitations).
- **Incremental builds for the import graph**: `kittine-compiler build`
  still recompiles every reachable `.kitty` file on every invocation (it
  has to, to know if anything changed), but now only actually *rewrites*
  a `.rs` file when its freshly generated content differs from what's
  already on disk — a `.kitty` file that recompiles to byte-identical
  Rust (including one whose only edit was a comment, since comments carry
  no codegen effect) leaves its output file's mtime untouched. This
  matters because downstream tooling decides whether to redo work by
  looking at file mtimes: `cargo build` recompiles a Rust module when its
  source file's mtime changes, and `vite-plugin-kittine`'s own
  `buildWasmIfNeeded` freshness check (`newestMtime` over the crate's
  `.rs`/`.kitty`/`.toml` files) was being defeated every single time,
  since unconditionally rewriting every reachable dependency made the
  whole crate look freshly modified on every build regardless of what
  actually changed. Editing one leaf `.kitty` file in `example-app` now
  triggers only that leaf's Rust module recompiling, verified with a real
  `npm run build` (~31s → ~5s for a no-op rebuild).
- **Logical `&&` / `||`**: combine two or more comparisons into one
  condition — `age >= 18 && status >> 'active'`, `age < 13 || age >= 65`.
  `&&` binds tighter than `||` (`a || b && c` reads as `a || (b && c)`).
  Works in `if>`/`orif>` conditions (each side still anchored on a
  `<{name}>` read, same as a single comparison today) and, with no such
  restriction, anywhere a general expression is valid (`purr` returns,
  `craft<...>` arguments, JSX `{ expr }`). Both lower to Rust's own
  `&&`/`||`, short-circuiting exactly as they do in Rust.
- **`private func`/`purr`**: opts a top-level item out of being importable
  (every `func`/`purr` is importable by default). Lowers to a plain
  (non-`pub`) Rust item — `import`ing a `private` one from another file is
  then a Rust compile error (`E0603`) on its own; Kittine doesn't
  re-implement that check itself.

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
- A string-literal signal initializer (`signal("Admin")`) now produces an
  owned `String` (`signal("Admin".to_string())`), not a borrowed
  `&'static str` — the same class of bug as the `f64` fix above, just for
  strings: `&str` compiles fine on its own, until that signal's value is
  later required to be an owned `String` (passed as a `Word`-typed prop to
  another component), where it silently fails. This fix is scoped to
  signal initializers specifically — a broader attempt (making every
  string literal owned everywhere) broke `leptos_router::StaticSegment`,
  which requires `&str`; reverted that part after catching it via a real
  compile, not just tests.
- A view-position `spin`'s loop variable (`{move || item}`) is now always
  `.clone()`d, regardless of its element type. Leptos's reactive closures
  need to be callable more than once (`Fn`), and moving a non-`Copy` item
  (a `Word`) out of one only satisfies `FnOnce` — invisible with a `Num`
  (`Copy`) array, which is what the original list-rendering feature had
  been tested with, and only surfaced once a `Word[]`-typed prop was
  actually rendered in a list.

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
