# `kittine-compiler` CLI Reference

`kittine-compiler` is the standalone Rust binary that lexes, parses, and
lowers a single `.kitty` file into a Leptos 0.7 `.rs` file. It has no
dependency on Vite or Node — the Vite plugin just shells out to it.

## `kittine-compiler build <input> [--output <path>]`

Compiles `<input>` and writes the generated Rust source.

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

- `0` on success; prints `kittine-compiler: wrote <path>` to stdout.
- non-zero on any lex, parse, or I/O error; prints
  `kittine-compiler: error: <message>` to stderr, including a `line:col`
  position for syntax errors.

### Other flags

- `--version` — print the compiler version.
- `--help` — print usage.

## Using it outside Vite

Because the CLI only needs a file path, it's straightforward to call from
any build system, a pre-commit hook, or a plain shell script/CI job to keep
generated `.rs` files in sync:

```sh
for f in src/**/*.kitty; do
  kittine-compiler build "$f"
done
```
