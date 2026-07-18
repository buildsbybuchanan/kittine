# Security Policy

Kittine is an early-stage, single-maintainer language project. This policy
is scoped to what that actually means in practice — please read it before
filing a report so expectations match reality.

## Supported versions

There are no tagged releases yet; only the `main` branch is maintained.
Security fixes land on `main` and are noted in [CHANGELOG.md](CHANGELOG.md).

## Threat model

Kittine's compiler (`kittine-compiler`) is a source-to-source transpiler:
it lexes and parses a `.kitty` file and emits Rust source, which you then
build yourself with `cargo`/`wasm-bindgen`. As with any compiler, **do not
run `kittine-compiler` on `.kitty` source you don't trust** — the same
caution you'd apply to running any unfamiliar build tool on unfamiliar
input applies here.

Within that model, the security-relevant properties this project does
promise are:

- The lexer and parser should never panic, hang, or exhibit undefined
  behavior on any input, malformed or otherwise — a crash or infinite loop
  on adversarial `.kitty` input is a real bug, please report it.
- Code generation must correctly escape string content when emitting Rust
  string literals (see `escape_str` in `kittine-compiler/src/codegen.rs`).
  A `.kitty` string literal that can make its escaped content "break out"
  of the generated Rust string and inject arbitrary Rust source is a real
  vulnerability — please report it.
- The `vscode-kittine` extension does syntax highlighting only; it does not
  execute `.kitty` file contents.

## Reporting a vulnerability

This repository is currently private, so GitHub's public security-advisory
workflow isn't available. To report a security issue:

1. Open an issue in this repository with as much detail as you can share
   (affected file/function, a minimal reproducing `.kitty` snippet, and
   what you'd expect to happen instead).
2. If the issue is sensitive enough that you'd rather not put full details
   in a repository issue, reach the maintainer, Sivario Buchanan, through
   [buildsbybuchanan.com](https://buildsbybuchanan.com) and reference this
   repository.

There is no bug bounty and no guaranteed response SLA at this stage of the
project — but every report will get a reply, and confirmed issues will be
credited in the changelog unless you ask otherwise.

## Disclosure

Given this is a young project with no external users depending on a
specific release cut yet, there's no formal embargo process. Please still
give the maintainer a reasonable chance to land a fix before writing about
a vulnerability publicly.
