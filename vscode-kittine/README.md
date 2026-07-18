# Kittine for VS Code

Syntax highlighting and file-type support for [Kittine](..) (`.kitty`), a
small language with a deliberately distinctive syntax for state and control
flow, plus an embedded JSX-like view syntax. Kittine compiles to idiomatic
[Leptos 0.7](https://leptos.dev) Rust, which in turn compiles to WebAssembly
and runs client-side in the browser.

![Kittine](images/kittine-icon.png)

This is an **editor-only** extension — comparable to what a TextMate grammar
gives you for any language before a full language server exists. It does
**not** compile, type-check, or run Kittine itself. For that you need the
[`kittine-compiler`](../kittine-compiler) CLI or the
[`vite-plugin-kittine`](../vite-plugin-kittine) Vite plugin, both in the
parent [`kittine`](..) project.

## Features

- **Full syntax highlighting** for every Kittine-specific construct:

  | Kittine syntax | What it is | Highlighted as |
  |---|---|---|
  | `func Name() { ... }` | Component declaration | `storage.type.function` |
  | `<{name}>` | Variable/signal read or declaration site | `variable.other.kittine` |
  | `<{name}> >> value` | Combined declare **and** mutate (no separate `let`/`set`) | `keyword.operator.assignment` for `>>` |
  | `if>` / `orif>` / `else>` | Control flow (`orif>` = "or if", i.e. `else if`) | `keyword.control.conditional` |
  | `<{name}> >> value` *inside* `if>`/`orif>` | Equality test, not assignment — same tokens, different scope by position | `keyword.control.conditional` |
  | `craft<...>` | Console logging | `keyword.other.craft` |
  | `'...'` / `"..."` | Fully interchangeable single- and double-quoted strings | `string.quoted.single` / `string.quoted.double` |
  | `yes>` / `no>` | Boolean literals | `constant.language.boolean` |
  | `[a, b, c]` | Array literal | `punctuation.definition.array` |
  | `<<Num>>` / `<<Word>>` / `<<Flag>>` | Optional type tags, checked against literals at compile time | `storage.type.kittine` |
  | `spin<{item}> in list }{ .. }{` | Loop over an array, fenced by `}{` | `keyword.control.loop` |
  | `func Name(<<Type>> prop) { .. }` | Component with typed props | `storage.type.function` |
  | `purr name(<<Type>> arg) <<Type>> { .. }` | Plain value-returning function | `storage.type.function` |
  | `import { A, B } from '...'` | Bring components/functions in from another file | `keyword.control.import` |
  | `private func/purr Name(..)` | Not importable elsewhere (Rust-enforced) | `storage.modifier` |
  | `>>` / `<` / `<=` / `>` / `>=` / `!=` | Comparisons, usable generally not just in conditions | `keyword.operator.assignment` / `keyword.operator.comparison` |
  | `<<Num[]>>` / `<<Word[]>>` / `<<Flag[]>>` | Array-typed prop/return tags | `storage.type.kittine` |
  | `<Router>`/`<Routes>`/`<Route>`/`<A>` | `leptos_router`, composed like any component — no Kittine-specific syntax | `entity.name.tag` |
  | `return ( <jsx> )` | The embedded JSX-like view syntax that closes a component | `entity.name.tag`, `entity.other.attribute-name` |
  | `// comment` | Line comments (no block comment form exists) | `comment.line.double-slash` |

  Because these map onto standard TextMate scopes, any color theme that
  already styles those scopes — which is effectively all of them — colors
  Kittine source sensibly with zero Kittine-specific theme setup.

- **A distinct file icon** for `.kitty` files, layered on top of whatever
  icon theme you already have (Seti, Material Icon Theme, etc.) — no need to
  switch themes to see it.

- **Bracket matching and auto-closing** tuned to Kittine's custom
  delimiters, not just the generic ones:
  - `<{` ↔ `}>` (variable brackets)
  - `'` ↔ `'` (single-quoted strings)
  - `"` ↔ `"` (double-quoted strings)
  - `(` ↔ `)`, `{` ↔ `}`, and `[` ↔ `]` (expressions, JSX, blocks, arrays)

- **Indentation-aware editing** for `if>` / `orif>` / `else>` chains. Kittine
  blocks are column-delimited rather than brace-delimited (see the
  [language reference](../docs/LANGUAGE.md#control-flow)), so the extension
  configures VS Code's auto-indent to increase after `if>`/`orif>`/`else>`
  and dedent on a sibling `orif>`/`else>`.

- **Line commenting** via `Ctrl+/` / `Cmd+/` uses Kittine's `//` correctly.

## Getting started

1. Install the extension (see below).
2. Open any `.kitty` file — the `Kittine` language mode activates
   automatically, no configuration needed.
3. See the [language reference](../docs/LANGUAGE.md) in the main Kittine
   project for the full syntax spec, or
   [GETTING_STARTED.md](../docs/GETTING_STARTED.md) for how to scaffold and
   run an actual Kittine project.

A minimal `.kitty` file, to sanity-check highlighting once installed:

```kitty
func App() {
    <{count}> >> 0

    return (
        <div>
            <button onClick={<{count}> >> count + 1}>
                "Clicks: "
                <{count}>
            </button>
        </div>
    )
}
```

## Installation

This extension isn't published on the VS Code Marketplace or Open VSX —
install it from the `.vsix` file directly.

### From the command line

```sh
cd vscode-kittine
npx @vscode/vsce package --no-rewrite-relative-links
code --install-extension kittine-lang-0.3.0.vsix
```

Restart VS Code, or reload the window (`Ctrl+Shift+P` → "Developer: Reload
Window"), if `.kitty` files don't pick up highlighting immediately.

### From the VS Code UI

1. Build (or download) `kittine-lang-0.3.0.vsix`.
2. Open the Extensions view (`Ctrl+Shift+X`).
3. Click the `...` menu → **Install from VSIX...** → select the file.

### Sharing it

The `.vsix` is a self-contained installable file — send it directly, or
attach it to a GitHub release. No marketplace account is required for
anyone to install it with `code --install-extension`.

## Requirements

- VS Code `^1.85.0`.
- Nothing else for highlighting itself. To actually compile `.kitty` files
  you need Rust and the `kittine-compiler` CLI — see
  [CLI.md](../docs/CLI.md) — or the Vite plugin.

## Known limitations

This extension mirrors the compiler's current scope; it does not add
features beyond what Kittine itself supports yet (see
[Known limitations](../docs/LANGUAGE.md#known-limitations) in the language
reference for the full list — no logical `&&`/`||`, no SSR/SSG). There is
also no language server: no diagnostics, no completions, no hover info, no
go-to-definition — purely TextMate-grammar-based highlighting.

## Rebuilding after changes

If you edit the grammar (`syntaxes/kittine.tmLanguage.json`), the language
configuration (`language-configuration.json`), or the manifest
(`package.json`), re-package and reinstall:

```sh
cd vscode-kittine
npx @vscode/vsce package --no-rewrite-relative-links
code --install-extension kittine-lang-<version>.vsix --force
```

`--force` overwrites the already-installed version. The
`--no-rewrite-relative-links` flag keeps this README's relative image/link
paths pointing at the files bundled inside the `.vsix` (this repository is
private, so `vsce`'s default behavior of rewriting them to absolute GitHub
URLs would produce a broken image once installed).

## Author

Kittine was created by **Sivario Buchanan**. Licensed under [MIT](LICENSE).
