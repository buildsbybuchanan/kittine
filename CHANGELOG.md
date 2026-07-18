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

- **`hold name >> expr`**: a plain, non-reactive local binding — unlike
  `<{name}> >> value`, never declares a signal. Lowers to a bare `let
  name = expr;`, evaluated once. Exists specifically for calling a
  Leptos hook that depends on reactive context (`use_navigate()`, ...)
  eagerly at component setup instead of lazily inside an event handler
  (see the move-conflict fix below and `docs/LANGUAGE.md` §
  Programmatic navigation for why that timing matters). Reading a
  `hold`-bound name — bare, calling it, via a method call, or via
  calling the result of an expression — gets the same `.clone()`
  treatment a `Word`/array prop already had. Verified against real
  Leptos 0.7, then wired into `example-app`'s `User.kitty`, replacing
  the signal-based workaround the first version of programmatic
  navigation had used.
- **Re-exports**: `export import { Name } from './file.kitty'` re-exposes
  an imported name under the importing file's own name — a third file
  can then `import { Name }` from *this* file without reaching all the
  way back to wherever `Name` is actually defined. Lowers to `pub use`
  instead of a plain `use`; the intermediate `mod` declaration stays
  non-`pub` either way (a `pub use` re-export doesn't need its source
  module to be `pub`, only the item being re-exported). Verified with a
  real three-file chain against actual Leptos 0.7, then wired into
  `example-app` as a `components.kitty` barrel file re-exporting
  `Nav`/`Card`/`NavList`, confirmed with Playwright against the running
  dev server.
- **Path-qualified expressions**: `Type::method()`, `Type::CONST`, and
  multi-segment paths like `std::cmp::max(1, 2)` — renders verbatim
  (`segments.join("::")`) and combines with the existing method-call
  chain / calling-the-result-of-an-expression machinery for free.
  Completes the programmatic-navigation demo `use_navigate()` needed:
  `NavigateOptions::default()` was the one piece blocking it. Verified
  against real Leptos 0.7, then wired into `example-app`'s `User.kitty`
  — and confirmed to actually work with Playwright against a real
  running dev server (a click that changed the URL, not just a
  successful compile).
  - **Real gotcha found along the way, not assumed**: calling
    `use_navigate()` *lazily*, inside the `onClick` handler itself,
    compiles fine but panics at runtime (`You cannot call use_navigate
    outside a <Router>`) — Leptos's context-dependent hooks resolve
    their context against whichever reactive owner is active when the
    hook function actually runs, which is correct during a component's
    synchronous setup but not by the time a click fires later from the
    browser's event loop. `example-app`'s `User.kitty` calls
    `use_navigate()` eagerly instead, via `<{navigate}> >>
    use_navigate()` (repurposing signal declaration to force the right
    timing, since Kittine has no plain non-reactive local binding yet —
    see [ROADMAP.md § Next up](docs/ROADMAP.md#next-up)), and reads it
    back with `<{navigate}>('/', NavigateOptions::default())`.
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
- **`key(expr)` control for view-position `spin`**: an optional clause
  right before the `}{` fence overrides the default
  `key=|item| format!("{item}")` Leptos `<For>` key — `spin<{item}> in
  items key(item.to_uppercase()) }{ .. }{` lowers to `key=|item|
  item.clone().to_uppercase()`. `item` is in scope while evaluating the
  key expression, same as in the body (and gets the same `.clone()`
  treatment — Leptos's `<For>` key closure receives `&T`, and `.clone()`
  on a `&T` still yields an owned `T`). `key` isn't a reserved word — only
  recognized contextually in this one position, the same way `in` already
  is — so it stays available as an ordinary identifier everywhere else.
  Verified against real Leptos 0.7 and wired into `example-app`'s
  `NavList.kitty`.
- **A string literal passed to a `<<Word>>`-typed `purr` parameter now
  renders as an owned `String`, whether the callee is same-file or
  reached through the whole `import` graph.** `greet('World')`, where
  `greet` is `purr greet(<<Word>> name) <<Word>> { .. }`, now lowers to
  `greet("World".to_string())` instead of `greet("World")` — landed in
  two steps: first for same-file calls (the compiler already knows the
  callee's signature from its own definition), then extended across
  `import`s by having `kittine-compiler build` collect every reachable
  file's `purr` signatures in a first lex+parse-only pass, before
  generating any single file's code (every file is parsed twice across a
  full build — cheap next to what `cargo`/`wasm-bindgen` cost
  downstream). Only a call to a function Kittine has no signature for at
  all (a real Rust/Leptos function, or a typo) still renders the literal
  bare. `example-app`'s `Home.kitty` now imports `greet` from a new
  `Greetings.kitty` to demonstrate exactly this.
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

- **Mutating a `Word` signal directly to a brand-new literal now
  compiles.** `<{label}> >> 'reset'` as a *mutation* (not the signal's
  first/declaring occurrence) used to render `*n = "reset"` — a bare
  `&'static str` assigned into `*n: &mut String`, which doesn't
  type-check. Same class of fix as `render_signal_init`'s existing
  treatment of a signal's first occurrence, just for a later mutation.
  Concatenation (`<{label}> >> 'x' + <{label}>`) was never affected.
  Verified against real Leptos 0.7, then wired a "Reset to Guest" button
  into `example-app`'s `Home.kitty`, confirmed with Playwright against
  the running dev server.
- **A non-`Copy` value (a `Word`/array prop, a view-position `spin`'s loop
  variable, or a `hold`-bound local) read from more than one reactive
  closure within the same component now compiles.** A `move` closure
  captures every variable it uses *by value* — including ones only ever
  read via `.clone()` inside the closure body — so two sibling closures
  both reading the same original value (`<h1>{ active }</h1><p>{ active
  }</p>`, or two buttons both calling a `hold`-bound `navigate`) used to
  fight over moving it, and the second one failed with `E0382: use of
  moved value`. Found by actually compiling that exact pattern against
  real Leptos, not assumed — the existing "always `.clone()` inside the
  closure" fix only ever handled a single closure being called more than
  once *reactively*, not two separate closures both referencing the same
  source variable. Fixed by pre-cloning every non-`Copy` tracked name a
  closure references into its own local **before** the closure itself
  (`{ let active = active.clone(); move || active.clone() }`) — rustc's
  own E0382 diagnostic suggests exactly this shape. Applies everywhere a
  reactive closure is generated: JSX child interpolations, event
  handlers, and other JSX attribute expressions.
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
