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
  arrays (including array-typed props/returns — `#n[]`/`#w[]`/`#f[]`),
  booleans, type tags (`#n`/`#w`/`#f`, a two-character sigil form —
  see [LANGUAGE.md § Type tags](LANGUAGE.md#type-tags)) that are now
  **optional** on a prop or `purr` param/return type, inferred from body
  usage when omitted (see [LANGUAGE.md § Type
  inference](LANGUAGE.md#type-inference)),
  logical `&&`/`||` combining comparisons into one condition, method
  calls (`receiver.method(arg, ..)`, chains work), calling the result of
  an expression (`callee(arg, ..)` where `callee` isn't a bare name),
  tuple literals (`(expr, expr, ..)`), path-qualified expressions
  (`Type::method()`, `Type::CONST`, multi-segment paths).
- **User-declared data types**: `litter Name { field type, .. }` (a
  struct — `Name { field: expr, .. }` constructs it, `.field` reads a
  field), `breed Name { Variant(type)?, .. }` (a closed set of variants —
  an enum — `Circle(5)`/bare `Idle` construct one), `pounce> subject`
  pattern-matching a `breed` value (statement-only for now), and minimal
  generics groundwork (at most one type parameter per `litter`/`breed`,
  e.g. `litter Holder<#t> { value #t }`, inferred at each construction
  site with no explicit instantiation) — and now a real **trait system**:
  `claw Name { method(params) type, .. }` declares one, `bare Claw for
  Target { purr method(..) .. }` implements it (Rust's `impl Claw for
  Target`), and the generic type parameter above can be bounded by one
  (`litter NamedHolder<#t: Named> { value #t }`, a real Rust trait bound
  Rust's own compiler enforces). See [LANGUAGE.md §
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
- **Tooling**: `kittine-compiler` CLI (build), a Vite plugin driving the
  full compiler → cargo → wasm-bindgen pipeline, a VS Code extension
  (TextMate grammar only — no language server), Vercel deployment config.
  `kittine-compiler build` only rewrites a `.rs` file when its generated
  content actually changed, so unrelated dependencies keep their mtime
  and downstream `cargo`/`wasm-bindgen`/Vite freshness checks correctly
  skip redoing work — verified with a real `npm run build` (~31s → ~5s
  for a no-op rebuild of `example-app`).

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
tested compiler (97+ tests, every feature round-tripped against actual
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
   model and react to failure now. What's still missing: `pounce>` is
   statement-only, so a function can't unwrap a `Result` and `return` the
   unwrapped value in one expression the way Rust's own `match`/`?` can;
   and an *un*-modeled runtime panic (an actual Rust `panic!`, not a
   `breed`-modeled failure) is still a hard crash reported only in the
   browser devtools console.
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

- **`kittine-compiler` provisioning for Vercel builds (kittine-ide).**
  `vite-plugin-kittine` shells out to `kittine-compiler` on every `.kitty`
  transform, so it must resolve on `PATH` before Vite starts. Locally this
  is solved with `cargo install --path vendor/kittine/kittine-compiler`
  (installs to `~/.cargo/bin`, already on `PATH`) — see the
  `kittine-ide` README for the Windows/git-bash PATH-mangling gotcha this
  surfaced (`spawnSync kittine-compiler ENOENT`). That fix is
  machine-local only; `kittine-ide`'s `scripts/vercel-build.sh` (per its
  `README.md`) still needs to install/build `kittine-compiler` and put it
  on `PATH` as part of the Vercel build image before this repo's builds
  will actually succeed there. Not yet done as of 2026-07-19.

Everything else that was previously listed here has landed (see Done
below). SSR/SSG (previously the one open item, flagged as needing a real
architecture decision rather than a quick increment) is done too — see
[SSR.md](SSR.md) and the Done entry below. Add here the moment a real gap
turns up (per the standing rule at the top of this file).

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

### Phase 2 — Standard library

File I/O, JSON/XML/YAML/CSV (de)serialization, HTTP client, string/date/
number formatting utilities, collections beyond arrays (maps), validation,
logging, environment/config access, encryption/hashing primitives.

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

A real package registry, publishing, dependency resolution and lock files,
workspaces; a production Tree-sitter grammar; a minimal-but-real Language
Server Protocol implementation (diagnostics, hover, go-to-definition,
rename); VS Code Marketplace publication; a formatter and linter; a REPL
and debugger.

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

**Do not start building the site itself until told to** — this roadmap
is preparation, not a green light. The moment that instruction comes, the
right starting decision is CSR (`example-app`-style, via Vite) vs. SSR
(`example-ssr`-style, via `cargo-leptos`) for kittine.dev specifically —
see the [comparison table in SSR.md](SSR.md#why-this-needs-a-different-toolchain)
for the trade-off (SSR gets real SEO/first-paint; CSR keeps the simpler
"just static files" deployment story this repo's `vercel.json` already
relies on for `example-app`).
