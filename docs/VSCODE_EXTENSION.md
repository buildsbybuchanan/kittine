# Kittine VS Code Extension

The [`vscode-kittine`](../vscode-kittine) extension gives `.kitty` files
proper editor support in VS Code: syntax highlighting for every Kittine
construct, a dedicated file icon, and bracket matching/auto-closing tuned
to Kittine's custom operators.

It does **not** compile or type-check Kittine — that's the job of
`kittine-compiler` / the Vite plugin (see [CLI.md](CLI.md) and
[GETTING_STARTED.md](GETTING_STARTED.md)). This is an editor-only
extension, comparable to what a TextMate grammar gives you for any
language before a full language server exists.

## Installing it

### From a built `.vsix` (what you have right now)

```sh
cd vscode-kittine
npx @vscode/vsce package --no-rewrite-relative-links
code --install-extension kittine-lang-0.4.0.vsix
```

Restart VS Code (or reload the window: `Ctrl+Shift+P` → "Developer: Reload
Window") if `.kitty` files don't pick up highlighting immediately.

`--no-rewrite-relative-links` matters here: by default `vsce` rewrites
relative links/images in `README.md` into absolute URLs pointing at the
GitHub repo in `package.json#repository`, so they render on the Marketplace
web page. That repo is currently **private**, so those URLs 404 — without
the flag, the extension's icon shows as a broken-image placeholder once
installed. The flag keeps the README's image reference relative, so VS
Code resolves it against the file bundled inside the `.vsix` instead.

### Sharing it with friends

The `.vsix` produced by `vsce package` is a self-contained installable
file — send it directly, or attach it to a GitHub release, and anyone can
run:

```sh
code --install-extension kittine-lang-0.4.0.vsix
```

No marketplace publishing step is required for this. If you do want it
listed on the [VS Code
Marketplace](https://marketplace.visualstudio.com/vscode) or
[Open VSX](https://open-vsx.org/) later, `vsce publish` / `ovsx publish`
work from this same package once you register a publisher account —
nothing in this extension's structure needs to change to support that.

## What it highlights

| Kittine syntax | Scope |
|---|---|
| `<{name}>` | `punctuation.definition.variable.*` / `variable.other.kittine` |
| `>>` / `<` / `<=` / `>` / `>=` / `!=` | `keyword.operator.assignment` / `keyword.operator.comparison` |
| `&&` / `\|\|` | `keyword.operator.logical` |
| `if>` / `orif>` / `else>` | `keyword.control.conditional` |
| `craft<...>` | `keyword.other.craft` |
| `'...'` and `"..."` | `string.quoted.*` |
| `yes>` / `no>` | `constant.language.boolean` |
| `<<Num>>` / `<<Word>>` / `<<Flag>>` (or `[]` for an array of one) | `storage.type.kittine` |
| `spin` / `in` | `keyword.control.loop` |
| `[` / `]` | `punctuation.definition.array` |
| `func`, `purr`, `return` | `storage.type.function` / `keyword.control.flow` |
| `private` | `storage.modifier` |
| `import`, `from` | `keyword.control.import` |
| JSX tags/attributes (incl. `leptos_router`'s `Router`/`Routes`/`Route`/`A`) | `entity.name.tag`, `entity.other.attribute-name` |
| `// comment` | `comment.line.double-slash` |

Any VS Code color theme that styles the standard TextMate scopes above
(essentially all of them) will color Kittine source sensibly without any
Kittine-specific theme customization.

## Rebuilding the extension after changes

If you edit the grammar (`syntaxes/kittine.tmLanguage.json`) or the
manifest (`package.json`), re-package and re-install:

```sh
cd vscode-kittine
npx @vscode/vsce package --no-rewrite-relative-links
code --install-extension kittine-lang-<version>.vsix --force
```

`--force` overwrites the already-installed version.
