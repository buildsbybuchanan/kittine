# Contributing to Kittine

Kittine is a small, hand-rolled language project — contributions are
welcome, but keep in mind the goal is to keep the compiler simple enough
that one person can hold the whole pipeline (lexer → parser → codegen) in
their head at once.

## Project layout

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the pieces fit
together before diving in.

## Making changes to the language

If you're adding or changing syntax:

1. Update [docs/LANGUAGE.md](docs/LANGUAGE.md) first — treat it as the
   spec, not an afterthought.
2. Update `kittine-compiler/src/lexer.rs` (new tokens), `parser.rs` (new
   grammar), and `codegen.rs` (new lowering) together — the three stay in
   lockstep by design.
3. Add a test to `kittine-compiler/src/tests.rs` exercising the new syntax
   end-to-end (source in, generated Rust out).
4. If the change affects highlighting, update
   `vscode-kittine/syntaxes/kittine.tmLanguage.json` and re-package the
   extension (see [docs/VSCODE_EXTENSION.md](docs/VSCODE_EXTENSION.md)).

## Running the test suite

```sh
cd kittine-compiler
cargo test
```

## Verifying end-to-end

Nothing beats actually compiling and running the example app after a
compiler change:

```sh
npm install
npm run build:plugin
npm run dev
```

then editing `example-app/src/App.kitty` to exercise whatever you changed.

## Style

- Rust: default `rustfmt`, no unusual conventions.
- TypeScript: matches the existing `vite-plugin-kittine/src/index.ts` —
  explicit types on exported functions, no `any`.
- Keep the codegen output human-readable; it's meant to be inspectable,
  not just correct.
