# Deploying a Kittine project fast

Two build shapes exist today (see [SSR.md](SSR.md) for the full CSR-vs-SSR
comparison); each has a different story for "one command, fast, works
anywhere" — this doc is that story for both, plus the concrete pattern
that gets a `cargo-leptos` project's Vercel deploys under 5 minutes for
real, not just in theory.

## CSR (Vite) projects: already one command, already fast

`example-app`, `kittine-ide` — anything built with `vite-plugin-kittine`
already has this solved. `vite-plugin-kittine` hooks `.kitty` compilation
into Vite's own transform pipeline, so `npm run dev` / `npm run build` is
already the single command that does everything: compile `.kitty` → `.rs`
→ `cargo build --target wasm32-unknown-unknown` → `wasm-bindgen`. Nothing
else to add here — this is the "as easy as building a React app" bar
already met, because Vite's plugin system gives Kittine a hook cargo-leptos
doesn't have (see below).

The only per-project setup requirement: `kittine-compiler` must resolve on
`PATH` before Vite starts (`vite-plugin-kittine` shells out to it). On
Vercel, that means building it from source (or a vendored copy) as an
early build step — see `kittine-ide/scripts/vercel-build.sh` for the
reference implementation, including the prebuilt-`wasm-bindgen-cli`
install trick that avoids `cargo install`ing it from source (minutes
saved right there).

## SSR (`cargo-leptos`) projects: one command locally, real caching for CI

`example-ssr`, `kittine-website` — `cargo-leptos` has no plugin hook
equivalent to Vite's, so `.kitty` → `.rs` compilation and the actual
`cargo leptos build` are two genuinely separate steps. Two things fix
this, both verified for real in `kittine-website` (see that repo's own
`scripts/` and `.github/workflows/deploy.yml` for the working versions):

### 1. One local command

A thin wrapper script collapses the two steps:

```sh
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

KITTINE_COMPILER="${KITTINE_COMPILER:-../kittine/kittine-compiler/target/release/kittine-compiler}"
if [ ! -x "$KITTINE_COMPILER" ]; then
  cargo build --release --manifest-path ../kittine/kittine-compiler/Cargo.toml
fi

"$KITTINE_COMPILER" build src/App.kitty
cargo leptos "${@:-build}"
```

`./scripts/build.sh`, `./scripts/build.sh watch`, `./scripts/build.sh
serve` — one command each, matching the CSR/Vite ergonomics above. This
doesn't watch `.kitty` files during `watch`/`serve` (`cargo-leptos` has no
hook to trigger on), so re-run the script after editing a `.kitty` file
mid-session.

### 2. Real caching for CI (the actual fix for slow Vercel builds)

**Measured, not assumed**: a `cargo-leptos` project deployed to Vercel via
a custom `buildCommand` (`framework: null` in `vercel.json`, required
since there's no first-class Leptos preset) gets **no persistent cache for
`~/.cargo` or `target/`** between deploys — Vercel's build cache for that
framework setting only covers `node_modules/**` and lockfiles. Every
single deploy recompiles the entire Rust/Leptos dependency tree from
scratch. A real cold `vercel --prod` deploy of `kittine-website` took
**6m22s** wall-clock, ~4m45s of it pure dependency compilation — no
`[profile.release]` tuning fixes this, because the cost is in *other
crates'* compilation, not the project's own.

A compile-speed-favoring release profile still helps (this part of the
build genuinely does speed up, and it's free):

```toml
[profile.release]
opt-level = 1     # not "s"/"z" -- those trade compile time for binary size
lto = false       # LTO disables per-codegen-unit parallelism entirely
strip = true      # post-compile, nearly free, keep this one
```

But the actual fix is moving the build off Vercel's build machine
entirely: build once in **GitHub Actions**, where `actions/cache` *can*
genuinely persist `~/.cargo`/`target/` across runs (keyed on
`Cargo.lock`'s hash — unchanged deps means a full cache hit and a build
that finishes in well under a minute), then ship the result to Vercel as
a **prebuilt deployment** (`vercel deploy --prebuilt`), so Vercel does no
compilation at all. See `kittine-website/.github/workflows/deploy.yml`
for the full, working version — the shape:

1. `actions/cache` on `~/.cargo/registry`, `~/.cargo/git`, `target/`,
   keyed on `hashFiles('Cargo.lock')`.
2. Run the project's own build script from above (reuse it — don't
   duplicate its steps into the CI config).
3. Assemble a [Build Output API
   v3](https://vercel.com/docs/build-output-api/v3) directory by hand
   (`.vercel/output/static/` = a copy of the static export,
   `.vercel/output/config.json` = `{"version":3}`) — this is what lets
   `vercel deploy --prebuilt` skip running `buildCommand` again inside
   Vercel's own build step.
4. `vercel deploy --prebuilt --prod --token=$VERCEL_TOKEN`.

Two one-time manual setup steps this can't do for you (both deliberately
require a human, not automation): a `VERCEL_TOKEN` repo secret, and
turning off the project's native Git auto-deploy in the Vercel dashboard
so it stops racing the fast path on every push.

## Why this is also "compatible with all hosting providers"

Nothing in the artifact this pattern produces (`vercel-static/` — plain
HTML, a `pkg/` WASM bundle, and static assets) is Vercel-specific. The
*only* Vercel-specific piece is the final upload step
(`vercel deploy --prebuilt`); the exact same GitHub Actions artifact
deploys to Netlify, Cloudflare Pages, GitHub Pages, or a plain S3 +
CloudFront bucket by swapping only that last step. A `cargo-leptos`
SSR-then-static-export Kittine site was never actually Vercel-locked —
it just didn't have a documented fast path to prove it before now.

## What "fastest language ever" would actually require

Said plainly, once, so it doesn't need repeating in every doc that
mentions build speed: Kittine's *runtime* speed is Rust/WebAssembly's
runtime speed — real, competitive, but not a claim unique to Kittine,
since the language erases to nothing at compile time (see [LANGUAGE.md §
Compilation model](LANGUAGE.md#compilation-model)) and adds no runtime of
its own. "Fastest language ever made" isn't a claim this project makes
anywhere else in its docs, and this file doesn't start now — what's real
and shipped is faster *builds* (this doc) and a syntax that's shorter to
*write* (see [LANGUAGE.md § Brevity by
design](LANGUAGE.md#brevity-by-design)), which are the two things
actually under Kittine's control.
