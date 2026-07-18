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
  function calls, arithmetic/string-concat expressions, arrays, booleans,
  type tags (`<<Num>>`/`<<Word>>`/`<<Flag>>`).
- **Modules**: `import { A, B } from './file.kitty'`, resolved and compiled
  recursively by `kittine-compiler build` (cycle detection included).
- **Component composition**: `<Name prop='x' />` for a PascalCase tag,
  passing typed props as plain values (not reactive DOM-attribute
  closures).
- **List rendering in views**: `spin<{item}> in list }{ .. }{` inside
  `return ( ... )` lowers to a reactive Leptos `<For>`, keyed by
  `format!("{item}")`.
- **Component children**: an untyped `children` param (`func Card(children)
  { .. { children() } .. }`) renders whatever JSX a caller nests inside
  `<Card>...</Card>` — no `children=` attribute needed, Leptos's `view!`
  macro wires it through automatically.
- **Codegen targets real Leptos 0.7 CSR** — every language feature above
  has been round-tripped through `cargo check`/`cargo build` against the
  actual `leptos` crate, not just asserted against generated-string
  snapshots.
- **Tooling**: `kittine-compiler` CLI (build), a Vite plugin driving the
  full compiler → cargo → wasm-bindgen pipeline, a VS Code extension
  (TextMate grammar only — no language server), Vercel deployment config.

**Can it build a full website yet? No.** Everything above adds up to a
solid *single-page app* — one component mounted to one root, with state,
composition, lists, and shared logic. A website needs multiple navigable
pages and (usually) fast/SEO-able first paint, and neither exists yet:
there is no routing at all, and rendering is CSR-only (blank page until
the WASM bundle loads and mounts — nothing for a crawler to see without
executing JS). See the top of [Next up](#next-up).

See [LANGUAGE.md § Known limitations](LANGUAGE.md#known-limitations) for
the precise, current boundary of what's supported — that section is the
authoritative day-to-day list; this file is about direction, not spec.

## Next up

Roughly in priority order, driven by "what does writing the actual Kittine
website (in Kittine) need next":

1. **Routing / multi-page navigation.** Not started at all — there is no
   way to define more than one page or navigate between URLs. This is the
   single biggest blocker to calling anything built in Kittine "a website"
   rather than "a single-page app." Leptos has a router
   (`leptos_router`); Kittine needs syntax + codegen to expose it (route
   definitions, links, at minimum).
2. **SSR/SSG.** Currently CSR-only. A marketing/business site cares about
   first paint and SEO; Leptos already supports both server rendering and
   static generation, Kittine just doesn't expose either yet.
3. **Array-typed props/returns.** `<<Num>>`/`<<Word>>`/`<<Flag>>` cover
   scalars; there's no tag for "a list of `Num`" yet, so a `purr` can't
   return one and a component can't take one as a prop — this blocks
   anything data-driven (nav menus, card grids) that isn't a hardcoded
   literal.
4. **`export` / visibility control.** Every top-level `func`/`purr` is
   implicitly importable by any file today; there's no way to keep
   something file-private.
5. **Basic comparison operators.** `>>` is equality-only; no `<`, `>`,
   `<=`, `>=`, `!=`. Needed for anything beyond exact-match conditionals
   (pagination, validation ranges, sort order).
6. **Incremental/cached builds for the import graph.** `kittine-compiler
   build` currently recompiles every reachable `.kitty` file on every
   invocation — correct, but wasteful once a real site has many files.
7. **`key` control for view-position `spin`.** Always keys by
   `format!("{item}")` today; no way to key by something else (an id
   field, an index) once array elements stop being bare scalars.

Done: ~~List rendering in views~~ (`spin` in `return ( ... )` → Leptos
`<For>`) — landed 2026-07-18. ~~Component children~~ (untyped `children`
param + `children()`) — landed 2026-07-18.

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

An HTTP server + routing + middleware story; database connectivity (SQLite
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
rename); GitHub Linguist submission so `.kitty` shows as **Kittine** in
repo language stats; VS Code Marketplace publication; a formatter and
linter; a REPL and debugger.

### Phase 6 — Production hardening

Benchmarking, profiling, monitoring/metrics hooks, security-review pass
across the whole toolchain, semantic versioning + release process, a
stable v1.0 grammar freeze.

## The Kittine website

The plan (per explicit instruction) is to build **buildsbybuchanan-style
kittine.dev in Kittine itself**, once — and only once — enough of [Next
up](#next-up) lands to make that practical. Composing a *single* page
is ready (imports ✅, props ✅, list rendering ✅, component children ✅).
A real multi-page site additionally needs routing ❌ (not started — see
[Next up](#next-up) #1) and ideally SSR/SSG ❌ for first paint and SEO, plus
array-typed props for structured nav/card data. Do not start building the
site itself until told to — this roadmap is preparation, not a green light.
