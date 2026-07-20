# `kittine-compiler` CLI Reference

`kittine-compiler` is the standalone Rust binary that lexes, parses, and
lowers a single `.kitty` file into a Leptos 0.7 `.rs` file. It has no
dependency on Vite or Node — the Vite plugin just shells out to it.

## `kittine-compiler build <input> [--output <path>]`

Compiles `<input>` and writes the generated Rust source. Every `import { .. }
from '<path>'` reachable from `<input>` is compiled too, recursively (cycle
detection included), into the sibling `.rs` path each import expects — so
running this on your app's entry point regenerates the whole import graph in
one call. A dependency's output file is only actually rewritten if its
generated content changed; a `.kitty` file that recompiles to
byte-identical Rust leaves its `.rs` file's mtime untouched, so downstream
tools that key off mtimes (`cargo`, `wasm-bindgen`, Vite's own file watcher)
don't redo work for files that didn't really change.

```sh
kittine-compiler build src/App.kitty
# -> writes src/App.rs

kittine-compiler build src/App.kitty --output src/generated.rs
# -> writes src/generated.rs
```

- `<input>` — path to a `.kitty` file. Required.
- `-o, --output <path>` — output path for the generated `.rs` file.
  Defaults to `<input>` with its extension replaced by `.rs`.

### Exit status

- `0` on success; prints `kittine-compiler: wrote <path>` to stdout if
  `<input>`'s own output changed, or `kittine-compiler: <path> is already
  up to date` if it recompiled to byte-identical content.
- non-zero on any lex, parse, or I/O error; prints
  `kittine-compiler: error: <message>` to stderr, including a `line:col`
  position for syntax errors.

### Other flags

- `--version` — print the compiler version.
- `--help` — print usage.

## `kittine-compiler fmt <path> [--check] [--force]`

Reformats `.kitty` source into a canonical style. `<path>` is a single file
or a directory (every `.kitty` file under it, recursively). Every reformat
is self-verified: the output is reparsed and its AST compared against the
original before anything is written, so a formatter bug fails loudly
instead of silently corrupting a file.

- `--check` — don't write; exit non-zero if anything would change.
- `--force` — reformat files containing `//` comments too, losing them
  (the lexer discards comments before parsing, so `fmt` has nothing to
  preserve them with; it skips such files by default rather than
  silently dropping content).

## `kittine-compiler lint <path>`

Lints `.kitty` source for genuine issues: unused imports, unused `private`
items/params/`hold` bindings, and duplicate field/variant/method/param
names that would fail once generated to Rust (e.g. `cargo build`'s
`E0124`) — caught here first. `<path>` is a single file or a directory.
Exits non-zero if any warnings were found.

## `kittine-compiler add <name> [--version <version>]`

Adds `<name>` as a dependency in this directory's `kittine.toml`, creating
a minimal manifest first if one doesn't exist yet. Resolves against the
[package registry](#the-package-registry) to find the latest published
version if `--version` is omitted. Doesn't download anything — run
`install` afterward to actually fetch it.

## `kittine-compiler install`

Resolves every dependency listed in `kittine.toml` against the registry,
downloads each tarball, verifies its sha256 against the registry index
(refusing to extract on a mismatch), and extracts it into
`kitten_modules/<name>/`. Writes the exact resolved set — name, version,
checksum, source — to `kittine.lock`, so a second `install` elsewhere gets
byte-identical dependencies rather than whatever happens to be latest that
day.

A bare-name import — `import { X } from 'some-package'` (no `./` prefix, no
`.kitty` extension) — resolves to `kitten_modules/some-package/lib.kitty`,
searched upward from the importing file's own directory (the same
upward-search `node_modules` resolution uses), so it works the same from
any file in the project.

## `kittine-compiler publish`

Publishes the current directory as a package: requires a `kittine.toml`
with `[package] name`/`version` set, and a `lib.kitty` at the package root
(the fixed entry point a dependent's bare-name import resolves against).
Packs the directory into a `.tar.gz` (excluding `target/`, `.git/`,
`node_modules/`, and `kitten_modules/`), then uploads it as a new version
via the `gh` CLI, which must already be authenticated with write access to
the registry repo. Refuses to overwrite an already-published version.

## The package registry

There's no server — the registry is the public
[`buildsbybuchanan/kittine-registry`](https://github.com/buildsbybuchanan/kittine-registry)
repo. `add`/`install` only ever do plain, unauthenticated HTTPS `GET`s
against it (an `index/<name>.json` per package, tarballs as GitHub Release
assets), so installing a Kittine package needs nothing beyond internet
access. `publish` is the one maintainer-only operation and needs the `gh`
CLI. See that repo's README for the exact index format.

## Using it outside Vite

Because the CLI only needs a file path, it's straightforward to call from
any build system, a pre-commit hook, or a plain shell script/CI job to keep
generated `.rs` files in sync:

```sh
for f in src/**/*.kitty; do
  kittine-compiler build "$f"
done
```
