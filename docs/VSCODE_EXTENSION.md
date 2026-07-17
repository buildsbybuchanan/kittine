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
npx @vscode/vsce package
code --install-extension kittine-lang-0.1.0.vsix
```

Restart VS Code (or reload the window: `Ctrl+Shift+P` → "Developer: Reload
Window") if `.kitty` files don't pick up highlighting immediately.

### Sharing it with friends

The `.vsix` produced by `vsce package` is a self-contained installable
file — send it directly, or attach it to a GitHub release, and anyone can
run:

```sh
code --install-extension kittine-lang-0.1.0.vsix
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
| `>>` | `keyword.operator.assignment` |
| `if>` / `orif>` / `else>` | `keyword.control.conditional` |
| `craft<...>` | `keyword.other.craft` |
| `¨...¨` and `"..."` | `string.quoted.*` |
| `func`, `return` | `storage.type.function` / `keyword.control.flow` |
| JSX tags/attributes | `entity.name.tag`, `entity.other.attribute-name` |
| `// comment` | `comment.line.double-slash` |

Any VS Code color theme that styles the standard TextMate scopes above
(essentially all of them) will color Kittine source sensibly without any
Kittine-specific theme customization.

## Rebuilding the extension after changes

If you edit the grammar (`syntaxes/kittine.tmLanguage.json`) or the
manifest (`package.json`), re-package and re-install:

```sh
cd vscode-kittine
npx @vscode/vsce package
code --install-extension kittine-lang-<version>.vsix --force
```

`--force` overwrites the already-installed version.
