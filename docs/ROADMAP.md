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
  reactive state (`<{name}> >> value`), `if>`/`orif>`/`else>` control flow,
  `spin` loops (both imperative-statement and reactive-list-in-view forms),
  function calls, arithmetic/string-concat expressions, comparisons
  (`>>`/`<`/`<=`/`>`/`>=`/`!=`, usable generally, not just in conditions),
  arrays (including array-typed props/returns — `<<Num[]>>`/`<<Word[]>>`/
  `<<Flag[]>>`), booleans, type tags (`<<Num>>`/`<<Word>>`/`<<Flag>>`),
  logical `&&`/`||` combining comparisons into one condition, method
  calls (`receiver.method(arg, ..)`, chains work), calling the result of
  an expression (`callee(arg, ..)` where `callee` isn't a bare name),
  tuple literals (`(expr, expr, ..)`).
- **Modules**: `import { A, B } from './file.kitty'`, resolved and compiled
  recursively by `kittine-compiler build` (cycle detection included).
  `private func`/`purr` opts an item out of being importable, enforced by
  Rust's own privacy rules.
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
  running dev server. Programmatic navigation (`use_navigate()`) is a
  known, documented gap (needs a path-qualified expression Kittine
  doesn't have yet). CSR-only — no SSR/SSG integration yet.
- **Codegen targets real Leptos 0.7 CSR** — every language feature above
  has been round-tripped through `cargo check`/`cargo build` against the
  actual `leptos` crate, not just asserted against generated-string
  snapshots. Routing was additionally driven end-to-end with Playwright
  against a real running dev server (navigation, the 404 fallback, and
  continued reactivity all actually observed, not just compiled).
- **Tooling**: `kittine-compiler` CLI (build), a Vite plugin driving the
  full compiler → cargo → wasm-bindgen pipeline, a VS Code extension
  (TextMate grammar only — no language server), Vercel deployment config.
  `kittine-compiler build` only rewrites a `.rs` file when its generated
  content actually changed, so unrelated dependencies keep their mtime
  and downstream `cargo`/`wasm-bindgen`/Vite freshness checks correctly
  skip redoing work — verified with a real `npm run build` (~31s → ~5s
  for a no-op rebuild of `example-app`).

**Can it build a full website yet? Getting close, for the right kind of
site.** A real multi-page app — composed components, typed props, list
rendering, children, shared logic, and now genuine client-side routing
with a 404 fallback — all work and are verified end-to-end. What's still
missing specifically for "website" (as opposed to "app"): SSR/SSG (first
paint is currently blank until WASM loads, and there's nothing for a
crawler without executing JS — fine for an internal tool, not ideal for a
public marketing/business site that cares about SEO). See [Production readiness](#production-readiness)
for the broader "is this ready to build real things on" answer, and the
top of [Next up](#next-up) for what's next specifically toward "website."

See [LANGUAGE.md § Known limitations](LANGUAGE.md#known-limitations) for
the precise, current boundary of what's supported — that section is the
authoritative day-to-day list; this file is about direction, not spec.

## Production readiness

**Not yet.** This is answered honestly here every time something changes
— per standing instruction, not a one-time verdict. Kittine is a real,
tested compiler (65+ tests, every feature round-tripped against actual
Leptos, routing (including dynamic segments) driven end-to-end in a real
browser) with a language core solid enough for a genuine multi-page
client-side app. That's real progress, not the same thing as
production-ready. What's still missing:

1. **SSR/SSG.** CSR-only today — and, per the investigation above, not a
   quick follow-up: it's a real architecture fork (adopt `cargo-leptos`
   and retire Vite for SSR-mode projects, or hand-roll an Axum server +
   dual `ssr`/`hydrate` build alongside Vite), not an increment on the
   current toolchain.
2. **No error-handling story.** Kittine has no `Result`/`Option`-shaped
   construct; a runtime panic in generated Rust is a hard crash reported
   only in the browser devtools console, with nothing a `.kitty` author
   can catch or recover from.
3. **No way for a Kittine *program* to have its own tests.** The compiler
   is well-tested; a person writing `.kitty` files has no test runner
   surfaced to them at all.
4. **No versioned releases.** Kittine itself has no v1.0/semver — several
   breaking syntax changes have already happened in rapid succession
   (removing `¨...¨`, adding four new constructs) with no migration story
   or deprecation window, because there's no released version to be
   compatible *with* yet.
5. **No dedicated security review.** Kittine source becomes Rust source;
   the string-escaping logic (`escape_str` in `codegen.rs`) that stands
   between a `.kitty` string literal and a generated Rust string literal
   hasn't had a focused audit for injection-style edge cases.
6. **Tooling stops at "does it compile."** The VS Code extension is
   TextMate-only — no diagnostics, no autocomplete, no go-to-definition —
   so mistakes surface at `kittine-compiler build` time, not while typing.

None of these are hard blockers to *experimenting* with Kittine or
building the planned example site — they're what stands between "this
works" and "I'd stake a real product on this." Phase 1 (real error
handling, a real type system) and Phase 6 (security review, semver,
grammar freeze) in [Full vision](#full-vision-phased-honest) are where
these get addressed.

## Next up

Roughly in priority order, driven by "what does writing the actual Kittine
website (in Kittine) need next":

1. **SSR/SSG — investigated, not a bounded task like the rest of this
   list.** Currently CSR-only. Checked what SSR actually requires for
   Leptos 0.7 directly (Leptos's own book, `cargo-leptos`'s own docs)
   rather than assuming, and it's a real architecture fork, not an
   increment:
   - Leptos SSR needs a Rust HTTP server (`leptos_axum` or
     `leptos_actix`) plus **two separate builds of the same crate**
     behind Cargo feature flags — `hydrate` (client, `wasm32-unknown-
     unknown`) and `ssr` (server, native) — and a different client entry
     point (`hydrate()` instead of `mount_to_body()`).
   - **`cargo-leptos`** (the standard tool for this) explicitly *replaces*
     Vite-style dev-server/build orchestration — its own docs describe
     it as "not designed for parallel use with Vite or similar tools." It
     runs both builds, wires up hydration, and serves everything itself
     from its own dev server (`127.0.0.1:3000` by default).
   - That means adopting it isn't "add a flag to vite-plugin-kittine" —
     it's retiring Vite as the dev/build tool for any Kittine project
     that wants SSR, in favor of `cargo-leptos`'s own toolchain. The
     alternative (hand-rolling Axum + a dual `ssr`/`hydrate` build
     *alongside* Vite, without `cargo-leptos`) avoids that specific
     conflict but is more custom code to build and maintain, not less
     work overall.
   - **Conclusion:** this is real Phase 4 material — a dedicated
     architecture decision (which of the two paths above, and what that
     means for `vite-plugin-kittine`'s role) — not a same-shape "next up"
     item alongside things like logical operators or dynamic routes.
     Staying CSR-only is the honest current state; revisit this with a
     scoped design pass, not a quick increment, when it's actually time
     to build the public site.
2. **Path-qualified expressions (`Type::method()`, `Type::CONST`).**
   Discovered while wiring up the dynamic-route demo (see Done below):
   Kittine's grammar has no `::`, which blocks constructing
   `NavigateOptions::default()` — the one piece standing between
   `use_navigate()` and a real programmatic-navigation demo. Everything
   else needed to call it (calling the result of an expression,
   `use_navigate()('/home')`) already works.
3. **Re-exports.** `private` controls whether an item is importable at
   all, but there's no way to import something into a file and then
   re-expose it under that file's own name for a third file to import.

Done: ~~List rendering in views~~ (`spin` in `return ( ... )` → Leptos
`<For>`) — landed 2026-07-18. ~~Component children~~ (untyped `children`
param + `children()`) — landed 2026-07-18. ~~Routing~~ (`leptos_router`
composed with zero new syntax) — landed 2026-07-18. ~~Comparison
operators~~ (`<`/`<=`/`>`/`>=`/`!=`, usable generally not just in
conditions) — landed 2026-07-18. ~~Array-typed props/returns~~
(`<<Num[]>>`/`<<Word[]>>`/`<<Flag[]>>`) — landed 2026-07-18. ~~`export`/
visibility control~~ (`private func`/`purr`, enforced by Rust's own
privacy rules) — landed 2026-07-18. ~~Logical `&&`/`||`~~ (combine
comparisons into one condition, usable generally not just in
conditions) — landed 2026-07-18. ~~Incremental/cached builds for the
import graph~~ (`.rs` files are only rewritten when their content
actually changed) — landed 2026-07-18. ~~Same-file string-literal ->
`Word` `purr` parameter~~ (the compiler now knows a same-file callee's
signature) — landed 2026-07-18. ~~Dynamic-segment routes, demonstrated~~
(method calls, a tuple path, and `leptos_router::hooks` in scope,
verified end-to-end with Playwright; programmatic navigation stayed open
as item 2 above, once `NavigateOptions::default()` turned out to need
path-qualified expressions) — landed 2026-07-18. ~~Cross-file
string-literal -> `Word` `purr` parameter~~ (`kittine-compiler build`
now collects every reachable file's `purr` signatures before generating
any single file's code, so an imported callee gets the same coercion a
same-file one already had) — landed 2026-07-18. ~~`key` control for
view-position `spin`~~ (an optional `key(expr)` clause overrides the
default `format!("{item}")` key; verified against real Leptos and wired
into `example-app`'s `NavList.kitty`) — landed 2026-07-18.

## Full vision (phased, honest)

This is the "production-ready ecosystem" scope: everything a developer
would eventually expect to build **business websites, e-commerce, CMS/CRM/
ERP systems, SaaS platforms, REST/GraphQL APIs, desktop apps, CLIs, and
enterprise software** in Kittine alone. None of the phases below are
started unless explicitly listed under [Status](#status-what-works-today)
above. Writing full docs/tutorials for anything in this list before it's
real would mean documenting features that don't exist — so this section
stays a plan, not a spec, until an item graduates into `LANGUAGE.md`.

### Phase 1 — Language completeness (extends [Next up](#next-up))

Structs/records, enums, pattern matching, generics groundwork, a real type
system beyond the current three scalar tags, module visibility, proper
error types/`Result`-style handling in Kittine syntax, a formal grammar
spec document.

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

SSR/SSG (Leptos already supports both — Kittine needs to expose them),
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
kittine.dev in Kittine itself**, once — and only once — enough of [Next
up](#next-up) lands to make that practical. Multi-page composition with
real, structured data is ready: imports ✅, props ✅, list rendering ✅,
component children ✅, routing ✅, array-typed props/returns ✅ (nav/card
data can be real arrays now, not just hardcoded literals). What's left
that would matter for a *real public* site — as opposed to what's
buildable today — is mainly SSR/SSG ❌ (first paint + SEO), still open in
[Next up](#next-up). Do not start building the site itself until told to — this
roadmap is preparation, not a green light.
