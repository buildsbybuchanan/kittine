# Kittine Roadmap

This is the living plan for Kittine: what actually works today, what's next,
and the full long-term vision. It exists so a future session — working in
this repo or in some *other* project that just needs "a bit more Kittine"
— can tell at a glance what's real, what's next, and where to pick up.

**Standing rule for anyone (human or agent) working with Kittine, even from
another repo:** if you hit a missing dependency, a feature gap, or discover
something Kittine needs to support your use case, add it to the
[Next up](#next-up) section below (or a new phase if it doesn't fit) and
push the change to this repo's `main`. This file must stay the single
source of truth for "what does Kittine actually support right now" —
don't let that knowledge live only in a chat transcript or a sibling
project's notes.

## Status: what works today

Verified by `cargo test` (`kittine-compiler/src/tests.rs`) and by actually
compiling `example-app` against real Leptos 0.7 — not aspirational.

- **Core language**: components (`func`), plain functions (`purr`),
  reactive state (`<{name}> >> value`), a plain non-reactive local
  binding (`hold name >> expr`), `if>`/`orif>`/`else>` control flow,
  `spin` loops (both imperative-statement and reactive-list-in-view forms),
  function calls, arithmetic/string-concat expressions, comparisons
  (`>>`/`<`/`<=`/`>`/`>=`/`!=`, usable generally, not just in conditions),
  arrays (including array-typed props/returns — scalar `#n[]`/`#w[]`/
  `#f[]` *and now a `litter`/`breed` array*, e.g. `DocEntry[]` — see
  [LANGUAGE.md § A litter/breed name as a prop or purr param/return
  type](LANGUAGE.md#a-litterbreed-name-as-a-prop-or-purr-paramreturn-type)),
  booleans, type tags (`#n`/`#w`/`#f`, a two-character sigil form —
  see [LANGUAGE.md § Type tags](LANGUAGE.md#type-tags)) that are now
  **optional** on a prop or `purr` param/return type, inferred from body
  usage when omitted (see [LANGUAGE.md § Type
  inference](LANGUAGE.md#type-inference)),
  logical `&&`/`||` combining comparisons into one condition, method
  calls (`receiver.method(arg, ..)`, chains work), calling the result of
  an expression (`callee(arg, ..)` where `callee` isn't a bare name),
  tuple literals (`(expr, expr, ..)`), path-qualified expressions
  (`Type::method()`, `Type::CONST`, multi-segment paths), and closure
  literals (`|param, ..| expr`, lowering verbatim to a Rust closure — the
  missing filter/map-predicate mechanism a real iterator method needs, see
  [LANGUAGE.md § Closures](LANGUAGE.md#closures)).
- **User-declared data types**: `litter Name { field type, .. }` (a
  struct — `Name { field: expr, .. }` constructs it, `.field` reads a
  field), `breed Name { Variant(type)?, .. }` (a closed set of variants —
  an enum — `Circle(5)`/bare `Idle` construct one), `pounce> subject`
  pattern-matching a `breed` value — both as a **statement** (branch and
  act) and, now, as an **expression** (branch and *compute*, e.g. `return
  (pounce> result Ok(v) >> v Err(e) >> 0)`, unwrapping a `Result`-shaped
  value and returning it in one step, the way Rust's own `match`/`?`
  can) — and minimal generics groundwork (at most one type parameter per
  `litter`/`breed`, e.g. `litter Holder<#t> { value #t }`, inferred at
  each construction site with no explicit instantiation) — and a real
  **trait system**: `claw Name { method(params) type, .. }` declares one,
  `bare Claw for Target { purr method(..) .. }` implements it (Rust's
  `impl Claw for Target`), and the generic type parameter above can be
  bounded by one (`litter NamedHolder<#t: Named> { value #t }`, a real
  Rust trait bound Rust's own compiler enforces). See [LANGUAGE.md §
  Litters](LANGUAGE.md#litters), [§ Breeds](LANGUAGE.md#breeds), [§
  Pattern matching](LANGUAGE.md#pattern-matching), [§
  Generics](LANGUAGE.md#generics), [§ Claws](LANGUAGE.md#claws).
- **Modules**: `import { A, B } from './file.kitty'`, resolved and compiled
  recursively by `kittine-compiler build` (cycle detection included).
  `private func`/`purr` opts an item out of being importable, enforced by
  Rust's own privacy rules. `export import { A } from './file.kitty'`
  re-exports `A` (`pub use`), so a third file can import it through this
  one instead of reaching back to where it's actually defined.
- **Component composition**: `<Name prop='x' />` for a PascalCase tag,
  passing typed props as plain values (not reactive DOM-attribute
  closures).
- **List rendering in views**: `spin<{item}> in list }{ .. }{` inside
  `return ( ... )` lowers to a reactive Leptos `<For>`, keyed by
  `format!("{item}")` by default, or by an optional `key(expr)` clause.
- **Component children**: an untyped `children` param (`func Card(children)
  { .. { children() } .. }`) renders whatever JSX a caller nests inside
  `<Card>...</Card>` — no `children=` attribute needed, Leptos's `view!`
  macro wires it through automatically.
- **Routing**: `leptos_router` is in scope in every generated file
  (including `leptos_router::hooks`, added this round);
  `Router`/`Routes`/`Route`/`A` compose exactly like any other component,
  with no dedicated Kittine syntax at all (see
  [LANGUAGE.md § Routing](LANGUAGE.md#routing)). Dynamic route segments
  work end-to-end — a tuple path (`(StaticSegment('user'),
  ParamSegment('id'))`) plus a method-call chain
  (`use_params_map().get().get('id')`) to read the value back — verified
  in `example-app`'s real `/user/:id` page with Playwright against a
  running dev server. Programmatic navigation (`use_navigate()`) also
  works, via [path-qualified expressions](LANGUAGE.md#path-qualified-expressions)
  (`NavigateOptions::default()`) and calling the hook eagerly (see
  [LANGUAGE.md § Programmatic navigation](LANGUAGE.md#programmatic-navigation)
  for the non-obvious but necessary pattern).
- **Server-side rendering (SSR) is real**, via a second toolchain
  (`cargo-leptos` + Axum, not Vite) — `example-ssr/` is a working
  multi-page site verified with real `curl` (genuine HTML content in the
  first response, no JS needed) and Playwright (hydration + client-side
  routing both confirmed against a running server). `kittine-compiler`
  needed **zero changes** — the exact same `.kitty` → `.rs` output that
  powers `example-app`'s CSR build powers `example-ssr` too. See
  [SSR.md](SSR.md) for the full setup and the real gotchas found while
  wiring it up (`HydrationScripts` placement, static-asset serving).
  `example-app`'s Vite-based CSR path
  is unaffected and remains the simpler default for apps that don't need
  SEO/first-paint.
- **Kebab-case JSX attributes** (`data-*`/`aria-*`) parse correctly —
  a `-` right after an attribute name is always a name continuation,
  never subtraction. `leptos_meta` (`<Title>`/`<Meta>`/`<Link>`/
  `<Stylesheet>`) is now in scope in every generated file alongside
  `leptos_router`, ready for a page to use once Leptos's own
  `provide_meta_context()` wiring is documented — no `.kitty` file
  exercises it yet.
- **Reading an `on<Event>` handler's event value.** The reserved
  identifier `event`, used anywhere inside an `on<Event>` handler's own
  expression, now reads the fired event's string value
  (`event_target_value(&ev)`) — `onInput={<{query}> >> event}` mutates
  `query` to whatever the user actually typed, instead of only a fixed
  literal. The handler's closure binds `ev` (`move |ev| ...`) only when
  the expression actually references `event`; every other handler still
  gets the cheaper `move |_| ...` it always did. Scoped narrowly (a
  `Scope::event_bound` flag threaded only through an `on<Event>`
  attribute's own render call), not a blanket reserved word — a real
  signal/param/`hold` binding named `event` anywhere else in a program
  keeps rendering normally. See [LANGUAGE.md § The view
  syntax](LANGUAGE.md#the-view-syntax). Verified with 2 new compiler
  tests (140 total) and a real `cargo check` against Leptos 0.7 —
  `example-app`'s `Home.kitty` gained a real text `<input>` that types
  directly into the `username` signal, replacing what was previously only
  a fixed-value "Reset to Guest" button.
- **Phase 2 (standard library) begins: collections beyond arrays, JSON
  serialization, a real reference operator, and logging levels.**
  - **`stash{ key: expr, .. }`** ("collections beyond arrays" from [Full
    vision § Phase 2](#phase-2--standard-library)) is a real `String`-keyed
    map, lowering to `std::collections::HashMap::from([..])`. Reuses the
    exact same grammar `litter` construction already has (`Name { field:
    expr, .. }`, just with the reserved name `stash`), so parsing/`fmt`
    round-tripping/lint all came for free. Typed like an array
    (`#n{}`/`#w{}`/`#f{}` — a `Num`/`Word`/`Flag`-valued map, mirroring
    `#n[]`/`#w[]`/`#f[]`), scalar-values-only for now, same scope limit as
    arrays. See [LANGUAGE.md § Stashes](LANGUAGE.md#stashes).
  - **Every `litter`/`breed` now derives `serde::Serialize`/
    `serde::Deserialize`** unconditionally (same as the existing `Clone,
    Debug`), so a value round-trips through JSON — or any other
    serde-backed format (YAML, CSV, ...) — via a plain path-qualified
    call, no dedicated syntax needed. This is what "JSON/XML/YAML/CSV
    (de)serialization" from Phase 2 actually needed at the language
    level; XML specifically is weaker-fit for serde's derive model and
    isn't demonstrated yet. A project with even one `litter`/`breed` now
    needs `serde` (`features = ["derive"]`) as a real Cargo dependency.
    See [LANGUAGE.md § Litters](LANGUAGE.md#litters).
  - **A new `&expr` reference operator** — Kittine had no way to spell a
    Rust reference at all before this, which silently blocked calling
    *any* real Rust/crate function that takes one (`serde_json::to_string(
    &value)` being the immediate case, but this is a general interop gap,
    not a JSON-specific one — most of the wider Rust ecosystem takes
    references somewhere). `&` binds like unary `-`; referencing a
    non-`Copy` scope-tracked value still gets the usual pre-clone
    treatment first. See [LANGUAGE.md § Reference
    operator](LANGUAGE.md#reference-operator).
  - **`warn<expr>`/`error<expr>`** join `craft<expr>` as two more levels
    of the same statement, mapping to `leptos::logging::warn!`/`error!`
    instead of `log!` — real severity levels a browser's devtools console
    distinguishes, which `craft<...>` alone couldn't produce. See
    [LANGUAGE.md § Printing](LANGUAGE.md#printing-craft--warn--error).
  - Verified with 10 new compiler tests (150 total, up from 140) and a
    real `cargo check` against Leptos 0.7 for every piece —
    `example-app`'s `Home.kitty` gained a `stash`-typed `prices` signal
    displayed reactively plus real `warn</error<` calls, and `Shapes.kitty`
    gained a `serde_json::to_string(&origin)` call serializing its
    existing `Point` litter to JSON and displaying the result. A real
    parser ambiguity was found and fixed along the way, not just assumed
    away: a return-type's `#n{}` map suffix and a `purr`'s own following
    body brace are both bare `{`/`}` pairs back to back
    (`purr f() #n{} { .. }`) — `Parser::parse_signature_type` now needs a
    3-token lookahead specifically in return-type position (a real 2nd
    brace pair must follow) to tell them apart; verified with a dedicated
    regression test plus a real compile of the previously-broken shape.
  - **Left deliberately undemonstrated, not blocked at the language
    level** (already callable via existing path-qualified-call/method-call
    interop, `&` where a function needs a reference — just not exercised
    with a real dependency + example in this round, to keep this round's
    Cargo-dependency surface area from growing further than the `serde`
    addition already above): environment/config access
    (`std::env::var(..)`, server-side only), file I/O (`std::fs::..`,
    also server-side only — `example-app` is CSR/WASM, which has no
    filesystem), and encryption/hashing primitives (any hashing crate,
    called the same way `serde_json` now is).
  - **Still genuinely blocked, not just undemonstrated** — an HTTP client
    and Leptos `Resource`-based data fetching both need real `async`/
    `await` support and lambda/closure-argument syntax, neither of which
    exist in Kittine yet; see the dynamic-WASM-module-loading gap logged
    below, a related but distinct blocker. Validation and
    string/date/number formatting utilities beyond what `MethodCall`
    interop already reaches are still open, lower-priority Phase 2 items.
  Landed 2026-07-22.
- **Codegen targets real Leptos 0.7** — every language feature above
  has been round-tripped through `cargo check`/`cargo build` against the
  actual `leptos` crate, not just asserted against generated-string
  snapshots, under **both** CSR/hydrate and SSR feature configurations.
  Routing was additionally driven end-to-end with Playwright against a
  real running dev server (navigation, the 404 fallback, and continued
  reactivity all actually observed, not just compiled). A non-`Copy`
  value (a `Word`/array prop, a `spin` loop variable, or a `hold`-bound
  local) read from more than one reactive closure in the same component
  now correctly compiles too — a real move-conflict bug found by actually
  compiling that exact pattern, not assumed away by the earlier "always
  `.clone()`" fix.
- **Tooling**: `kittine-compiler` CLI (build/fmt/lint), a Vite plugin
  driving the full compiler → cargo → wasm-bindgen pipeline, a VS Code
  extension (TextMate grammar only — no language server), Vercel
  deployment config. `kittine-compiler build` only rewrites a `.rs` file
  when its generated content actually changed, so unrelated dependencies
  keep their mtime and downstream `cargo`/`wasm-bindgen`/Vite freshness
  checks correctly skip redoing work — verified with a real `npm run
  build` (~31s → ~5s for a no-op rebuild of `example-app`).
  `kittine-compiler fmt <file-or-dir>` (`--check`, `--force`) is a real
  formatter: an AST pretty-printer that self-verifies every reformat by
  reparsing its own output and refusing to write unless the resulting AST
  is exactly equal to the original (not a snapshot/assumption — a real
  `PartialEq` check over the whole tree), and separately refuses to touch
  a file containing `//` comments unless `--force`, since the lexer
  discards comments before the parser ever sees them and fmt has no way
  to preserve them. `kittine-compiler lint <file-or-dir>` catches unused
  imports/`private` items/params/`hold` bindings and duplicate field/
  variant/method/param names (the latter a real `cargo build` error,
  e.g. E0124, caught before ever reaching `cargo`) — see [LANGUAGE.md §
  Known limitations](LANGUAGE.md#known-limitations) for what it doesn't
  catch (no line numbers, since `Stmt`/`Expr` carry no source position;
  no reachability analysis, so purely-self-recursive dead code can slip
  through). Both landed together and are exercised by `example-app`/
  `example-ssr`'s real `.kitty` files: reformatting all of them with fmt
  regenerated byte-identical `.rs` output (proof the rewrite changed
  nothing semantically, not just an assertion), and lint found and fixed
  one real pre-existing issue (a false-flag on `components.kitty`'s
  barrel-file `export import`s, fixed by exempting re-exports from the
  unused-import check — barrel files are supposed to go locally unused).
  **A real package manager now exists**: `kittine.toml`/`kittine.lock`
  (a manifest + a checksummed lockfile), `kittine-compiler add`/
  `install`/`publish`, and a real hosted registry backing them —
  [`buildsbybuchanan/kittine-registry`](https://github.com/buildsbybuchanan/kittine-registry),
  a public repo with no server of its own: one `index/<name>.json` per
  package (versions, tarball URLs, sha256 checksums) fetched over plain
  HTTPS, with tarballs hosted as GitHub Release assets. `install`
  downloads and sha256-verifies every dependency into `kitten_modules/`;
  a bare-name import (`import { X } from 'some-package'`, no `./` prefix)
  resolves to `kitten_modules/some-package/lib.kitty` via the same
  upward-search `node_modules` uses. `publish` (maintainer-only, needs
  the `gh` CLI) packs a package directory into a tarball and uploads it.
  Verified for real, not just unit-tested: a real package
  (`kittine-strings`, a small `purr shout(text)`) was published to the
  live registry, then `add`+`install`+`build`'d fresh in a separate
  directory, and the generated Rust — including the cross-package
  `#[path]`/`use` wiring — was checked with a real `cargo check` against
  actual Leptos 0.7. See [CLI.md § The package
  registry](CLI.md#the-package-registry), [LANGUAGE.md § Package
  imports](LANGUAGE.md#package-imports). What's still scoped out: exact-
  version-only dependency requirements (no semver ranges), and no
  dependency-of-a-dependency resolution yet (a published package can't
  itself declare `kittine.toml` dependencies that `install` follows
  transitively) — see [Full vision § Phase
  5](#phase-5--package-ecosystem--tooling-depth) for what's still open
  there (publishing UX beyond the CLI, workspaces).
  Separately, `kittine-compiler` now also builds to WebAssembly
  (`crate-type = ["cdylib", "rlib"]`, a `wasm-bindgen`-exported
  `compile_kitty_single_file`) — the single-file lex/parse/codegen path
  has no filesystem dependency, so it compiles cleanly to
  `wasm32-unknown-unknown` and runs in a browser with zero server
  involvement. Verified for real: built with the exact `wasm-bindgen`
  version already pinned elsewhere in this ecosystem, then round-tripped
  through real generated JS glue under Node (both the success path and a
  real parse-error path). This is what `kittine-website`'s new
  `/playground` page runs on — see [The Kittine
  website](#the-kittine-website).

**Can it build a full website yet? Yes — the language and rendering gaps
that were blocking it are both closed.** A real multi-page app — composed
components, typed props, list rendering, children, shared logic, genuine
client-side routing with a 404 fallback — all work and are verified
end-to-end. The remaining "website, not just app" gap (SSR/SSG for real
first paint and SEO) is now real too, via `example-ssr/` — see
[SSR.md](SSR.md). See [Production readiness](#production-readiness) for
the broader "is this ready to build real things on" answer (still "not
yet," for reasons unrelated to rendering — see below), and [The Kittine
website](#the-kittine-website) for what this means for actually building
kittine.dev.

See [LANGUAGE.md § Known limitations](LANGUAGE.md#known-limitations) for
the precise, current boundary of what's supported — that section is the
authoritative day-to-day list; this file is about direction, not spec.

## Production readiness

**Not yet.** This is answered honestly here every time something changes
— per standing instruction, not a one-time verdict. Kittine is a real,
tested compiler (162+ tests, every feature round-tripped against actual
Leptos under both CSR/hydrate and SSR configurations, routing (including
dynamic segments) driven end-to-end in a real browser) with a language
core solid enough for a genuine multi-page site, CSR or server-rendered.
That's real progress, not the same thing as production-ready. What's
still missing:

1. **Error handling is narrower than "no story" now, but still real.**
   `breed Result { Ok(#t), Err(#w) }`-shaped types and `pounce>` branching
   on them both work today (see [LANGUAGE.md §
   Breeds](LANGUAGE.md#breeds), [§ Pattern
   matching](LANGUAGE.md#pattern-matching)) — a `.kitty` author *can*
   model and react to failure now, **and can unwrap a `Result` and
   `return` the unwrapped value in one expression** — `pounce>` works as
   an expression now, not just a statement (see [LANGUAGE.md § `pounce>`
   as an expression](LANGUAGE.md#pounce-as-an-expression)) — closing the
   specific gap this item used to name. What's still missing: an
   *un*-modeled runtime panic (an actual Rust `panic!`, not a
   `breed`-modeled failure) is still a hard crash reported only in the
   browser devtools console; and a `pounce>` expression's bare-string-
   literal-arm coercion (needed so a `match`'s arms all agree on a type)
   only covers a `purr`'s own `return (...)` value today, not a `hold`/
   signal value — see [LANGUAGE.md § Known
   limitations](LANGUAGE.md#known-limitations).
2. **No way for a Kittine *program* to have its own tests.** The compiler
   is well-tested; a person writing `.kitty` files has no test runner
   surfaced to them at all.
3. **No versioned releases.** Kittine itself has no v1.0/semver — several
   breaking syntax changes have already happened in rapid succession
   (removing `¨...¨`, adding four new constructs) with no migration story
   or deprecation window, because there's no released version to be
   compatible *with* yet.
4. **No dedicated security review.** Kittine source becomes Rust source;
   the string-escaping logic (`escape_str` in `codegen.rs`) that stands
   between a `.kitty` string literal and a generated Rust string literal
   hasn't had a focused audit for injection-style edge cases.
5. **Tooling stops at "does it compile."** The VS Code extension is
   TextMate-only — no diagnostics, no autocomplete, no go-to-definition —
   so mistakes surface at `kittine-compiler build` time, not while typing.

None of these are hard blockers to *experimenting* with Kittine or
building the planned example site — they're what stands between "this
works" and "I'd stake a real product on this." Phase 1 (language
completeness — done as of 2026-07-19, with `pounce>`-as-expression the
one tracked exception feeding directly into gap 1 above) and Phase 6
(security review, semver, grammar freeze) in [Full
vision](#full-vision-phased-honest) are where these get addressed.

## Next up

- **No way to load or call into a separately-built WebAssembly module at
  runtime.** Discovered re-checking `kittine-website`'s `/playground` page
  after landing `event`-value reading (see the `Done` item just below):
  the page's compile button still needs hand-written JavaScript, and the
  reason isn't the event-reading gap anymore — it's that `playground.js`
  does a dynamic `import("/playground/kittine_compiler.js")` of
  `kittine-compiler`'s own `wasm-bindgen` output, then calls the exported
  `compile_kitty_single_file` function and catches whatever JS exception
  it throws on a parse error. None of that has a Kittine equivalent: no
  dynamic-import syntax, no way to call an arbitrary already-loaded JS
  function reference (only a same-file `purr`/an imported Kittine item/a
  known Rust path via `Type::method()`), and no `try`/`catch`-equivalent
  for a thrown JS exception (`pounce>` matches a Kittine `breed`, not a
  JS `Error`). A real fix needs some kind of JS-interop story, not just a
  parser tweak — likely its own scoped design, not a quick follow-on to
  `event`.
Done: ~~No array-of-`litter`/`breed` types~~ and ~~no lambda/closure-
argument syntax for a filter predicate~~ (array-typed props/returns are no
longer scalar-only — a bare `litter`/`breed` name, optionally
`[]`-suffixed, is now a real `func`/`purr` param/return type, e.g.
`purr matching(DocEntry[] entries, query) DocEntry[] { .. }`; and a
closure literal, `|param, ..| expr`, lowers verbatim to a Rust closure —
`.filter(|e| e.title.contains(&query))` now parses and generates real,
correct Rust. See [LANGUAGE.md § A litter/breed name as a prop or purr
param/return type](LANGUAGE.md#a-litterbreed-name-as-a-prop-or-purr-paramreturn-type),
[§ Closures](LANGUAGE.md#closures). Two real bugs surfaced and fixed along
the way, found by actually wiring both features into `example-app`'s real
`Shapes.kitty`, not assumed: a `spin`'s default reactive-list key
(`format!("{item}")`) required `Display`, which a generated `litter`/
`breed` never derives (only `Debug`) — switched to `format!("{item:?}")`
universally, working for every element type at the cost of a quoted key
string (harmless — a `<For>` key only needs to be unique and stable, never
user-visible); and a `pounce>`-as-expression (see the entry below) mixing
a computed arm with a bare string-literal arm in a `Word`-returning
`purr` produced a real `match`-arm type mismatch (`E0308`), fixed by
extending the existing bare-literal-owning coercion to look inside a
`pounce>` return value's arms, not just the return value itself. 12 new
compiler tests (162 total); verified with a real `cargo check` and
`npm run build` against Leptos. **Not yet applied to `kittine-website`'s
`/docs/language` topic-filter search box**, the original motivating
case — tracked separately below, since that's a website change, not a
language one.
- **`kittine-website`'s `/docs/language` topic-filter search box is still
  static hardcoded JSX**, not the real data-backed free-text search the
  two language features above were built to unblock. The language-side
  gap is closed; wiring it into the actual production docs page is a
  distinct, tracked follow-on.

Done: ~~`pounce>` is statement-only~~ (it now also works as an
**expression** — `pounce> subject Variant(binding)? >> expr .. else>
expr`, reachable anywhere an expression is expected — a `purr`'s `return
(...)`, a `hold` binding's value, a call argument, not just the start of
a statement. Closes the specific, long-tracked gap in Kittine's error-
handling story: a function can now unwrap a `breed Result { Ok(#t),
Err(#w) }`-shaped value and `return` the payload in one step, the way
Rust's own `match`/`?` can — see [LANGUAGE.md § `pounce>` as an
expression](LANGUAGE.md#pounce-as-an-expression). Same column-indented
arm grammar as the statement form; the formatter's self-verifying
round-trip check needed a real fix too, since the printer has no way to
know what column its caller will place a `pounce>` at — solved by always
printing it starting on a fresh line at column 1, which the column-
sensitive grammar tolerates regardless of where it's nested) — landed
2026-07-23, alongside the array-of-`litter`/closures entry above (same
round of work).

Done: ~~No way to read an `on<Event>` handler's event value~~ (the
reserved `event` identifier inside an `on<Event>` handler's own expression
now reads `event_target_value(&ev)`, and the handler's closure binds `ev`
instead of discarding it as `_` — see [Status § the new
bullet](#status-what-works-today) for the full description) — landed
2026-07-22. **Not yet applied to `kittine-website`'s `/docs/language`
topic-filter search box**, the original motivating case: that page's
cards are static hardcoded JSX, and turning them into a real free-text
search needs the cards to be *data* (an array of struct-like entries)
filtered reactively — Kittine's array types are scalar-only today
(`#n[]`/`#w[]`/`#f[]`, no array-of-`litter`), so there's nowhere to put a
`Vec<DocEntry>` prop/return yet. Logged as a new, distinct gap below
rather than force a half-built version onto the production docs page.

Done: ~~A formatter and linter~~ (`kittine-compiler fmt`/`lint`, closing
that specific Phase 5 item — see [Status § Tooling](#status-what-works-today)
for the full description). `fmt` is a hand-written AST pretty-printer that
self-verifies every reformat (reparses its own output, refuses to write
unless the resulting AST is exactly `PartialEq`-equal to the original —
not an assumption) and refuses to touch a file with `//` comments unless
`--force`, since the lexer discards them before parsing and there's
nothing for fmt to preserve. `lint` catches unused imports/`private`
items/params/`hold` bindings and duplicate field/variant/method/param
names — the last a real `cargo build` error (E0124) once generated,
caught here first. 34 new compiler tests (132 total); verified for real
by reformatting every `.kitty` file in `example-app`/`example-ssr` and
recompiling — the regenerated `.rs` output is byte-identical to what was
committed before, proof (not just an internal assertion) that the rewrite
changed nothing semantically. `lint` also caught one genuine
pre-existing false positive against itself during that same real run:
`components.kitty`'s barrel-file `export import`s were flagged "unused,"
which is wrong — a re-export's whole point is to go unused *locally* —
fixed by exempting `is_export` imports from that check. A dedicated
package manager remains not built; Kittine still leans on Cargo for
dependencies — landed 2026-07-20.

Done: ~~`kittine-compiler` provisioning for Vercel builds (kittine-ide)~~
(`kittine-ide/scripts/vercel-build.sh` builds `kittine-compiler` from the
`vendor/kittine` submodule and puts it on `PATH` before the Vite build
runs — already done as of this check, this item was just stale) —
confirmed 2026-07-19. ~~Real build-speed fix for `cargo-leptos`/Vercel
projects~~ (a `framework: null` Vercel project gets no persistent cache
for `~/.cargo`/`target/` between deploys — measured, not assumed: a real
cold `kittine-website` deploy took 6m22s, ~4m45s of it pure dependency
recompilation, unaffected by `[profile.release]` tuning since the cost is
in *other* crates. Real fix: build once in GitHub Actions, where
`actions/cache` genuinely persists across runs, then ship the result to
Vercel as a prebuilt deployment (`vercel deploy --prebuilt`) so Vercel
does no compilation at all — see [DEPLOYMENT.md](DEPLOYMENT.md) for the
full pattern, `kittine-website/.github/workflows/deploy.yml` for the
working implementation. Also landed: a one-local-command wrapper script
collapsing the `.kitty`-compile + `cargo-leptos` two-step into one,
matching the single-command ergonomics `vite-plugin-kittine`'s CSR path
already had; a compile-speed-favoring `[profile.release]`
(`opt-level = 1, lto = false`, matching `kittine-ide`'s already-proven
approach) which surfaced and fixed a real latent bug — a clean release
build hitting rustc's default query-recursion limit on the site's
real, deeply-nested view tree, fixed with `#![recursion_limit = "256"]`)
— landed 2026-07-19. ~~Ecosystem docs (`kittine-ide`) kept in sync~~
(`vendor/kittine` submodule bumped to latest `main`; `ide-app/Cargo.toml`
was missing `leptos_meta` as a dependency, same class of gap already
fixed in `example-app` — added and verified with a real `cargo check` +
full `npm run build`; `README.md`'s language cheat-sheet
updated off the retired `<<Type>>` syntax and "no structs/generics"
claims, both wrong since earlier today) — landed 2026-07-19.

Done: ~~Traits (`claw`), trait implementations (`bare .. for ..`), bounded
generics~~ (closes Phase 1's "a real type system" gap — see
[LANGUAGE.md § Claws](LANGUAGE.md#claws), [§
Generics](LANGUAGE.md#generics). `claw Name { method(params) type, .. }`
declares a trait; `bare Claw for Target { purr method(..) .. }`
implements it, reusing ordinary `purr` codegen with an implicit `self`;
`litter`/`breed`'s existing generic type parameter can now be bounded by
a `claw` (`<#t: Named>`), a real Rust trait bound Rust's own compiler
enforces. Also fixed a real pre-existing bug found along the way: a
`Word`-returning `purr`/method with a bare string-literal `return`
rendered an uncoerced `&str` (`E0308` against real rustc) — now correctly
owned. Verified with 6 new compiler tests (97 total) and a real `cargo
check`/`npm run build` against Leptos 0.7 — `example-app`'s
`Shapes.kitty` gained a `Named` claw implemented for `Point` and a
bounded `NamedHolder<#t: Named>`) — landed 2026-07-19. This closes Phase
1 in full (module visibility was judged already-closed — see the Phase 1
section below); the one documented exception is `pounce>` staying
statement-only, tracked as [Production readiness](#production-readiness)
gap 1, not a Phase 1 item.

Done: ~~Structs (`litter`), enums (`breed`), pattern matching (`pounce>`),
minimal generics groundwork~~ (Phase 1 language-completeness work — see
[LANGUAGE.md § Litters](LANGUAGE.md#litters), [§
Breeds](LANGUAGE.md#breeds), [§ Pattern
matching](LANGUAGE.md#pattern-matching), [§
Generics](LANGUAGE.md#generics), and the CHANGELOG entry for the full
design. A `litter`/`breed` may carry at most one unbounded type parameter;
a `breed` variant carries at most one payload value; `pounce>` is
statement-only (can't yet compute a value for a `return`). Verified with 6
new compiler tests (91 total) and a real `cargo check`/`npm run build`
against Leptos 0.7 — `example-app` gained a real `Shapes.kitty` composing
all four features together, wired into `Home.kitty`) — landed 2026-07-19.
~~A formal grammar spec document~~ ([GRAMMAR.md](GRAMMAR.md), the complete
EBNF grammar derived from the lexer/parser source, not written
aspirationally — closes that specific Phase 1 item; [LANGUAGE.md § Full
grammar summary](LANGUAGE.md#full-grammar-summary) stays as a shorter
quick-reference version) — landed 2026-07-19. ~~Scalar type inference for
props/`purr` params and return types~~
(`purr greet(name) { return ('Hello, ' + name) }` needs no `#w` tag at all
— a new post-parse `infer` pass derives `Word`/`Num`/`Flag` from how the
name is used in the body, local to that one function/component, scalars
only; an explicit tag still always wins. Verified with 7 new tests and a
real `cargo check`, and by regenerating `example-app`'s `Greetings.kitty`/
`Home.kitty` with their tags dropped — byte-identical `.rs` output to the
explicitly-tagged version. See [LANGUAGE.md § Type
inference](LANGUAGE.md#type-inference)) — landed 2026-07-19 on the
`syntax` branch. ~~Kebab-case JSX attributes~~ (`data-*`/`aria-*` parse as a single
attribute name, not identifier-minus-identifier; found while building the
real marketing site's `data-kittine-component` inspect-mode fingerprint)
and ~~`leptos_meta` in scope~~ (unconditional `use leptos_meta::*;` in
every generated file, `example-app/Cargo.toml` gained the dependency it
was missing; preparatory — no page uses `<Title>`/`<Meta>` yet, since that
also needs `provide_meta_context()` wiring this repo hasn't documented) —
landed 2026-07-19. ~~Type-tag sigil redesign~~ (`<<Num>>`/`<<Word>>`/`<<Flag>>` and
their `[]` array forms retired in favor of `#n`/`#w`/`#f`/`#n[]`/`#w[]`/`#f[]`
— a breaking change, no closing delimiter needed, and shorter than every
Rust type they stand for: `#n` (2 chars) vs `f64` (3), `#w` (2) vs
`String` (6), `#f` (2) vs `bool` (4), array forms similarly. Motivated by
the standing "always shorter than the Rust it generates" design rule —
see [LANGUAGE.md § Brevity by design](LANGUAGE.md#brevity-by-design) —
which the old bracket-wrapped form violated for every scalar type. All 76
compiler tests, every `example-app`/`example-ssr` `.kitty` source file,
the VS Code grammar, and the docs were updated together; regenerating the
committed `.rs` output changed zero bytes, confirming the tag is purely
front-end sugar) — landed 2026-07-19. ~~List rendering in views~~ (`spin` in `return ( ... )` → Leptos
`<For>`) — landed 2026-07-18. ~~Component children~~ (untyped `children`
param + `children()`) — landed 2026-07-18. ~~Routing~~ (`leptos_router`
composed with zero new syntax) — landed 2026-07-18. ~~Comparison
operators~~ (`<`/`<=`/`>`/`>=`/`!=`, usable generally not just in
conditions) — landed 2026-07-18. ~~Array-typed props/returns~~
(`#n[]`/`#w[]`/`#f[]`) — landed 2026-07-18. ~~`export`/
visibility control~~ (`private func`/`purr`, enforced by Rust's own
privacy rules) — landed 2026-07-18. ~~Logical `&&`/`||`~~ (combine
comparisons into one condition, usable generally not just in
conditions) — landed 2026-07-18. ~~Incremental/cached builds for the
import graph~~ (`.rs` files are only rewritten when their content
actually changed) — landed 2026-07-18. ~~Same-file string-literal ->
`Word` `purr` parameter~~ (the compiler now knows a same-file callee's
signature) — landed 2026-07-18. ~~Dynamic-segment routes, demonstrated~~
(method calls, a tuple path, and `leptos_router::hooks` in scope,
verified end-to-end with Playwright) — landed 2026-07-18. ~~Cross-file
string-literal -> `Word` `purr` parameter~~ (`kittine-compiler build`
now collects every reachable file's `purr` signatures before generating
any single file's code, so an imported callee gets the same coercion a
same-file one already had) — landed 2026-07-18. ~~`key` control for
view-position `spin`~~ (an optional `key(expr)` clause overrides the
default `format!("{item}")` key; verified against real Leptos and wired
into `example-app`'s `NavList.kitty`) — landed 2026-07-18. ~~Path-qualified
expressions~~ (`Type::method()`, `Type::CONST`, multi-segment paths) and
~~programmatic navigation, demonstrated~~ (`use_navigate()` fully works
now — verified against a real running dev server with Playwright, which
also caught a real runtime-only gotcha: the hook must be called eagerly,
not from inside the event handler, or it panics) — landed 2026-07-18.
~~Re-exports~~ (`export import { Name } from './file.kitty'` lowers to
`pub use`, letting a third file import through an intermediate one;
verified with a real three-file chain against actual Leptos and wired
into `example-app` as a `components.kitty` barrel file) — landed
2026-07-18. ~~Plain (non-reactive) local-variable binding~~ (`hold name
>> expr` lowers to a bare `let`; replaces the signal-based workaround
programmatic navigation had used, wired into `example-app`'s
`User.kitty`) — landed 2026-07-18. ~~Move-conflict fix for a non-`Copy`
value read from more than one reactive closure~~ (a real bug found by
actually compiling that exact pattern against Leptos, not assumed —
every non-`Copy` scope-tracked read now pre-clones into its own local
before its closure) — landed 2026-07-18. ~~Word-signal mutation to a
brand-new literal~~ (`<{label}> >> 'reset'` now owns the string, same
class of fix as the `Word`-parameter string-literal work; wired into
`example-app`'s `Home.kitty` as a "Reset to Guest" button) — landed
2026-07-18. ~~SSR/SSG~~ (real server-side rendering via `cargo-leptos` +
Axum — a second toolchain alongside Vite, not a replacement for it —
verified with real `curl` output and Playwright-confirmed hydration +
client-side routing in `example-ssr/`; `kittine-compiler` needed zero
changes, since CSR vs. SSR is entirely a downstream Cargo-feature
decision, not anything Kittine's codegen is aware of; see
[SSR.md](SSR.md)) — landed 2026-07-18.

## Full vision (phased, honest)

This is the "production-ready ecosystem" scope: everything a developer
would eventually expect to build **business websites, e-commerce, CMS/CRM/
ERP systems, SaaS platforms, REST/GraphQL APIs, desktop apps, CLIs, and
enterprise software** in Kittine alone. None of the phases below are
started unless explicitly listed under [Status](#status-what-works-today)
above. Writing full docs/tutorials for anything in this list before it's
real would mean documenting features that don't exist — so this section
stays a plan, not a spec, until an item graduates into `LANGUAGE.md`.

### Phase 1 — Language completeness (extends [Next up](#next-up)) — **done, with one documented exception**

Every item originally listed under this phase has landed, as of
2026-07-19:

- ~~Structs/records~~ (`litter`) — [Litters](LANGUAGE.md#litters).
- ~~Enums~~ (`breed`) — [Breeds](LANGUAGE.md#breeds).
- ~~Pattern matching~~ (`pounce>`) — [Pattern
  matching](LANGUAGE.md#pattern-matching).
- ~~Generics groundwork~~ — one type parameter per `litter`/`breed`,
  optionally bounded by a `claw` — [Generics](LANGUAGE.md#generics).
- ~~A real type system beyond the current three scalar tags~~ —
  same-function-body inference for `Num`/`Word`/`Flag` ([Type
  inference](LANGUAGE.md#type-inference)), `litter`/`breed` as
  user-declared structural types, and a full trait system (`claw`/`bare
  .. for ..`) for shared behavior/capability contracts across them —
  [Claws](LANGUAGE.md#claws). Two intentional scope limits remain, not
  bugs: type inference doesn't propagate across function calls or into
  array element types, and generics stay single-parameter (no bounds
  beyond one `claw`, no generic `purr`/`func`).
- ~~Module visibility~~ — judged already closed rather than extended:
  `private`/importable-by-default is now uniform across every
  declaration kind (`func`, `purr`, `litter`, `breed`, `claw`), and
  Kittine's compilation model (every file folds into one flat Rust
  module tree for a single app) has no real crate/package boundary for
  `pub(crate)`-style granularity to actually mean anything. Revisit if a
  real multi-package use case ever demonstrates otherwise.
- ~~A formal grammar spec document~~ — [GRAMMAR.md](GRAMMAR.md).

**The one deliberate exception**: `pounce>` is still statement-only — a
`purr` can't compute and `return` a value depending on which `breed`
variant matched, the way Rust's own `match`/`?` can. This is tracked as
[Production readiness](#production-readiness) gap 1, not re-listed as an
open Phase 1 item, because it's a narrow, well-scoped, already-documented
gap rather than an open-ended one — closing it is a natural next
increment, not a blocker to calling the rest of this phase done.

### Phase 2 — Standard library — **in progress**

~~Collections beyond arrays~~ (`stash{ .. }`, a real `String`-keyed map),
~~JSON (de)serialization~~ (every `litter`/`breed` derives
`serde::Serialize`/`Deserialize` — YAML/CSV covered by the same derive,
just undemonstrated; XML is a weaker fit for serde's derive model), and
~~logging~~ (`warn<...>`/`error<...>` join `craft<...>`) are done — see
[Status](#status-what-works-today) for the full description. File I/O and
environment/config access are callable today via existing path-qualified-
call interop (server-side only — no filesystem/env access from CSR/WASM)
but have no real example wired up yet. Still fully open: an HTTP client
(genuinely blocked — needs `async`/`await` and lambda/closure-argument
syntax, neither of which exist yet), string/date/number formatting
utilities beyond what already-existing `MethodCall` interop reaches, and
validation.

### Phase 3 — Backend & data

An HTTP server + middleware story (client-side *routing* is already done —
see [Status](#status-what-works-today)); database connectivity (SQLite
→ Postgres/MySQL/MSSQL), a query builder or ORM, migrations, sessions/
cookies/JWT/OAuth, background jobs/queues/scheduling, caching (Redis-style),
rate limiting, WebSockets/SSE.

### Phase 4 — Full web framework

~~SSR/SSG~~ (done — see [Status](#status-what-works-today), [SSR.md](SSR.md)),
form handling + validation, file uploads, image optimization, CSRF/security
headers, SEO/metadata management, streaming responses. A "backend
framework" and "frontend framework" in the vision doc are really this phase
plus Phase 3, built on the same compiler rather than as separate products.

### Phase 5 — Package ecosystem & tooling depth

~~A formatter and linter~~ (done — `kittine-compiler fmt`/`lint`, see
[Status § Tooling](#status-what-works-today)). ~~A real package registry,
publishing, dependency resolution and lock files~~ (done —
`kittine.toml`/`kittine.lock`, `add`/`install`/`publish`, and the hosted
`kittine-registry`, see [Status § Tooling](#status-what-works-today)).
Still open: workspaces (a single `kittine.toml` covering multiple local
packages), semver *ranges* for dependency requirements (exact-version-only
today), transitive dependency resolution (a published package's own
dependencies aren't followed yet), a production Tree-sitter grammar; a
minimal-but-real Language Server Protocol
implementation (diagnostics, hover, go-to-definition, rename) — `lint`'s
CLI diagnostics are a real step toward this but aren't an LSP, since they
carry no source position and don't run inside an editor; a REPL and
debugger. **VS Code Marketplace publication — prepped, not yet actually
published**: `vscode-kittine/package.json` had a real bug fixed (its
`repository.url` pointed at a personal fork that no longer matches
`origin`, `siv-the-programmer/kittine`, instead of the real public repo,
`buildsbybuchanan/kittine` — would have broken the Marketplace listing's
repo link and `vsce`'s automatic relative-link rewriting in the README),
the grammar (`syntaxes/kittine.tmLanguage.json`) was updated for every
construct that's landed since the last extension version (`stash{...}`,
`warn<...>`/`error<...>`, the `&` reference operator, the reserved
`event` identifier), version bumped to 0.5.0, and a real `.vsix` was
packaged with `vsce package` (clean, no warnings) — confirmed via `vsce
show buildsbybuchanan.kittine-lang` that no version has ever actually been
published under this name yet. What's left needs the user, not more
compiler/tooling work: a Visual Studio Marketplace publisher account for
`buildsbybuchanan` and an Azure DevOps Personal Access Token (Marketplace:
Manage scope) to actually run `vsce publish` — neither is something an
agent can create on someone's behalf.

**Explicitly not pursuing right now: GitHub Linguist submission.**
Checked Linguist's actual current contribution requirements directly
(not from memory) — they require at least 2000 indexed files using the
`.kitty` extension across a reasonable spread of distinct public
repositories before a new-language PR will even be reviewed, and will
close PRs for languages that don't clear that bar. Kittine exists in one
private repo today, which doesn't just fall short of that — it isn't
visible to the assessment at all (private repos don't show up in the
public GitHub Code Search Linguist's maintainers use). Getting `.kitty`
recognized as "Kittine" with its own color on GitHub.com requires real
public adoption first; revisit this once that's true, not before.

### Phase 6 — Production hardening

Benchmarking, profiling, monitoring/metrics hooks, security-review pass
across the whole toolchain, semantic versioning + release process, a
stable v1.0 grammar freeze.

## The Kittine website

The plan (per explicit instruction) is to build **buildsbybuchanan-style
kittine.dev in Kittine itself**. Every technical gap that was blocking
this is now closed: imports ✅, props ✅, list rendering ✅, component
children ✅, routing (including dynamic segments and programmatic
navigation) ✅, array-typed props/returns ✅ (nav/card data can be real
arrays, not just hardcoded literals), and SSR/SSG ✅ (real first paint +
SEO, via `example-ssr`'s `cargo-leptos` + Axum setup — see
[SSR.md](SSR.md)). [Production readiness](#production-readiness) still
lists real, honest gaps (error handling, no program-level test runner, no
versioned releases, no security review, no LSP) — none of them are
rendering/language gaps, and none of them block *starting* the site, but
they're worth knowing about before treating anything built on Kittine as
hardened.

**Update: the site has since been built** — it lives in its own
[`kittine-website`](https://github.com/buildsbybuchanan/kittine-website)
repo (not this one), went with SSR via `cargo-leptos` (real first paint +
SEO), and deploys to Vercel as a static export produced by real GitHub
Actions builds (see that repo's `docs`/`README.md` for specifics —
this section stays here for the historical CSR-vs-SSR reasoning, not as
a still-open decision). It vendors this repo as a `vendor/kittine` git
submodule for two things: `kittine-compiler` itself (dev-time `.kitty` ->
`.rs` compilation, checked-in output, same as `example-app`), and — new —
a `wasm-bindgen` build of this crate's `compile_kitty_single_file` export
powering `kittine-website`'s `/playground` page (compile a `.kitty`
snippet to Rust entirely client-side; see [Status § Tooling](#status-what-works-today)
for the WASM export itself). The website's own Ecosystem/Roadmap pages
track its own status, not duplicated here.
