# Changelog

All notable changes to Kittine are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Kittine does not yet follow Semantic Versioning tags/releases — entries are
grouped by date until the first tagged release.

## [Unreleased] - 2026-08-05

### Added (a real `Date` type: `now>` / `#d` / `.formatted` / `.toDate`)

- **`now>`, `#d`, `.formatted(pattern)`, `.toDate(pattern)`** — a real
  `Date` type, closing the date/time half of the "string/date/number
  formatting utilities" gap the 2026-08-01 entry below deliberately left
  open (it needed an actual type, not just more number formatting). `now>`
  is the literal (same `>`-suffixed shape as `yes>`/`no>`), lowering to
  `chrono::Utc::now()`; `#d` is the type tag, the same two-character-sigil
  convention `#n`/`#w`/`#f` already use (`#d[]`/`#d{}` work too);
  `.formatted`/`.toDate` are two more reserved method names (same
  precedent as `.fixed`/`.padded`/`.grouped`), lowering to
  `receiver.format(&(pattern)).to_string()` and
  `chrono::NaiveDateTime::parse_from_str(&(receiver), &(pattern)).unwrap()
  .and_utc()` respectively. A project with a `Date` value needs `chrono`
  as a real Cargo dependency — `features = ["wasmbind"]` specifically on a
  CSR/WASM target, since `chrono::Utc::now()` otherwise compiles fine but
  panics at runtime in the browser (`std::time::SystemTime::now()` has no
  `wasm32-unknown-unknown` implementation without it). `Date` is `Copy`
  (`chrono::DateTime<Utc>`'s `Offset` type, `Utc`, is a zero-sized `Copy`
  unit struct), so it joins `Num`/`Flag` rather than getting the
  `Word`/`litter`/`breed` pre-clone treatment. `.toDate` needs a full
  date+time pattern, not date-only (inherited from
  `chrono::NaiveDateTime::parse_from_str` itself, which has no date-only
  parse path), and its `.unwrap()` panics on a malformed input rather than
  returning a `Result` — the same "no un-modeled-failure story yet"
  limitation every other interop escape hatch already has. See
  [LANGUAGE.md § Date and time](docs/LANGUAGE.md#date-and-time).
  Verified with 7 new compiler tests (174 total, up from 167) and a real
  `cargo check --target wasm32-unknown-unknown` against Leptos 0.7 —
  `example-app`'s `Home.kitty` gained a `joined` signal (`now>`, displayed
  via `.formatted("%Y-%m-%d")`) and a `launchDay` signal (a fixed date
  parsed from a string literal via `.toDate(...)`, displayed via
  `.formatted("%B %d, %Y")`), both rendered live in the view.

## [Unreleased] - 2026-08-01

### Added (number formatting: `.fixed` / `.padded` / `.grouped`)

- **`.fixed(precision)`, `.padded(width)`, `.grouped()`** — three reserved
  method names closing the first slice of the "string/date/number
  formatting utilities beyond what `MethodCall` interop already reaches"
  gap logged in [ROADMAP.md § Phase 2](docs/ROADMAP.md#phase-2--standard-library).
  Real Rust has no `.fixed()`/`.padded()`/`.grouped()` inherent method on
  `f64` (or anything else) to interop with — unlike ordinary
  [method calls](docs/LANGUAGE.md#method-calls), which render verbatim and
  trust Rust's own type checker, these three synthesize a real `format!`
  call (or, for `.grouped()`, a small self-contained block expression)
  instead. `.fixed`/`.padded` lower to `format!`'s `{:.*}`/`{:0>1$}`
  dynamic-precision/dynamic-width specifiers, which is also why their
  argument can be any expression (a signal read), not just a literal — a
  plain `{:.2}`-style spec only accepts a compile-time constant.
  `.grouped()` (thousands separators) has no `format!` macro-syntax
  equivalent at all, so it's the one case that lowers to a block
  expression: split the `Display` text on an optional leading `-` and an
  optional `.`, group the integer part into comma-separated 3-digit
  chunks from the right, leave the sign/decimal part untouched. Same
  reserved-identifier trade-off `stash` already accepted (see the
  2026-07-22 entry below): a real method genuinely named `fixed`/`padded`/
  `grouped` on some other receiver type would collide with this.
  Deliberately scoped narrow rather than attempting the whole
  "string/date/number formatting" gap in one pass: date/time formatting
  needs a `Date`/`Time` type Kittine doesn't have at all yet (a real
  design decision — its own literal syntax, probably its own type-tag —
  logged honestly as still open rather than half-built), and string
  formatting had no real gap left to close (case conversion/`trim`/etc.
  already reach through plain `MethodCall` passthrough). See
  [LANGUAGE.md § Number formatting](docs/LANGUAGE.md#number-formatting).
  Verified with 4 new compiler tests (167 total, up from 163) and a real
  `cargo check --target wasm32-unknown-unknown` against Leptos 0.7 —
  `example-app`'s `Home.kitty` gained a `revenue` signal formatted three
  ways (`revenue.fixed(2)`, `revenue.grouped()`, and `count.padded(4)`
  reusing the existing click-counter signal), logged via `craft<...>` and
  rendered live in the view.

## [Unreleased] - 2026-07-23

### Added (array-of-`litter`/`breed` types, closures, `pounce>` as an expression)

- **A `litter`/`breed` name (optionally `[]`-suffixed) is now a real
  `func`/`purr` param/return type**, not just a [litter field's
  type](docs/LANGUAGE.md#litters) — closing the "no array-of-`litter`/
  `breed` types" gap logged 2026-07-22. `DocEntry[] entries` mirrors
  `#n[]`/`#w[]`/`#f[]`'s own array convention (`Vec<DocEntry>` once
  generated); `DocEntry entry` is the scalar form. A param position tells
  a custom type apart from an untyped param name by lookahead (a
  capitalized identifier immediately followed by another identifier names
  a type). See [LANGUAGE.md § A litter/breed name as a prop or purr
  param/return type](docs/LANGUAGE.md#a-litterbreed-name-as-a-prop-or-purr-paramreturn-type).
- **`|param, ..| expr` closure literals**, lowering verbatim to a real
  Rust closure — the missing filter/map-predicate mechanism a real
  iterator method (`.filter()`, `.map()`) needs, closing that half of the
  same gap. The zero-param form (`|| expr`) shares the lexer's `||` token
  with logical-or, unambiguous since it only reaches a primary (prefix)
  position. A closure param reads bare inside its own body (never
  `.clone()`d), a new `Scope::closure_params` tracked separately from
  `hold_items`/`spin_items` specifically so it doesn't pick up their
  clone-on-read treatment. See [LANGUAGE.md §
  Closures](docs/LANGUAGE.md#closures).
- **`pounce>` now also works as an expression**, not just a statement —
  `pounce> subject Variant(binding)? >> expr .. else> expr`, reachable
  anywhere an expression is expected (a `purr`'s `return (...)`, a `hold`
  binding's value, a call argument), closing the long-tracked "can't
  unwrap a `Result` and return the value in one expression" gap in
  Kittine's error-handling story (see [Production
  readiness](docs/ROADMAP.md#production-readiness)). Same column-indented
  arm grammar as the statement form. The formatter's self-verifying
  round-trip check needed a real fix: the printer can't know what column
  its caller will place a `pounce>` at, solved by always printing it
  starting on a fresh line at column 1, which the column-sensitive
  grammar tolerates regardless of nesting. See [LANGUAGE.md § `pounce>` as
  an expression](docs/LANGUAGE.md#pounce-as-an-expression).

Two real bugs found and fixed while wiring both features into
`example-app`'s real `Shapes.kitty`, not assumed: a `spin`'s default
reactive-list key (`format!("{item}")`) required `Display`, which a
generated `litter`/`breed` never derives (only `Debug` — `#[derive(..)]`
can't produce `Display` for an arbitrary struct at all) — switched the
default to `format!("{item:?}")` universally (harmless: a `<For>` key only
needs to be unique and stable, never user-visible); and a `pounce>`
expression mixing a computed arm with a bare string-literal arm in a
`Word`-returning `purr` produced a real `match`-arm type mismatch
(`E0308`) — fixed by extending the existing bare-literal-owning coercion
to look inside a `pounce>` return value's arms. `kittine-compiler fmt`/
`lint` and the `vscode-kittine` TextMate grammar were updated alongside
the parser/codegen changes (missing `litter`/`breed`/`claw`/`bare`/`hold`/
`pounce>` keyword highlighting was also added to the grammar while there
— a pre-existing gap from Phase 1, not new).

Verified with 12 new compiler tests (162 total) and a real `cargo check`
+ `npm run build` against Leptos 0.7. `example-app`'s `Shapes.kitty`
gained a `shapeLabel` `purr` (pounce-as-expression) and a `farPoints`
`purr` (array-of-litter param/return plus a closure `.filter()`).

## [Unreleased] - 2026-07-22

### Added (Phase 2 begins: `stash` maps, JSON via `serde`, a reference operator, log levels)

- **`stash{ key: expr, .. }`** — a real `String`-keyed map, Kittine's
  "collections beyond arrays." Lowers to `std::collections::HashMap::
  from([..])`, reusing the exact same `Name { field: expr, .. }` grammar
  a `litter` construction already has (parsing/`fmt`/lint all come free).
  Typed with `#n{}`/`#w{}`/`#f{}` on a prop/param/return, mirroring
  arrays' own `#n[]`/`#w[]`/`#f[]` — scalar values only for now, same
  scope limit arrays already have. A real parser ambiguity was found and
  fixed along the way: a `purr`'s return-type `#n{}` map suffix and its
  own following body `{ .. }` are both bare brace pairs back to back,
  which `Parser::parse_signature_type` now disambiguates with a 3-token
  lookahead (return-type position only requires a genuine 2nd brace pair
  to follow before treating `{}` as the map suffix). See [LANGUAGE.md §
  Stashes](docs/LANGUAGE.md#stashes).
- **Every `litter`/`breed` now derives `serde::Serialize`/
  `serde::Deserialize`** unconditionally, alongside the existing `Clone,
  Debug` — a value round-trips through `serde_json` (or any other
  serde-backed format) via a plain path-qualified call, no dedicated
  syntax needed. A project with even one `litter`/`breed` now needs
  `serde` (`features = ["derive"]`) as a real Cargo dependency. See
  [LANGUAGE.md § Litters](docs/LANGUAGE.md#litters).
- **A new `&expr` reference operator** — Kittine had no way to spell a
  Rust reference at all before this, silently blocking any real Rust/
  crate function that takes one (starting with `serde_json::to_string(
  &value)` above, but general — most of the wider Rust ecosystem takes
  references somewhere). Binds like unary `-`; a non-`Copy` scope-tracked
  reference target still gets the usual pre-clone treatment first. See
  [LANGUAGE.md § Reference operator](docs/LANGUAGE.md#reference-operator).
- **`warn<expr>`/`error<expr>`** join `craft<expr>` as two more levels of
  the same statement (`leptos::logging::warn!`/`error!` instead of
  `log!`) — real console-severity levels `craft<...>` alone couldn't
  produce. See [LANGUAGE.md § Printing](docs/LANGUAGE.md#printing-craft--warn--error).

Verified with 10 new compiler tests (150 total) and a real `cargo check`
against Leptos 0.7 for every piece — `example-app`'s `Home.kitty` gained
a `stash`-typed `prices` signal plus real `warn</error<` calls, and
`Shapes.kitty` gained a `serde_json::to_string(&origin)` call serializing
its existing `Point` litter to JSON. See [ROADMAP.md § Status](docs/ROADMAP.md#status-what-works-today)
for what's left open in Phase 2 (an HTTP client needs `async`/`await` and
lambda syntax, neither built yet; file I/O/env access are interop-
reachable today but undemonstrated; validation and deeper string/date/
number formatting are still open).

### Added (reading an `on<Event>` handler's event value)

- **The reserved `event` identifier**, usable anywhere inside an
  `on<Event>` handler's own expression (`onInput`, `onChange`, ...), now
  reads the fired event's string value — `onInput={<{query}> >> event}`
  lowers to `on:input=move |ev| set_query.update(|n| *n =
  event_target_value(&ev))`. Only handlers that actually reference `event`
  get the `move |ev| ...` closure; every other handler keeps the cheaper
  `move |_| ...` it always had. Scoped narrowly via a new
  `Scope::event_bound` flag threaded only through an `on<Event>`
  attribute's own render call — not a blanket reserved word, so a real
  signal/param/`hold` binding named `event` elsewhere in a program is
  unaffected. Closes the one open item from the previous release's `Next
  up` section. See [LANGUAGE.md § The view
  syntax](docs/LANGUAGE.md#the-view-syntax). Verified with 2 new compiler
  tests (140 total) and a real `cargo check` against Leptos 0.7 —
  `example-app`'s `Home.kitty` gained a real text `<input>` that types
  directly into the `username` signal.

### Also discovered (logged, not yet fixed)

- **No array-of-`litter`/`breed` types** — array-typed props/returns stay
  scalar-only (`#n[]`/`#w[]`/`#f[]`). Found while checking whether the new
  `event` support could turn `kittine-website`'s `/docs/language`
  topic-filter pills into a real free-text search over the doc cards; it
  can't yet, since a filterable card list needs to be real struct data in
  an array, which Kittine's type system doesn't represent. See
  [ROADMAP.md § Next up](docs/ROADMAP.md#next-up).

## [Unreleased] - 2026-07-21

### Added (a real package manager + a WASM playground export)

- **`kittine.toml` / `kittine.lock`** — a package manifest (`[package]`
  name/version, `[dependencies]`) and a checksummed lockfile.
  `kittine-compiler add <name>`/`install`/`publish` round out the CLI.
  `install` resolves against a real hosted registry —
  [`buildsbybuchanan/kittine-registry`](https://github.com/buildsbybuchanan/kittine-registry),
  no server of its own, just an `index/<name>.json` per package fetched
  over plain HTTPS with tarballs as GitHub Release assets — and
  sha256-verifies every download into `kitten_modules/<name>/`. A
  bare-name import (`import { X } from 'some-package'`) resolves there
  via the same upward-search `node_modules` resolution uses. `publish`
  is maintainer-only, via the `gh` CLI. Verified for real: a package
  (`kittine-strings`) published to the live registry, then
  added/installed fresh elsewhere and `cargo check`'d against actual
  Leptos 0.7. See [CLI.md](docs/CLI.md), [LANGUAGE.md § Package
  imports](docs/LANGUAGE.md#package-imports).
- **`kittine-compiler` now builds to WebAssembly** (`crate-type =
  ["cdylib", "rlib"]`, a `wasm-bindgen`-exported
  `compile_kitty_single_file`): the single-file lex/parse/codegen path
  has no filesystem dependency, so it runs in a browser with zero server
  involvement. Verified against real generated JS under Node (success
  and parse-error paths both). Powers `kittine-website`'s new
  `/playground` page.

### Added (Phase 1 completion: traits, bounded generics)

- **`claw Name { method(params) type, .. }`** — a trait (Kittine's term
  for a named capability contract; a "claw" is something a cat *has*).
  Every method is a bare signature — name, params, return type, all
  mandatory (no body to infer from). Compiles to `pub trait Name { fn
  method(&self, params) -> Type; }`.
- **`bare Claw for Target { purr method(..) .. }`** — implements a `claw`
  for a `litter`/`breed` (Rust's `impl Claw for Target`; a cat "bares its
  claws" to show a capability). Each method reuses the ordinary `purr`
  grammar/codegen verbatim, plus an implicit `self` available in the body
  with no declaration needed — the same treatment `children` already
  gets in a component. Calling the method on a value needs no new call
  syntax: `value.method(arg)` is just an ordinary method call, already
  supported.
- **Bounded generics**: a `litter`/`breed`'s type parameter can now name
  a `claw` it must implement — `litter NamedHolder<#t: Named> { value #t
  }` — compiling to a real Rust trait bound (`struct NamedHolder<T:
  Named> { .. }`), checked by Rust's own compiler at every construction
  site. This closes the "no bounds" gap from the previous round's minimal
  generics groundwork.
- **Fixed a real, pre-existing bug**: a `Word`-returning `purr` (or now,
  `claw` method) whose `return (expr)` was a *bare* string literal (no
  `+` concatenation to trigger the existing string-format special case)
  rendered as an uncoerced `&'static str`, which doesn't type-check
  against the `String` the signature promises (`E0308`, confirmed
  against real rustc). `purr constant() #w { return ('hi') }` now
  correctly renders `"hi".to_string()`. Found while building the `claw`
  demo below — a method that just returns a literal hit it immediately —
  but it affected any plain `purr` with this exact shape, not just
  methods.
- This closes Phase 1 — [ROADMAP.md § Full
  vision](docs/ROADMAP.md#full-vision-phased-honest) — in full: module
  visibility was judged already-closed (uniform `private` across every
  declaration kind, and Kittine's flat single-module compilation model
  has no real crate/package boundary for `pub(crate)`-style granularity
  to mean anything), and a full trait system (this entry) closes the
  remaining "real type system" gap alongside the structs/enums/pattern-
  matching/minimal-generics work from the previous round. The one
  documented exception is `pounce>` staying statement-only — see
  [docs/LANGUAGE.md § Known limitations](docs/LANGUAGE.md#known-limitations).
- Verified with 6 new compiler tests (97 total, 0 regressions) and real
  `cargo check`/`npm run build` (full Vite → `cargo` → `wasm-bindgen`
  pipeline) against Leptos 0.7 — `example-app`'s `Shapes.kitty` gained a
  `Named` claw implemented for `Point`, called via `origin.describe()`,
  and a bounded `NamedHolder<#t: Named>` instantiated with a real `Point`
  value, compiling clean under `wasm32-unknown-unknown` to a working wasm
  binary.

### Added (Phase 1: structs, enums, pattern matching, minimal generics)

- **`litter Name { field type, .. }`** — a plain data record (Kittine's
  term for a Rust struct; a "litter" of related fields, matching the
  cat-themed naming of every other keyword). A struct literal (`Point {
  x: 1, y: 2 }`) constructs one; `.field` reads a field with no
  method-call parens, unlike `.method()`. Compiles to `#[derive(Clone,
  Debug)] pub struct Name { pub field: Type, .. }`. A field's `Word`
  literal value gets the same `.to_string()` coercion a `purr` argument
  does, via the same whole-`import`-graph `Signatures` collection
  `purr`/`breed` already used (extended, not duplicated).
- **`breed Name { Variant(type)?, .. }`** — a closed set of named
  variants (a Rust enum; a "breed" is one of several kinds of cat, a
  variant one of several kinds of value). A variant carries at most one
  payload value. `Circle(5)` constructs a payload-carrying variant
  (`Shape::Circle(5f64)`); a bare `Idle` references a unit one
  (`Shape::Idle`) — both told apart from an ordinary `purr` call/variable
  read by the compiler already knowing every reachable `breed`'s
  variants, not by any new syntax. Compiles to `#[derive(Clone, Debug)]
  pub enum Name { Variant(Type), .. }`.
- **`pounce> subject` `Variant(binding)? >> stmt` `else> stmt`** — pattern
  matches a `breed` value, lowering to a real Rust `match` with each
  pattern fully qualified (`Shape::Circle(r) => { .. }`) and `else>`
  becoming the wildcard `_ =>`. Arms are indented **one level under**
  `pounce>` (its own children, not siblings the way `orif>`/`else>` sit
  beside `if>`), and each arm is exactly one statement. **Statement-only
  for this round** — there's no way yet to use a `pounce>`'s result as a
  value inside a `return ( ... )`; see [docs/LANGUAGE.md § Known
  limitations](docs/LANGUAGE.md#known-limitations). This is the concrete
  piece of Kittine's "no error-handling story" gap that's now closed: a
  `breed Result { Ok(#t), Err(#w) }`-shaped type plus branching on it
  both work for real today.
- **Minimal generics groundwork**: a `litter`/`breed` may declare *at
  most one* type parameter — `litter Holder<#t> { value #t }` — with no
  bounds, no multiple parameters, and no generic `purr`/`func`. No
  explicit instantiation syntax at the construction site either
  (`Holder<#n> { .. }`) — Rust infers the concrete type from the field
  value itself, the same way it infers any other generic constructor's
  type parameter, which is both simpler to implement and shorter to
  write. Compiles to a single Rust generic parameter, `T`.
- New `#t` type-tag sigil (`TokenKind::TypeGeneric`) for the above, and a
  new `:` token for struct-literal `field: value` syntax.
- `kittine-compiler`'s signature-collection map (previously
  `known_functions: HashMap<String, Vec<String>>`, `purr`-only) is now a
  `Signatures` struct covering `purr` params/return, `litter` fields, and
  `breed` variants together, threaded through `codegen::Scope` the same
  way as before — this is what lets a `litter` field's or `breed`
  variant's `Word`-typed value get the same cross-file string-literal
  coercion a `purr` argument already had.
- New [docs/GRAMMAR.md](docs/GRAMMAR.md): the complete, formal EBNF
  grammar for the whole language (lexical tokens through every syntactic
  production), derived directly from the lexer/parser source rather than
  written aspirationally — the Phase 1 "formal grammar spec document"
  item. [docs/LANGUAGE.md § Full grammar
  summary](docs/LANGUAGE.md#full-grammar-summary) stays as a shorter
  quick-reference version of the same grammar.
- Verified with 6 new compiler tests (91 total, 0 regressions) and real
  `cargo check`/`cargo build`/`npm run build` (full Vite → `cargo` →
  `wasm-bindgen` pipeline) against actual Leptos 0.7 — `example-app`
  gained a real `Shapes.kitty` component (a `Point` struct, a generic
  `Holder<#t>` instantiated with both a `Num` and a `Word` value, a
  `Shape` enum pattern-matched with `pounce>`) composed into `Home.kitty`
  and compiled clean under `wasm32-unknown-unknown`, producing a working
  wasm binary — not just asserted against generated-string snapshots.

### Added (syntax branch)

- **Scalar type tags on a prop/`purr` param or return type are now
  optional.** `purr greet(name) { return ('Hello, ' + name) }` needs no
  `#w` anywhere — `kittine-compiler` infers `Word` from the string
  concatenation in the body, the same way a human reading the code would.
  A new `infer` pass runs once, right after parsing, and fills in every
  omitted `ty`/`return_type` before codegen (or `known_functions`
  signature collection) ever sees the `Program` — so call-site coercions
  (e.g. the `Word`-parameter string-literal `.to_string()` treatment) work
  identically whether the type came from a hand-written tag or an
  inferred one. Rules: arithmetic/ordering comparison → `Num`; `+`/`==`/
  `!=` against a string literal → `Word`; `==`/`!=` against `yes>`/`no>`,
  or either side of `&&`/`\|\|` → `Flag`; no clue at all → `Word`
  (default). Explicit tags always win outright. Two intentional limits:
  inference is local to one function/component (doesn't propagate through
  a call to another `purr`), and array tags (`#n[]`/`#w[]`/`#f[]`) are
  still mandatory. See
  [docs/LANGUAGE.md § Type inference](docs/LANGUAGE.md#type-inference).
  Verified with 7 new compiler tests (85 total) and a real `cargo check`
  against Leptos 0.7 — `example-app`'s `Greetings.kitty`/`Home.kitty` had
  their tags dropped as a real demonstration, and regenerating them
  produced **byte-identical** `.rs` output to the explicitly-tagged
  version, confirming inference is purely front-end sugar.

### Added

- **Kebab-case JSX attribute names** (`data-*`, `aria-*`, and any other
  hyphenated attribute) are now parsed correctly — a `-` immediately
  after an attribute name is read as a continuation of that name, never
  as subtraction, since attribute-name position is a dedicated parsing
  context rather than a general expression. Found while building real
  `data-kittine-component`-style inspect-mode fingerprinting. Verified
  with two new compiler tests and a real `cargo check` build.
- **`leptos_meta` is now in scope in every generated file**, the same
  unconditional way `leptos_router` already is — `<Title>`, `<Meta>`,
  `<Link>`, `<Stylesheet>`, etc. are plain Leptos components, composable
  with zero new Kittine syntax. `example-app/Cargo.toml` gained the
  `leptos_meta` dependency it was missing (only `example-ssr` had it).
  Preparatory only: no `.kitty` file in this repo uses a `leptos_meta`
  component yet, since doing so also needs Leptos's own
  `provide_meta_context()` called in the app root — not yet a documented
  Kittine pattern.

### Changed

- **Breaking: type tags are now two-character sigils — `#n` (Num), `#w`
  (Word), `#f` (Flag), with `#n[]`/`#w[]`/`#f[]` for an array of one.**
  Retires the bracket-wrapped `<<Num>>`/`<<Word>>`/`<<Flag>>` /
  `<<Num[]>>`/`<<Word[]>>`/`<<Flag[]>>` form entirely — existing `.kitty`
  source using the old form must be updated (mechanical: `<<Num>>` → `#n`,
  `<<Word[]>>` → `#w[]`, etc., no semantic change). Motivated by the
  [Brevity by design](docs/LANGUAGE.md#brevity-by-design) rule added
  earlier today, which the old form was actually violating for every
  scalar type — `<<Word>>` (8 characters) was *longer* than the `String`
  (6 characters) it lowers to, and `<<Flag>>`/`<<Num>>` were longer than
  `bool`/`f64` too. The new sigil is shorter than every Rust type it can
  stand for (`#n` vs `f64`, `#w` vs `String`, `#f` vs `bool`, and the
  array forms similarly against `Vec<..>`) and needs no closing
  delimiter, unlike the old form's `>>`.
  - Lexed as a single fused token straight off the `#` + type-initial
    character (`lexer::TokenKind::TypeNum`/`TypeWord`/`TypeFlag`), the
    same way `if>`/`craft<` already fuse a keyword with its trailing
    punctuation — not two separate `<` tokens needing a matching `>>`
    like the retired form did. An unrecognized sigil letter (anything
    other than `n`/`w`/`f`) is now a lex error, not a parse error, since
    `#` has no other meaning in Kittine.
  - Purely a front-end syntax change: type tags still erase to nothing in
    codegen, so every `.kitty` file across `example-app` and `example-ssr`
    was updated to the new sigils and re-run through `kittine-compiler
    build` — the regenerated `.rs` output is **byte-for-byte identical**
    to before, confirming this is sugar, not a behavior change. All 76
    existing compiler tests updated to the new syntax and passing; the VS
    Code extension's TextMate grammar and every doc mentioning type tags
    (`LANGUAGE.md`, `README.md`, `ROADMAP.md`, `VSCODE_EXTENSION.md`,
    `vscode-kittine/README.md`) updated to match.

### Documentation

- **"Brevity by design"**: a new section in
  [docs/LANGUAGE.md](docs/LANGUAGE.md#brevity-by-design), and a matching
  table in [README.md](README.md#shorter-than-the-rust-it-generates),
  stating the design constraint explicitly and showing it with real
  Kittine-vs-generated-Rust side-by-sides pulled from
  `kittine-compiler/src/tests.rs` (not invented for effect) — signals,
  booleans, `craft<...>`, `spin` loops, and the `greet('World')` →
  `greet("World".to_string())` `Word`-parameter coercion. Codifies two
  mechanical rules (one token doing the work of several Rust ones; types
  and ownership inferred rather than spelled out) as a standing constraint
  for any future language addition, not just a one-time pass over the
  existing syntax. No compiler or grammar changes — existing `.kitty`
  syntax and generated output are unaffected.

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
  **Resolved below** — see "Server-side rendering" under Added.

### Changed

- **Breaking:** removed the `¨...¨` diaeresis-quoted string form. Strings are
  now written with `'...'` or `"..."` — fully interchangeable, pick whichever
  avoids escaping. Existing `.kitty` source using `¨...¨` must be updated.

### Added

- **Server-side rendering (SSR), via a new `example-ssr/` project and
  [docs/SSR.md](docs/SSR.md).** `kittine-compiler` needed **zero
  changes** — the same `.kitty` → `.rs` compiler that powers
  `example-app`'s CSR build powers `example-ssr` too; the only
  difference is which Cargo features the *downstream* crate enables on
  `leptos`, which Kittine's codegen has no concept of either way. Uses
  `cargo-leptos` + Axum (`leptos_axum`) — a second toolchain, run
  *alongside* `vite-plugin-kittine`/Vite, not a replacement for it;
  `example-app`'s CSR path is completely unaffected. Verified for real,
  not just compiled: `curl`'d the raw HTTP response and confirmed
  genuine pre-rendered HTML content (no JavaScript involved), then used
  Playwright against the running server to confirm hydration actually
  wires up interactivity and that client-side `<A>` routing between
  pages works after that.
  - **Two real runtime gotchas found by actually running this, not
    assumed:** `<HydrationScripts>` comes from `leptos`'s own prelude
    (not `leptos_meta`, where `MetaTags` lives) — omit it and the page
    renders correctly server-side but silently never becomes
    interactive, since nothing ever loads the client bundle. Separately,
    `.leptos_routes(..)` alone doesn't serve the wasm/JS bundle itself —
    `.fallback(leptos_axum::file_and_error_handler(shell))` is needed too,
    or the hydration script's own asset request 404s.
  - **Investigated and ruled out a smaller "SSG-only" scope** before
    committing to this: Leptos 0.7 has no built-in "prerender once at
    build time, ship static files" mode distinct from request-time SSR —
    generating static HTML at all still needs the same native `ssr`-feature
    build and the same `leptos_axum`-style rendering machinery, just run
    once per route instead of listening on a socket. No smaller path
    existed to find.
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
