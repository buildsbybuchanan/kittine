# The Kittine Language Reference

Kittine (`.kitty`) is a small language with a deliberately unusual, distinctive
syntax for state and control flow, plus an embedded JSX-like view syntax. It
compiles to idiomatic [Leptos 0.7](https://leptos.dev) Rust, which in turn
compiles to WebAssembly and runs client-side in the browser.

This document is the authoritative syntax and semantics reference. For how to
actually build and run a Kittine project, see [GETTING_STARTED.md](GETTING_STARTED.md).

## Table of contents

- [Components](#components)
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

A Kittine file is a sequence of component definitions:

```kitty
func App() {
    ...
}
```

`func Name() { ... }` declares a component named `Name`. It takes no
parameters (props are not yet supported — see [Known limitations](#known-limitations)).
Every component body may contain any number of statements, followed by
exactly one `return ( ... )` view expression as its last meaningful element.

A component compiles to a Leptos `#[component] pub fn Name() -> impl IntoView { ... }`.

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
| `<{x}> >> 0` (first occurrence) | `let (x, set_x) = signal(0);` |
| `<{x}> >> x + 1` (later occurrence) | `set_x.update(\|n\| *n += 1);` |
| `<{x}> >> x - 1` | `set_x.update(\|n\| *n -= 1);` |
| `<{x}> >> x * 2` | `set_x.update(\|n\| *n *= 2);` |
| `<{x}> >> 5` (later, non-self-referential) | `set_x.update(\|n\| *n = 5);` |
| `<{x}>` (read) | `x.get()` |

The `+= / -= / *= / /=` compound forms are only emitted when the right-hand
side is exactly `<selfname> <op> <number literal>`; any other mutation
expression lowers to the general `*n = <expr>;` form.

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

Type tags are optional. They exist for readability and for catching literal
type mistakes early, not because Kittine has (or needs) a full static type
system.

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
program      := component*
component    := "func" IDENT "(" ")" "{" stmt* return_stmt? "}"
return_stmt  := "return" "(" jsx_node ")"

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
primary      := NUMBER | STRING | BOOL | ARRAY | TYPE_TAG | IDENT
              | "<{" IDENT "}>" (">>" expr)?
              | "(" expr ")"

array        := "[" (expr ("," expr)*)? "]"
type_tag     := "<<" ("Num" | "Word" | "Flag") ">>" unary

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
#![allow(unused_braces, unused_variables)]

use leptos::prelude::*;
```

and then one `#[component] pub fn Name() -> impl IntoView { ... }` per
`func` in the source file, in source order.

## Known limitations

These are intentional scope boundaries of the current prototype, not bugs:

- **No component props.** `func Name() { ... }` takes no arguments; there is
  no syntax yet for passing data into a child component.
- **No component composition in JSX.** JSX tags are always treated as HTML
  elements, not references to other Kittine components.
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
