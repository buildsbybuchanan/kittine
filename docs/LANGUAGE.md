# The Kittine Language Reference

Kittine (`.kitty`) is a small language with a deliberately unusual, distinctive
syntax for state and control flow, plus an embedded JSX-like view syntax. It
compiles to idiomatic [Leptos 0.7](https://leptos.dev) Rust, which in turn
compiles to WebAssembly and runs client-side in the browser.

This document is the authoritative syntax and semantics reference. For how to
actually build and run a Kittine project, see [GETTING_STARTED.md](GETTING_STARTED.md).

## Table of contents

- [Components](#components)
- [Props](#props)
- [Functions (`purr`)](#functions-purr)
- [Calling functions](#calling-functions)
- [Modules and imports](#modules-and-imports)
- [Component composition](#component-composition)
- [Variables and state (`<{ }>` / `>>`)](#variables-and-state)
- [Strings](#strings)
- [Booleans (`yes>` / `no>`)](#booleans)
- [Arrays (`[ ]`)](#arrays)
- [Type tags (`<<Type>>`)](#type-tags)
- [Printing (`craft<...>`)](#printing-craft)
- [Control flow (`if>` / `orif>` / `else>`)](#control-flow)
- [Loops (`spin` / `}{`)](#loops)
- [Expressions and operators](#expressions-and-operators)
- [Comments](#comments)
- [The view syntax (`return ( ... )`)](#the-view-syntax)
- [Full grammar summary](#full-grammar-summary)
- [Compilation model](#compilation-model)
- [Known limitations](#known-limitations)

## Components

A Kittine file is a sequence of component and function definitions:

```kitty
func App() {
    ...
}
```

`func Name(<<Type>> prop, ..) { ... }` declares a component named `Name`.
Every component body may contain any number of statements, followed by
exactly one `return ( ... )` view expression as its last meaningful element.

A component compiles to a Leptos `#[component] pub fn Name(..) -> impl IntoView { ... }`.

## Props

```kitty
func Nav(<<Word>> active) {
    return ( <span>{ active }</span> )
}
```

Components can take parameters — Kittine's term for these is **props**,
matching how they're used: values passed in from a parent when the
component is composed into another view (see [Component
composition](#component-composition)). Unlike `<{name}> >> value` signals,
a prop is a plain value, not reactive state — there's no setter, and
reading it is just the bare name (`active`, not `<{active}>`).

Every prop must carry an explicit [type tag](#type-tags): `<<Num>>`,
`<<Word>>`, or `<<Flag>>`. There is no prop-type inference.

### Compilation

| Kittine | Generated Rust |
|---|---|
| `func Nav(<<Word>> active) { .. }` | `pub fn Nav(active: String) -> impl IntoView { .. }` |
| `func Card(<<Num>> price, <<Flag>> onSale) { .. }` | `pub fn Card(price: f64, onSale: bool) -> impl IntoView { .. }` |

Reading a `Word` (`String`) prop clones it (`active.clone()`) rather than
moving it, since a prop may be read from more than one reactive closure
inside the component body — `Num`/`Flag` props are `Copy` and don't need
this.

## Functions (`purr`)

```kitty
purr double(<<Num>> n) <<Num>> {
    return (n * 2)
}
```

`purr name(<<Type>> param, ..) <<ReturnType>> { .. return (expr) }` declares
a plain function: it computes and returns a value, and does not render a
view. Unlike a component, its signature also carries an explicit
**return-type** tag right after the parameter list, before the body's `{`.

A `purr` function is the idiomatic Kittine way to share logic (formatting,
computed values) between components without duplicating it.

### Compilation

`purr` becomes a plain, non-`#[component]` `pub fn`, with the body's
`return (expr)` becoming the function's tail expression (no `#[component]`
attribute, no `view!`):

```kitty
purr double(<<Num>> n) <<Num>> {
    return (n * 2)
}
```
```rust
pub fn double(n: f64) -> f64 {
    n * 2f64
}
```

## Calling functions

```kitty
craft<double(21)>
```

`name(arg, arg, ..)` calls a function — a `purr` you've defined, or one
brought into scope with [`import`](#modules-and-imports). A call is an
expression, valid anywhere an expression is: `craft<...>` arguments, JSX
`{ expr }` interpolations, variable declarations, conditions.

### Compilation

`name(args)` becomes a direct Rust call `name(args)`, with each argument
rendered exactly as it would be anywhere else (a signal read becomes
`.get()`, a `Word` value is cloned, and so on):

| Kittine | Generated Rust |
|---|---|
| `double(21)` | `double(21f64)` |
| `double(<{count}>)` | `double(count.get())` |

## Modules and imports

```kitty
import { Nav, Footer } from './components/Nav.kitty'
```

`import { Name, Name2 } from 'path/to/file.kitty'` brings one or more
components/functions from another `.kitty` file into scope. The path is
resolved relative to the importing file, and always ends in `.kitty`.
Imports must appear before any `func`/`purr` declarations in the file.

### Compilation

Each import becomes a Rust `mod` declaration pointing at the sibling `.rs`
file the imported `.kitty` file compiles to, plus a `use` bringing the
named items into scope:

```kitty
import { Nav } from './Nav.kitty'
```
```rust
#[path = "./Nav.rs"]
mod __kittine_mod_nav;
use __kittine_mod_nav::{Nav};
```

`kittine-compiler build` resolves this recursively: compiling a file that
imports others also compiles each imported file (and anything *it*
imports), writing every `.kitty` file in the import graph to its sibling
`.rs` path. An import cycle is a compile error, not an infinite loop.

## Component composition

```kitty
func Page() {
    return (
        <div>
            <Nav active='home' />
        </div>
    )
}
```

A JSX tag starting with an **uppercase** letter is a reference to another
component (defined locally or brought in via `import`) rather than an HTML
element — exactly the same rule
[Leptos's own `view!` macro](https://leptos.dev) uses. Attributes on a
component tag are its props, passed by value rather than as reactive DOM
attributes.

### Compilation

A component tag's attributes are passed as plain values (a string literal
prop becomes an owned `.to_string()`, not the bare `&str` an HTML attribute
would get) instead of being wrapped in the `move || ..` closure a real DOM
attribute needs for reactivity:

| Kittine | Generated Rust |
|---|---|
| `<Nav active='home' />` | `<Nav active="home".to_string()/>` |
| `<div title={label} />` | `<div title=move \|\| label.get()/>` |

## Variables and state

```kitty
<{count}> >> 0
```

`<{name}> >> value` is Kittine's single construct for both **declaring** and
**mutating** state, reused in three positions:

1. **As a statement**, the first time `name` appears in a component, it
   *declares* a reactive signal initialized to `value`.
2. **As a statement**, any later time `name` appears, it *mutates* the
   existing signal by re-evaluating `value` (which may reference the
   variable's own current value, e.g. `<{count}> >> count + 1`).
3. **Inline**, inside a JSX attribute expression (typically an event
   handler), e.g. `onClick={<{count}> >> count + 1}` — same mutation
   semantics as (2), just written where the event fires instead of at the
   top of the component.

A bare `<{count}>` (no `>>`) *reads* the current value of the variable.

The compiler tracks which names have already been declared in the current
component (in source order) to decide whether a given `<{name}> >> ...`
is a declaration or a mutation — there is no separate `let` vs. `set`
keyword.

### Compilation

| Kittine | Generated Rust |
|---|---|
| `<{x}> >> 0` (first occurrence) | `let (x, set_x) = signal(0f64);` |
| `<{x}> >> x + 1` (later occurrence) | `set_x.update(\|n\| *n += 1f64);` |
| `<{x}> >> x - 1` | `set_x.update(\|n\| *n -= 1f64);` |
| `<{x}> >> x * 2` | `set_x.update(\|n\| *n *= 2f64);` |
| `<{x}> >> 5` (later, non-self-referential) | `set_x.update(\|n\| *n = 5f64);` |
| `<{x}>` (read) | `x.get()` |

The `+= / -= / *= / /=` compound forms are only emitted when the right-hand
side is exactly `<selfname> <op> <number literal>`; any other mutation
expression lowers to the general `*n = <expr>;` form.

A whole-number literal always gets an explicit `f64` suffix wherever it's
an operand next to an already-concretely-typed `f64` value (a signal's
initializer, an arithmetic operand, a compound-assignment right-hand side).
This isn't cosmetic: `signal(0)` alone leaves `0`'s type to Rust's generic
inference, which is free to pick something other than `f64` if nothing else
in the function pins it down first — and it fails to compile the moment
that value is later passed somewhere that concretely requires `f64` (a
`purr` call, for instance). Spelling it `0f64` up front avoids the
ambiguity entirely.

## Strings

Kittine has **two** fully interchangeable string forms:

- **`'...'`** — single-quoted strings.
- **`"..."`** — double-quoted strings, commonly used for JSX text content
  (`"Clicks: "` in the examples).

Pick whichever quote character lets you avoid escaping the string's own
contents — both compile to exactly the same thing and can be used anywhere
a string literal is valid (variable values, `craft<...>` arguments,
conditions, array elements). Both forms support the escapes `\n`, `\t`,
`\\`, and an escaped version of their own quote character (`\'` inside
`'...'`, `\"` inside `"..."`). Both lower to a standard Rust `"..."` string
literal in the generated code.

## Booleans

```kitty
<{ready}> >> yes>
<{done}> >> no>
```

`yes>` and `no>` are Kittine's boolean literals — the `>` suffix matches the
same "keyword fused with punctuation" style as `if>` / `orif>` / `else>` /
`craft<`. They lower to Rust's `true` / `false`.

## Arrays

```kitty
<{scores}> >> [10, 20, 30]
craft<[1, 2, 3]>
```

`[expr, expr, ..]` is an array literal — a comma-separated list of
expressions inside square brackets. It lowers to Rust's `vec![..]`. Arrays
can hold any expression, including strings, booleans, and other arrays, and
can be declared as signal state exactly like any other value.

Because a `Vec` has no `Display` implementation, `craft<[..]>` formats
arrays with Rust's `{:?}` (`Debug`) formatter instead of `{}`.

## Type tags

```kitty
<{count}> >> <<Num>> 0
<{label}> >> <<Word>> 'hi'
<{ready}> >> <<Flag>> yes>
```

`<<Type>> value` is an explicit type tag — the idiomatic Kittine way to
annotate a value's type. There are three type names:

| Tag | Matches |
|---|---|
| `<<Num>>` | number literals |
| `<<Word>>` | string literals |
| `<<Flag>>` | boolean literals |

When the tagged value is a literal, the compiler checks it against the tag
at compile time and rejects a mismatch (`<<Num>> 'oops'` is a parse error).
When the tagged value is a variable read or a computed expression (its
static type isn't known at parse time), the annotation is trusted rather
than checked. Either way, the tag itself is erased during code generation —
Rust's own type inference already gives the underlying value the right
type, so `<<Num>> 0` and a bare `0` generate identical Rust.

Type tags are optional on a value (`<{count}> >> 0` and `<{count}> >>
<<Num>> 0` compile identically) — they exist for readability and for
catching literal type mistakes early, not because Kittine has (or needs) a
full static type system. They are **mandatory** in one place: every
[prop](#props) and [`purr` return type](#functions-purr), since a function
signature needs a real Rust type and Kittine has no inference for those
positions yet.

## Printing (`craft<...>`)

```kitty
craft<'hello world'>
```

`craft<expr>` logs `expr` to the browser console. String literals are
inlined directly; arrays are formatted with `{:?}`; everything else is
formatted with Rust's `{}` formatter:

| Kittine | Generated Rust |
|---|---|
| `craft<'hello'>` | `leptos::logging::log!("hello");` |
| `craft<<{count}>>` | `leptos::logging::log!("{}", count.get());` |
| `craft<[1, 2, 3]>` | `leptos::logging::log!("{:?}", vec![1, 2, 3]);` |

## Control flow

```kitty
if><{username}> >> 'Admin'
    craft<'Welcome Admin'>
orif><{username}> >> 'User'
    craft<'Welcome User'>
else>
    craft<'no output'>
```

- `if>`, `orif>` ("or if", Kittine's `else if`), and `else>` form a single
  chain, exactly like `if` / `else if` / `else` in Rust.
- **Conditions reuse the `<{name}> >> value` syntax as an equality test**:
  `if><{username}> >> 'Admin'` means "if `username` equals `"Admin"`", not
  "assign". The parser distinguishes this from a variable
  declaration/mutation purely by *position* — inside an `if>` / `orif>`
  condition, `>>` is always `==`.
- **Blocks are indentation-delimited, not brace-delimited.** A block
  belongs to the `if>` / `orif>` / `else>` that introduces it as long as its
  statements are indented further than that keyword's own column. A
  sibling `orif>` or `else>` must sit at exactly the same column as the
  `if>` that started the chain. There is no fixed indent width requirement
  (2 spaces, 4 spaces, and tabs-as-one-column all work) — what matters is
  that the block's column is strictly greater than the keyword's column,
  and siblings match it exactly.
- `if>` / `orif>` / `else>` blocks can be nested arbitrarily.

### Compilation

`if>` / `orif>` / `else>` become `if` / `else if` / `else`; a `>>` condition
becomes `==`, and a bare variable inside a condition becomes `.get()`:

```rust
if username.get() == "Admin" {
    leptos::logging::log!("Welcome Admin");
} else if username.get() == "User" {
    leptos::logging::log!("Welcome User");
} else {
    leptos::logging::log!("no output");
}
```

## Loops

```kitty
spin<{item}> in [1, 2, 3] }{
    craft<item>
}{
```

`spin<{item}> in <list> }{ .. }{` iterates `list`, binding each element to
`item` for the body. `list` can be an array literal or a `<{name}>` read of
a previously declared signal holding an array.

The `}{` fence — a closing brace immediately followed by an opening one — is
Kittine's loop-body delimiter. It's deliberately the mirror image of `{ }`:
a loop is a block that folds back on itself. The same two characters open
and close the body; the parser tracks which is which by position, not by a
different symbol.

### Compilation

`spin` becomes a plain Rust `for` loop; the loop variable is a bare local
(not a reactive signal), so reads of it inside the body are unwrapped, not
`.get()`-ed:

```rust
for item in (vec![1, 2, 3]).into_iter() {
    leptos::logging::log!("{}", item);
}
```

See [Known limitations](#known-limitations) for what `spin` does not (yet)
do — namely, render its items into the view.

## Expressions and operators

Precedence, lowest to highest:

1. `>>` — equality (only inside an `if>` / `orif>` condition, or as the
   top-level operator of a `<{name}> >> value` assignment)
2. `+` `-` (addition, subtraction — left-associative)
3. `*` `/` (multiplication, division — left-associative)
4. unary `-` (negation)
5. primary: numbers, strings, identifiers, `<{name}>` reads, parenthesized
   expressions

Numbers are floating point (`f64` internally); integer-valued literals are
rendered back as bare Rust integer literals (`0`, `1`, `42`) rather than
`0.0`, `1.0`, etc., so generated code reads naturally.

### `+` as string concatenation

Kittine has no type system, so `+` can't be checked ahead of time as "numeric"
or "string." Instead, the compiler looks at whether either side of a `+` is
literally a string (`'...'` or `"..."`):

- If **neither** side is a string literal, `+` lowers to Rust's numeric `+`,
  exactly as before (`<{count}> >> count + 1` → `*n += 1`).
- If **either** side is a string literal, the whole `+` lowers to
  `format!("{}{}", left, right)`, `Display`-formatting both operands. This
  means a variable on the other side doesn't need to be a string itself —
  numbers interpolate naturally:

  | Kittine | Generated Rust | Result (given `count` is `5`) |
  |---|---|---|
  | `'Taps: ' + <{count}>` | `format!("{}{}", "Taps: ", count.get())` | `"Taps: 5"` |
  | `<{mood}> + '!'` | `format!("{}{}", mood.get(), "!")` | e.g. `"Curious!"` |

  Chains resolve left-associatively as usual, so `'a' + x + 'b'` parses as
  `('a' + x) + 'b'` — the inner `+` already sees a string literal and
  becomes a `format!`, and the outer `+` sees `'b'` as a literal and does
  the same, so the whole chain concatenates as expected regardless of what
  `x` is.

  This works anywhere an expression is valid: `craft<...>` arguments, JSX
  `{ expr }` interpolations, variable declarations/mutations, and inline
  event-handler assignments.

> **Watch out:** because `>` and `>>` are lexed greedily, a `+`-expression
> ending in a variable read right before a closing `>` can accidentally
> merge two adjacent `>` characters into a single `>>` token, e.g.
> `craft<'Taps: ' + <{count}>>` — the `}>` closing the variable read and the
> `>` closing `craft<...>` collide. Add a space before the final bracket
> (`craft<'Taps: ' + <{count}> >`) to keep them separate tokens.

## Comments

`// like this`, to end of line. There is no block comment syntax.

## The view syntax

```kitty
return (
    <div>
        <button onClick={<{count}> >> count + 1}>
            "Clicks: "
            <{count}>
        </button>
    </div>
)
```

`return ( <jsx> )` is the last statement in a component and defines its
rendered output. The JSX-like tree supports:

- **Elements**: `<tag attr={expr} attr="literal">children</tag>` or
  self-closing `<tag />`.
- **Text children**: `"literal text"`.
- **Variable interpolation**: a bare `<{name}>` as a child renders that
  signal's current value reactively.
- **Expression interpolation**: `{ expr }` as a child renders an arbitrary
  expression reactively.
- **Event handlers**: any attribute named `on<Event>` (`onClick`,
  `onInput`, `onSubmit`, ...) becomes a Leptos `on:<event>=` binding, and
  its value is wrapped in a `move |_| ...` closure. A `<{name}> >> value`
  expression used as an event handler's value performs the mutation when
  the event fires.
- **Other attributes**: rendered as-is; string literals stay strings,
  `{expr}` attribute values are wrapped in a `move || ...` closure so they
  stay reactive.

### Compilation

- Elements become literal Leptos `view!{}` markup: `<div>...</div>`.
- `<{name}>` as a child becomes `{move || name.get()}`.
- `{expr}` as a child becomes `{move || <expr>}`.
- `onClick={...}` becomes `on:click=move |_| <mutation>`.
- Any other `attr={expr}` becomes `attr=move || <expr>`.

## Full grammar summary

```
program      := import* item*
item         := component | function
import       := "import" "{" IDENT ("," IDENT)* "}" "from" STRING
component    := "func" IDENT param_list "{" stmt* return_stmt? "}"
function     := "purr" IDENT param_list type_tag_name
                "{" stmt* return_stmt? "}"
param_list   := "(" (param ("," param)*)? ")"
param        := type_tag_name IDENT
type_tag_name:= "<<" ("Num" | "Word" | "Flag") ">>"
return_stmt  := "return" "(" jsx_node | expr ")"

stmt         := var_stmt | craft_stmt | if_stmt | spin_stmt | expr_stmt
var_stmt     := "<{" IDENT "}>" ">>" expr
craft_stmt   := "craft<" expr ">"
if_stmt      := "if>" condition INDENT_BLOCK
                ("orif>" condition INDENT_BLOCK)*
                ("else>" INDENT_BLOCK)?
condition    := "<{" IDENT "}>" ">>" expr
spin_stmt    := "spin" "<{" IDENT "}>" "in" expr "}{" stmt* "}{"
expr_stmt    := expr

expr         := additive (">>" additive)?
additive     := term (("+" | "-") term)*
term         := unary (("*" | "/") unary)*
unary        := "-" unary | primary
primary      := NUMBER | STRING | BOOL | ARRAY | TYPE_TAG | CALL | IDENT
              | "<{" IDENT "}>" (">>" expr)?
              | "(" expr ")"

array        := "[" (expr ("," expr)*)? "]"
type_tag     := type_tag_name unary
call         := IDENT "(" (expr ("," expr)*)? ")"

jsx_node     := jsx_element | STRING | "<{" IDENT "}>" | "{" expr "}"
jsx_element  := "<" IDENT jsx_attr* ("/>" | ">" jsx_node* "</" IDENT ">")
jsx_attr     := IDENT "=" (STRING | "{" expr "}")

STRING       := "'" char* "'" | '"' char* '"'
BOOL         := "yes>" | "no>"
```

## Compilation model

Every generated Rust file starts with:

```rust
// Generated by kittine-compiler. Do not edit by hand.
#![allow(unused_braces, unused_variables, dead_code)]

use leptos::prelude::*;
```

followed by one `mod` + `use` pair per `import`, and then one item per
`func`/`purr` in the source file, in source order: a `func` becomes
`#[component] pub fn Name(..) -> impl IntoView { ... }`; a `purr` becomes a
plain `pub fn name(..) -> ReturnType { ... }`.

## Known limitations

These are intentional scope boundaries of the current prototype, not bugs:

- **No prop type inference.** Every prop and `purr` return type needs an
  explicit `<<Num>>`/`<<Word>>`/`<<Flag>>` tag — there's no way to omit it
  and have the compiler figure it out.
- **No children for composed components.** `<Nav>...</Nav>` parses fine,
  but nothing wires JSX children through to a Kittine-defined component's
  props (there's no `children`-prop concept yet) — compose with
  attributes only (`<Nav active='home' />`), or self-close.
- **`import` only brings in items, not re-exports.** There's no `export`
  concept — every `func`/`purr` in a file is implicitly `pub` and
  importable; you can't restrict what a file exposes.
- **No array element/return types.** `<<Num>>`/`<<Word>>`/`<<Flag>>` cover
  scalars; there's no tag (yet) for "an array of `Num`", so array-typed
  props or `purr` returns aren't expressible.
- **`spin` loops are imperative, not reactive view rendering.** `spin`
  lowers to a plain Rust `for` loop, which is useful for logic (`craft<...>`,
  computing values) that runs once at component setup. There is no
  list-rendering (`<For>`) support yet — a `spin` loop cannot appear inside
  `return ( ... )` to render one element per item.
- **`craft<...>` inside `if>`/`orif>`/`else>`/`spin` runs once, at component setup,
  not inside a reactive `Effect`.** The generated `if x.get() == "..." { }`
  is a plain (non-reactive) Rust `if`, evaluated once when the component
  function runs. Leptos may print a dev-mode warning about reading a signal
  outside a tracked reactive context; this does not affect the correctness
  of the mounted view, but it also means the branch does **not** re-run
  automatically when the signal changes. If you need console output to
  react to signal changes, drive it from the same closure that reads the
  signal in the view (e.g. inside a `{move || ...}` interpolation) rather
  than from top-level `craft<...>` statements.
- **Numbers are always `f64`.** There is no integer/float distinction in
  the type system, matching JavaScript-style numeric semantics.
- **Comparison is equality-only.** `>>` is the sole comparison operator;
  there is no `<`, `>`, `<=`, `>=`, or `!=` in conditions yet.
