# The Kittine Language Reference

Kittine (`.kitty`) is a small language with a deliberately unusual, distinctive
syntax for state and control flow, plus an embedded JSX-like view syntax. It
compiles to idiomatic [Leptos 0.7](https://leptos.dev) Rust, which in turn
compiles to WebAssembly and runs client-side in the browser.

This document is the authoritative syntax and semantics reference. For how to
actually build and run a Kittine project, see [GETTING_STARTED.md](GETTING_STARTED.md).
For the complete, formal grammar (every token and production, not this
document's narrative walkthrough), see [GRAMMAR.md](GRAMMAR.md).

## Table of contents

- [Brevity by design](#brevity-by-design)
- [Components](#components)
- [Props](#props)
- [Functions (`purr`)](#functions-purr)
- [Calling functions](#calling-functions)
  - [Method calls](#method-calls)
  - [Number formatting (`.fixed` / `.padded` / `.grouped`)](#number-formatting)
  - [Date and time (`now>` / `#d` / `.formatted` / `.toDate`)](#date-and-time)
  - [Validation (`.isEmail` / `.isUrl` / `.isNumeric` / `.isAlpha` / `.isAlphanumeric` / `.minLength` / `.maxLength`)](#validation)
  - [Tuples](#tuples)
- [Modules and imports](#modules-and-imports)
  - [Visibility](#visibility)
- [Component composition](#component-composition)
  - [Children](#children)
- [Routing](#routing)
  - [Dynamic route segments](#dynamic-route-segments)
  - [Programmatic navigation](#programmatic-navigation)
- [Path-qualified expressions](#path-qualified-expressions)
- [Reference operator (`&`)](#reference-operator)
- [Variables and state (`<{ }>` / `>>`)](#variables-and-state)
  - [Plain local bindings (`hold`)](#plain-local-bindings-hold)
- [Strings](#strings)
- [Booleans (`yes>` / `no>`)](#booleans)
- [Arrays (`[ ]`)](#arrays)
- [Stashes (`stash{ }`, maps)](#stashes)
- [Type tags (`#n` / `#w` / `#f` / `#d`)](#type-tags)
- [Type inference](#type-inference)
- [Litters (`litter`, structs)](#litters)
- [Breeds (`breed`, enums)](#breeds)
- [Pattern matching (`pounce>`)](#pattern-matching)
- [Generics](#generics)
- [Claws (`claw`, `bare`, traits)](#claws)
- [Printing (`craft<...>` / `warn<...>` / `error<...>`)](#printing-craft--warn--error)
- [Control flow (`if>` / `orif>` / `else>`)](#control-flow)
- [Loops (`spin` / `}{`)](#loops)
- [Expressions and operators](#expressions-and-operators)
- [Comments](#comments)
- [The view syntax (`return ( ... )`)](#the-view-syntax)
- [Full grammar summary](#full-grammar-summary)
- [Compilation model](#compilation-model)
- [Known limitations](#known-limitations)

## Brevity by design

Every Kittine construct is a deliberately short, unusual stand-in for the
Rust it expands into — not shorthand for its own sake, but because the
generated code carries the ceremony (ownership, signal plumbing, `String`
vs. `&str`, macro ceremony) that Rust needs and a `.kitty` author shouldn't
have to type by hand. **A construct that isn't shorter than its Rust output
doesn't belong in Kittine.** A few side-by-sides, taken straight from this
repo's own test suite (`kittine-compiler/src/tests.rs`) rather than
invented for effect:

| Kittine | Generated Rust |
| --- | --- |
| `greet('World')` (where `purr greet(name) { .. }`, tag-free — see [Type inference](#type-inference)) | `greet("World".to_string())` |
| `<{count}> >> #n 0` | `let (count, set_count) = signal(0f64);` |
| `<{ready}> >> yes>` | `let (ready, set_ready) = signal(true);` |
| `if><{username}> >> 'Admin'` … `orif>` … `else>` | `if username.get() == "Admin" { .. } else if .. { .. } else { .. }` |
| `craft<'Welcome Admin'>` | `leptos::logging::log!("Welcome Admin");` |
| `spin<{n}> in [1, 2, 3] }{ craft<n> }{` | `for n in (vec![1, 2, 3]).into_iter() { leptos::logging::log!("{}", n); }` |
| `<{age}> >= 18 && <{status}> >> 'active'` | `(age.get() >= 18f64) && (status.get() == "active")` |

Two mechanical rules keep the promise real rather than aspirational:

1. **One token does the work of several Rust ones.** `>>` alone covers
   signal declare-or-mutate *and* equality comparison — context (is this
   the first `<{name}> >>` in the component, or inside a condition?)
   disambiguates, the same way `craft<...>` is always shorter than
   `leptos::logging::log!(...)` and `yes>`/`no>` are always shorter than
   `true`/`false`.
2. **Types and ownership are inferred, not spelled out.** `#w` is
   optional on a value and erased entirely from the generated Rust; a
   `.to_string()`/`.clone()` the borrow checker would otherwise force a
   human to add by hand (see the `greet` row above) is inserted by
   `kittine-compiler`, never by the `.kitty` author.

This is a standing constraint on the language, not a one-time pass: a
future addition to Kittine that produces *more* characters than writing
the Rust directly has failed its own design goal, however useful it might
otherwise be.

## Components

A Kittine file is a sequence of component and function definitions:

```kitty
func App() {
    ...
}
```

`func Name(#t prop, ..) { ... }` declares a component named `Name`.
Every component body may contain any number of statements, followed by
exactly one `return ( ... )` view expression as its last meaningful element.

A component compiles to a Leptos `#[component] pub fn Name(..) -> impl IntoView { ... }`.

## Props

```kitty
func Nav(active) {
    return ( <span>{ active }</span> )
}
```

Components can take parameters — Kittine's term for these is **props**,
matching how they're used: values passed in from a parent when the
component is composed into another view (see [Component
composition](#component-composition)). Unlike `<{name}> >> value` signals,
a prop is a plain value, not reactive state — there's no setter, and
reading it is just the bare name (`active`, not `<{active}>`).

A prop's [type tag](#type-tags) (`#n`, `#w`, `#f`) is **optional** — see
[Type inference](#type-inference). Writing one out (or an array tag,
`#n[]`/`#w[]`/`#f[]`, which inference doesn't cover) always overrides
whatever inference would have guessed. The one exception either way is
the special `children` parameter — see [Children](#children) — which
takes no type tag at all, tagged or not.

### Compilation

| Kittine | Generated Rust |
|---|---|
| `func Nav(#w active) { .. }` | `pub fn Nav(active: String) -> impl IntoView { .. }` |
| `func Card(#n price, #f onSale) { .. }` | `pub fn Card(price: f64, onSale: bool) -> impl IntoView { .. }` |
| `func NavList(#w[] items) { .. }` | `pub fn NavList(items: Vec<String>) -> impl IntoView { .. }` |

Reading a `Word` prop or any array-typed prop clones it (`items.clone()`)
rather than moving it, since a prop may be read from more than one
reactive closure inside the component body — neither `String` nor `Vec<T>`
is `Copy` the way `Num`/`Flag` are. If the *same* prop is read from more
than one place, each reactive closure additionally pre-clones it into its
own local **before** the closure itself — a `move` closure captures every
variable it uses *by value*, so two sibling closures both reading the
same original prop (`<h1>{ active }</h1><p>{ active }</p>`, say) would
otherwise fight over moving it, and the second one would fail to compile
(`E0382: use of moved value`) — found by actually compiling exactly that
pattern against real Leptos, not assumed:

```rust
{ let active = active.clone(); move || active.clone() }
```

This applies to every non-`Copy` scope-tracked read — a prop, a
view-position `spin`'s loop variable, and a [`hold`-bound
local](#plain-local-bindings-hold) — not just props.

## Functions (`purr`)

```kitty
purr double(n) {
    return (n * 2)
}
```

`purr name((#t)? param, ..) (#t)? { .. return (expr) }` declares
a plain function: it computes and returns a value, and does not render a
view. Unlike a component, its signature also has room for an explicit
**return-type** tag right after the parameter list, before the body's `{`
— optional, same as a param's tag (see [Type inference](#type-inference)).

A `purr` function is the idiomatic Kittine way to share logic (formatting,
computed values) between components without duplicating it.

### Compilation

`purr` becomes a plain, non-`#[component]` `pub fn`, with the body's
`return (expr)` becoming the function's tail expression (no `#[component]`
attribute, no `view!`):

```kitty
purr double(n) {
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

Calling a known `purr` — one defined in the same file, *or* reached
through the whole `import` graph — also gets one more piece of real type
checking: a bare string-literal argument at a `#w` parameter
position renders as an owned `String`, not a borrowed `&str`, because
`kittine-compiler build` collects every reachable file's `purr` signatures
before generating any single file's code (see [Compilation
model](#compilation-model)):

| Kittine | Generated Rust |
|---|---|
| `greet('World')` (where `purr greet(name) { .. }`) | `greet("World".to_string())` |

`greet`'s `Word` type here isn't written anywhere — it's derived by [Type
inference](#type-inference) from how `name` is used inside `greet`'s own
body, then flows into `known_functions` exactly like an explicit `#w`
tag would.

This works whether `greet` is defined in the same file or brought in via
`import` — only a call to a function Kittine has *no* signature for at
all (a real Rust/Leptos function, or a typo) renders the argument bare.

### Method calls

```kitty
use_params_map().get().get('id').unwrap_or_default()
```

`receiver.method(arg, ..)` calls a method on the result of any expression —
for interop with real Rust/Leptos APIs that aren't a Kittine `purr` (a
`leptos_router` hook, a standard-library method, anything). Chains work,
since the receiver of a method call is itself an arbitrary expression:
`a.b().c(1).d()`.

Kittine tracks no receiver or argument types here — unlike a same-file
`purr` call, where a `Num` parameter is known to be `f64`, a method call's
arguments render as plain literals (`0`, not `0f64`), since a real Rust
method just as often expects `usize`/`i32`/etc. as `f64`, and Kittine has
no way to tell which. Rust's own type checker is the source of truth on
whether the call is valid — same trust model as calling an unknown
function by name.

Calling the *result* of an expression (not a bare name) works the same
way — `expr(arg, ..)` immediately after any expression, most usefully
right after a call that returns a closure: `use_navigate()('/home')`
calls the closure `use_navigate()` itself returns. This is what makes
[programmatic navigation](#programmatic-navigation--a-real-current-gap)'s
first argument reachable at all, even though its second argument still
isn't (see that section).

### Number formatting

```kitty
revenue.fixed(2)
count.padded(4)
revenue.grouped()
```

Three method names are reserved exceptions to the "renders verbatim"
[method call](#method-calls) rule above: `fixed`, `padded`, and `grouped`
are number-formatting utilities Rust's `format!` macro syntax can express
but no real Rust *method* can — `f64` has no `.fixed()`/`.padded()`/
`.grouped()` inherent method to interop with, so unlike an ordinary method
call these three synthesize a real `format!` (or, for `.grouped()`, a small
self-contained block expression) instead of passing the call through as-is.
This is the "beyond what already-existing `MethodCall` interop reaches"
half of Phase 2's standard-library gap (see [ROADMAP.md § Phase
2](ROADMAP.md#phase-2--standard-library)) — Rust has no method-call syntax
for fixed-decimal precision, padding, or thousands grouping at all, so
there was nothing for ordinary interop to reach.

| Kittine | Generated Rust | Result (given `revenue` is `1234567.891`, `count` is `7`) |
| --- | --- | --- |
| `revenue.fixed(2)` | `format!("{:.*}", (2) as usize, (revenue.get()))` | `"1234567.89"` |
| `count.padded(4)` | `format!("{:0>1$}", (count.get()), (4) as usize)` | `"0007"` |
| `revenue.grouped()` | *(see below)* | `"1,234,567.891"` |

- **`.fixed(precision)`** — fixed decimal places, JS `.toFixed()`-style.
  Lowers to `format!`'s `{:.*}` dynamic-precision specifier, which takes the
  precision as a *runtime* positional argument instead of a compile-time
  literal (`{:.2}` only accepts a literal) — the reason `precision` can be
  any expression, not just a number literal: `revenue.fixed(decimals)`
  works exactly the same way with `decimals` a signal.
- **`.padded(width)`** — zero-left-pads a value's `Display` text to `width`
  characters (clock digits, order numbers, anything that needs a stable
  column width). Lowers to `format!`'s `{:0>1$}` fill/align/dynamic-width
  specifier — same "width can be any expression, not just a literal"
  reasoning as `.fixed`'s precision.
- **`.grouped()`** — thousands-separator formatting (`1234567` →
  `"1,234,567"`). Rust's `format!` has no grouping specifier at all (unlike
  precision/width, there's no macro syntax to reach for), so this is the
  one case that lowers to a small self-contained Rust block expression
  instead of a single `format!` call — still just one expression, valid
  everywhere a method call's result is (a `craft<...>` argument, a JSX `{
  expr }` interpolation, a call argument), it just spans more than one line
  of generated source. The block splits the value's `Display` text on an
  optional leading `-` and an optional `.`, groups the integer part into
  comma-separated 3-digit chunks from the right, and leaves the sign/
  decimal part untouched — `(-1234.5).grouped()` → `"-1,234.5"`.

All three take no receiver-type information either, same trust model as an
ordinary method call: nothing stops writing `someWord.fixed(2)` and having
it fail only once Rust's own type checker sees the generated
`format!("{:.*}", .., someWord.get())` and rejects a `String` where a
`Display`-of-a-number was implicitly expected by the *author's* intent
(though `String` does implement `Display`, so this specific misuse would
actually still compile — it just wouldn't do anything meaningful with
`.fixed`'s intent of controlling decimal places on a non-numeric value).

### Date and time

```kitty
<{joined}> >> #d now>
craft<joined.formatted("%Y-%m-%d")>
<{launchDay}> >> '2026-01-01 09:00:00'.toDate("%Y-%m-%d %H:%M:%S")
craft<launchDay.formatted("%B %d, %Y")>
```

`Date` is Kittine's fourth scalar type (alongside `Num`/`Word`/`Flag`),
closing the "date/time formatting" half of Phase 2's standard-library gap
(see [ROADMAP.md § Phase 2](ROADMAP.md#phase-2--standard-library)) —
deliberately left open in the `.fixed`/`.padded`/`.grouped` round above
since it needed a real type, not just more number formatting. It lowers to
`chrono::DateTime<chrono::Utc>`, and a project with a `Date` value needs
`chrono` as a real Cargo dependency (`features = ["wasmbind"]` on a
CSR/WASM target — see this section's own Compilation note below), the same
way a `litter`/`breed` needs `serde` for JSON.

- **`now>`** is the *only* way to produce a `Date` value from scratch —
  same `>`-suffixed-keyword shape as [`yes>`/`no>`](#booleans). Lowers to
  `chrono::Utc::now()`. There's no fixed-calendar-date literal (no
  `now>`-like spelling for "2026-01-01") — construct one via
  [`.toDate(pattern)`](#toDate) instead.
- **`#d`** is the type tag for `Date`, the same two-character-sigil
  convention `#n`/`#w`/`#f` already use — see [Type
  tags](#type-tags). `#d[]`/`#d{}` (an array or [`stash`](#stashes) of
  `Date`) work the same way their scalar siblings do.
- **`.formatted(pattern)`** — a `chrono` strftime-pattern format of a
  `Date` value, e.g. `moment.formatted("%Y-%m-%d")` → `"2026-08-04"`. A
  reserved method name, the `Date`-typed sibling of
  [`.fixed`/`.padded`/`.grouped`](#number-formatting): a real `chrono`
  method exists (`DateTime::format`) but under a shape (`&str` in,
  a lazy `DelayedFormat` out) a `.kitty` author calling it as an ordinary
  [method call](#method-calls) wouldn't get for free — this reserved name
  gives it the same "no dedicated syntax needed" ergonomics. Lowers to
  `receiver.format(&(pattern)).to_string()`.
- **`.toDate(pattern)`** — parses a `Word` into a `Date`, the reverse of
  `.formatted`, e.g. `"2026-08-04 12:00:00".toDate("%Y-%m-%d %H:%M:%S")`.
  Lowers to `chrono::NaiveDateTime::parse_from_str(&(receiver),
  &(pattern)).unwrap().and_utc()` — needs a *full* date+time pattern (a
  date-only pattern like `"%Y-%m-%d"` alone doesn't parse via this path),
  and `.unwrap()` panics on a bad parse rather than returning a `Result`
  — see [Known limitations](#known-limitations).

| Kittine | Generated Rust |
| --- | --- |
| `now>` | `chrono::Utc::now()` |
| `moment.formatted("%Y-%m-%d")` | `(moment.get()).format(&("%Y-%m-%d")).to_string()` |
| `raw.toDate("%Y-%m-%d %H:%M:%S")` | `chrono::NaiveDateTime::parse_from_str(&(raw.get()), &("%Y-%m-%d %H:%M:%S")).unwrap().and_utc()` |

#### Compilation

`Date`/`#d` follows the same `is_non_copy_param_type` reasoning `Num`/
`Flag` already get, not `Word`'s: `chrono::DateTime<Utc>` is `Copy` (its
`Offset` type, `Utc`, is a zero-sized `Copy` unit struct), so a `Date`
param/prop is passed by value like a number or boolean, never pre-cloned
like a `Word`/`litter`/`breed`.

### Validation

```kitty
email.isEmail()
website.isUrl()
zip.isNumeric()
username.isAlpha()
code.isAlphanumeric()
username.minLength(3) && username.maxLength(20)
```

Seven more reserved method names, closing the "validation" half of Phase
2's standard-library gap (see [ROADMAP.md § Phase
2](ROADMAP.md#phase-2--standard-library)) left open by the [number
formatting](#number-formatting) and [date/time](#date-and-time) rounds
above. None of `isEmail`/`isUrl`/`isNumeric`/`isAlpha`/`isAlphanumeric`/
`minLength`/`maxLength` is a real inherent method on `String`/`&str`, so,
same trade-off as `.fixed`/`.padded`/`.grouped`, these synthesize a real
boolean-valued Rust expression instead of passing the call through
verbatim.

| Kittine | Generated Rust (shape) | Result (given `zip` is `"90210"`) |
| --- | --- | --- |
| `zip.isNumeric()` | `(zip.get()).trim().parse::<f64>().is_ok()` | `true` |
| `name.minLength(3)` | `((name.get()).chars().count() as f64) >= ((3) as f64)` | — |

- **`.isEmail()`** — a real-enough (not RFC-5322-complete) shape check:
  exactly one `@`, a non-empty local part, and a domain part that
  contains a `.` without starting or ending on one, with no whitespace
  anywhere in the value. No MX-record lookup, no full RFC 5322 grammar —
  the same trust level as HTML5's own `<input type="email">` pattern.
- **`.isUrl()`** — a real-enough `http(s)://host...` shape check: a
  recognized scheme, a non-empty host containing a `.`, and no
  whitespace. No non-`http(s)` scheme support, no percent-encoding
  validation.
- **`.isNumeric()`** — whether the whole (trimmed) value parses as a real
  `f64`. Unlike `.isEmail`/`.isUrl`, this one has a real underlying Rust
  method to lean on (`str::parse`), the same "an existing method under a
  shape a `.kitty` author wouldn't guess" reasoning `.formatted`/`.toDate`
  already use for `chrono`.
- **`.isAlpha()` / `.isAlphanumeric()`** — every character
  alphabetic/alphanumeric (Unicode-aware, via `char::is_alphabetic`/
  `char::is_alphanumeric`) *and* the value non-empty — a bare
  `.chars().all(..)` on an empty string is vacuously `true`, which isn't
  the useful validation-utility answer, so both add an explicit
  non-empty check.
- **`.minLength(n)` / `.maxLength(n)`** — compares `.chars().count()`
  (not `.len()`, which counts UTF-8 bytes, not user-perceived characters)
  against `n`. `n` can be any expression (a signal read), not just a
  literal, cast to `f64` for the comparison since Kittine's `Num` is
  always `f64` — same "argument can be any expression" reasoning
  `.fixed`'s precision argument already has.

`.isEmail()`, `.isUrl()`, `.isAlpha()`, and `.isAlphanumeric()` each lower
to a small self-contained block expression (the receiver bound to a named
local before `.as_ref()` borrows from it — a `.get()` call's result is an
unnamed temporary, and a reference taken from it needs the temporary
itself named first to outlive the block's later statements, `E0716`
otherwise). `.isNumeric()`, `.minLength()`, and `.maxLength()` are single
expressions with no such binding needed.

### Tuples

```kitty
(StaticSegment('user'), ParamSegment('id'))
```

`(expr, expr, ..)` is a tuple literal — needed to combine multiple
`leptos_router` path segments into one dynamic route (see
[Routing](#routing)). A single parenthesized expression with no comma is
still just grouping, not a 1-tuple: `(age + 1)` is `age + 1`, not a
1-tuple containing it.

## Modules and imports

```kitty
import { Nav, Footer } from './components/Nav.kitty'
```

`import { Name, Name2 } from 'path/to/file.kitty'` brings one or more
components/functions from another `.kitty` file into scope. A path
containing `/` or ending in `.kitty` is resolved relative to the importing
file, same as always. Imports must appear before any `func`/`purr`
declarations in the file.

### Package imports

```kitty
import { shout } from 'kittine-strings'
```

A bare name — no `/`, no `.kitty` extension — is a *package* import
instead: it resolves to `kitten_modules/<name>/lib.kitty`, searched
upward from the importing file's own directory (the same upward-search
`node_modules` resolution uses, so it works the same from any file in the
project). `kitten_modules/` is populated by `kittine-compiler install`,
which resolves and downloads whatever's listed in `kittine.toml` from the
[package registry](CLI.md#the-package-registry) — see
[CLI.md](CLI.md) for the full `add`/`install`/`publish` reference.

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
`.rs` path. An import cycle is a compile error, not an infinite loop. A
package import's `#[path]` points at `kitten_modules/<name>/lib.rs`
instead, the sibling of the `lib.kitty` it resolved to.

### Visibility

```kitty
private purr internalHelper(#n n) #n {
    return (n * 2)
}
```

Every top-level `func`/`purr` is importable from any other file by
default. `private` before `func`/`purr` opts a specific one out — trying
to `import` a `private` item is then a compile error.

#### Compilation

`private` becomes a plain (non-`pub`) Rust item, instead of `pub`:

```rust
fn internalHelper(n: f64) -> f64 {
    n * 2f64
}
```

Kittine doesn't check `import`s against `private` itself — a plain Rust
item is only visible within its own module, so trying to `use` a `private`
one from another file's generated `mod` is *already* a Rust compile error
(`E0603: function \`..\` is private`) once the two files are compiled
together. One correctness guarantee, enforced for free by the compiler
Kittine already targets, instead of a second one Kittine would have to
reimplement and keep in sync.

### Re-exports

```kitty
export import { Nav } from './Nav.kitty'
```

An ordinary `import` only brings a name into scope for use *within* that
file — a third file can't reach it by importing from the file that just
imported it. `export` before `import` re-exposes the imported names under
this file's own name, so a third file's `import { Nav } from
'./this-file.kitty'` works without reaching all the way back to wherever
`Nav` is actually defined. Useful for a "barrel" file that re-exports
several components from one convenient path — `example-app`'s
`components.kitty` does exactly this for `Nav`/`Card`/`NavList`.

#### Compilation

`export import` becomes `pub use` instead of a plain `use`:

```rust
#[path = "./Nav.rs"]
mod __kittine_mod_nav;
pub use __kittine_mod_nav::{Nav};
```

The `mod` declaration itself stays a plain (non-`pub`) Rust item either
way — a `pub use` re-export doesn't need its *source* module to be `pub`
too, only the item it's re-exporting (which is already `pub`, or the
import wouldn't have compiled in the first place — see
[Visibility](#visibility)).

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

### Children

```kitty
func Card(#w title, children) {
    return (
        <div class='card'>
            <h3>{ title }</h3>
            { children() }
        </div>
    )
}

func Page() {
    return (
        <Card title='About'>
            <p>"Nested JSX content, passed through from the caller."</p>
        </Card>
    )
}
```

A parameter named exactly `children` — written with **no** type tag, unlike
every other prop — declares that a component accepts nested JSX content
from wherever it's composed. Call it with `children()` (an ordinary
[function call](#calling-functions)) inside the view to render whatever the
caller nested between the opening and closing tag. There is nothing to
write at the call site beyond nesting content normally
(`<Card ..>...</Card>`) — no `children={...}` attribute needed.

`children()` renders once (it isn't reactive) — write `{ children() }`, not
wrapped in anything, exactly as shown above.

#### Compilation

`children` becomes a `Children` parameter (from `leptos::prelude::*`,
already in scope); `{ children() }` becomes a bare `{children()}` — every
*other* `{ expr }` interpolation gets wrapped in a `move || ..` reactive
closure, but `Children` is call-once (`FnOnce`), so Kittine recognizes this
one specific shape and renders it bare instead:

```rust
pub fn Card(title: String, children: Children) -> impl IntoView {
    view! {
        <div class="card">
            <h3>{{ let title = title.clone(); move || title.clone() }}</h3>
            {children()}
        </div>
    }
}
```

Composition with nested content needs no codegen changes at all beyond
what [component composition](#component-composition) already does — the
`view!` macro wires nested JSX into the `children` prop automatically, the
same way it would for hand-written Leptos.

## Routing

Kittine has **no routing syntax of its own** — and doesn't need one.
[`leptos_router`](https://docs.rs/leptos_router) is in scope in every
generated file, and its components (`Router`, `Routes`, `Route`, `A`, ...)
are just ordinary components: PascalCase tags, composed exactly like any
`func`-defined component. A route's path segment
(`StaticSegment`/`ParamSegment`/`WildcardSegment`/`OptionalParamSegment`)
is a plain function call, which Kittine already supports. This is
deliberate: Kittine adds a language on top of Rust/Leptos, not a
parallel ecosystem underneath it — routing, like most of what a real
website needs, should come from the framework it already compiles to.

```kitty
import { Home } from './Home.kitty'
import { About } from './About.kitty'
import { NotFound } from './NotFound.kitty'

func App() {
    return (
        <Router>
            <nav>
                <A href='/'>"Home"</A>
                <A href='/about'>"About"</A>
            </nav>
            <main>
                <Routes fallback={NotFound}>
                    <Route path={StaticSegment('')} view={Home}/>
                    <Route path={StaticSegment('about')} view={About}/>
                </Routes>
            </main>
        </Router>
    )
}
```

A few things worth calling out about this example, since none of them are
Kittine-specific behavior — they're just how Leptos's `view!` macro already
treats these components:

- `view={Home}` and `fallback={NotFound}` are **bare component
  references**, not calls (`Home`, not `Home()`) — write them with braces
  since [JSX attribute values](#component-composition) need either a
  string literal or a `{ expr }`, never a bare unbraced identifier.
  `Home` isn't a signal or a prop, so it renders as a plain identifier,
  exactly the zero-argument view-returning function `Route`'s `view` prop
  and `Routes`'s `fallback` prop expect.
- `path={StaticSegment('')}` is an ordinary [function call](#calling-functions)
  — `StaticSegment` is a plain tuple constructor from `leptos_router`, not
  a macro. (`leptos_router`'s own docs often show `path!("...")`, a macro
  form Kittine doesn't support — use `StaticSegment("...")` /
  `ParamSegment("id")` instead, which are equivalent and already work.)
- `<A href='/about'>"About"</A>` renders a real `<a>` element that
  intercepts the click for client-side navigation — no full page reload.

### Compilation

Nothing routing-specific happens in codegen — the example above lowers via
the same rules as any other composition:

```rust
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::*;
use leptos_router::hooks::*;
// .. + one `mod`/`use` pair per import

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav>
                <A href="/".to_string()>"Home"</A>
                <A href="/about".to_string()>"About"</A>
            </nav>
            <main>
                <Routes fallback=NotFound>
                    <Route path=StaticSegment("") view=Home/>
                    <Route path=StaticSegment("about") view=About/>
                </Routes>
            </main>
        </Router>
    }
}
```

All three `leptos_router` `use` lines are emitted in **every** generated
file (see [Compilation model](#compilation-model)) — routing works the
moment you reach for it, without a separate opt-in step, at the cost of
an `unused_imports` allowance for files that don't.

### Dynamic route segments

A dynamic segment combines a static prefix and a `ParamSegment` in a
[tuple](#tuples), and reads back out via a [method-call
chain](#method-calls) on `use_params_map()` — verified end-to-end (real
compile, real dev server, real click-through) in `example-app`'s
`User.kitty`:

```kitty
<Route path={(StaticSegment('user'), ParamSegment('id'))} view={User}/>
```

```kitty
func User() {
    return (
        <p>"User id: "{ use_params_map().get().get('id').unwrap_or_default() }</p>
    )
}
```

`leptos_router::hooks` (`use_params_map`, `use_navigate`, `use_query_map`,
...) isn't re-exported at `leptos_router`'s crate root the way
`components` is, so it gets its own `use leptos_router::hooks::*;` in
every generated file's fixed preamble (see [Compilation
model](#compilation-model)) — the same "already in scope, zero opt-in
step" treatment as everything else routing-related.

### Programmatic navigation

`leptos_router::hooks::use_navigate()` returns a closure you call with
`(&str, NavigateOptions)` — verified end-to-end (real compile, real dev
server, real click-through, checking browser console output for a panic,
not just that the page loaded) in `example-app`'s `User.kitty`:

```kitty
func User() {
    hold navigate >> use_navigate()

    return (
        <button onClick={navigate('/', NavigateOptions::default())}>
            "Go home"
        </button>
    )
}
```

**`use_navigate()` must be called eagerly, at component setup — not
lazily inside the event handler.** The first version of this example
called it directly in `onClick={use_navigate()('/', ..)}`, which compiled
fine but **panicked at runtime**: `You cannot call use_navigate outside a
<Router>`. Leptos's context-dependent hooks (`use_navigate`,
`use_context`, and others) resolve their context from the *reactive
owner active at the moment the hook function runs* — during a
component's synchronous setup, that's correctly the `<Router>`'s; by the
time a click fires later from the browser's event loop, it isn't
anymore, even though the button is still physically inside `<Router>` in
the DOM. This was only caught by checking the actual browser console for
a panic after clicking — the page still "looked" fine (no crash overlay,
no URL change either, easy to miss) until the console was checked.

[`hold navigate >> use_navigate()`](#plain-local-bindings-hold) forces
the eager call at the right time: `use_navigate()` runs once, during
component setup, and `navigate` refers to whatever it returned from then
on. Calling `navigate('/', ..)` later — a bare call to a `hold`-bound
name — works from as many event handlers as needed; each one
independently pre-clones `navigate` before its own closure (see
[Method calls](#method-calls) and [Compilation
model](#compilation-model) for why that pre-clone matters whenever the
same non-`Copy` value is read from more than one closure).

Any bare expression works as a JSX event-handler value, not just a
`<{name}> >> value` mutation — `onClick={expr}` renders as `move |_|
expr`.

Everything else `leptos_router` supports — nested routes (`<ParentRoute>`
+ `<Outlet/>`), wildcard/catch-all segments — is available the same
zero-new-syntax way.

Everything on this page describes `example-app`'s **client-side rendered**
(CSR) setup, via Vite. Server-side rendering (real HTML in the first
response, for SEO/fast first paint) is also real, via a separate
toolchain (`cargo-leptos` + Axum, not Vite) — `kittine-compiler` needs no
changes for it. See [docs/SSR.md](SSR.md) for the full setup and
`example-ssr/` for a working multi-page example.

## Path-qualified expressions

```kitty
NavigateOptions::default()
std::cmp::max(1, 2)
```

`Segment::Segment(::Segment)*` — for a real Rust associated function,
static method, or constant Kittine has no dedicated syntax for
(`Type::method()`, `Type::CONST`, a multi-segment path like
`std::cmp::max`). Combines with the existing [method-call chain](#method-calls)
and [calling the result of an expression](#calling-the-result-of-an-expression)
machinery for free — a path is just another expression, so
`NavigateOptions::default()` is a path (`NavigateOptions::default`)
immediately called with no arguments, the same shape `use_navigate()`
already parses as for a bare name.

Kittine tracks no meaning for any segment — it renders the path verbatim
(`segments.join("::")`) and lets Rust's own name resolution and type
checker validate it, same trust model as an unknown function call or a
method call.

## Reference operator

```kitty
serde_json::to_string(&origin).unwrap_or_default()
```

`&expr` renders as a real Rust reference (`&<expr>`) — needed to call into
any real Rust/crate API that takes one, which is common (`serde_json::
to_string(&value)`, and much of the wider Rust ecosystem beyond it).
Every Kittine value is otherwise always owned; this exists purely as an
interop escape hatch, same trust model as [path-qualified
expressions](#path-qualified-expressions) and [method
calls](#method-calls) — Rust's own type checker validates the result, not
Kittine. `&` binds like unary `-`
(`parse_unary`), and referencing a non-`Copy` scope-tracked value (a
`Word`/array/`stash`/`litter` prop, `spin` item, or `hold` binding) still
gets the usual pre-clone treatment first (`&p.clone()`, not a dangling
reference to something already moved elsewhere), same as reading that
value any other way.

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
| `<{label}> >> 'reset'` (later, `Word` signal) | `set_label.update(\|n\| *n = "reset".to_string());` |
| `<{x}>` (read) | `x.get()` |

The `+= / -= / *= / /=` compound forms are only emitted when the right-hand
side is exactly `<selfname> <op> <number literal>`; any other mutation
expression lowers to the general `*n = <expr>;` form. A bare string
literal on the right-hand side of a mutation gets the same
`.to_string()` treatment a signal's first/declaring occurrence already
had — `*n = "reset"` doesn't type-check when `*n: &mut String` (a `Word`
signal), since a literal alone is `&'static str`. Concatenation
(`<{label}> >> 'x' + <{label}>`) was never affected by this, since a `+`
involving a string literal already always lowers to an owned
`format!(..)` regardless of position.

A whole-number literal always gets an explicit `f64` suffix wherever it's
an operand next to an already-concretely-typed `f64` value (a signal's
initializer, an arithmetic operand, a compound-assignment right-hand side).
This isn't cosmetic: `signal(0)` alone leaves `0`'s type to Rust's generic
inference, which is free to pick something other than `f64` if nothing else
in the function pins it down first — and it fails to compile the moment
that value is later passed somewhere that concretely requires `f64` (a
`purr` call, for instance). Spelling it `0f64` up front avoids the
ambiguity entirely.

### Plain local bindings (`hold`)

```kitty
hold navigate >> use_navigate()
```

`hold name >> expr` is a **plain, non-reactive** local binding — unlike
`<{name}> >> value`, it never declares a signal. `expr` is evaluated
exactly once, at that point in the component, and `name` refers to
whatever it returned from then on. There's no mutation form (unlike
`<{ }>`'s reuse for both declare and mutate) — writing `hold name >>` a
second time is just an ordinary Rust `let` shadowing the first.

This exists specifically for calling a Leptos hook that depends on
reactive context (`use_navigate()`, `use_context()`, ...) — see
[Programmatic navigation](#programmatic-navigation) for why calling one
*lazily*, inside an event handler, panics at runtime, and why `hold`
forcing an eager call at component setup is the fix. Reading a
`hold`-bound name — bare (`navigate`), calling it (`navigate(args)`), or
via [method calls](#method-calls)/[calling the result of an
expression](#calling-the-result-of-an-expression) — always renders with
the same `.clone()` treatment a `Word`/array prop gets, since a held
value might not be `Copy` (a closure, for instance) and may need to be
read from more than one reactive closure.

#### Compilation

`hold name >> expr` becomes a bare `let name = expr;`:

```rust
let navigate = use_navigate();
```

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

## Stashes

```kitty
<{prices}> >> #n{} stash{ milk: 2, eggs: 3, bread: 4 }
craft<prices.get('milk').cloned().unwrap_or_default()>
```

A `stash{ key: expr, key: expr, .. }` literal is Kittine's collection type
"beyond arrays" — a `String`-keyed map. It lowers to
`std::collections::HashMap::from([("key".to_string(), expr), ..])`. Keys
are written like struct-literal field names (a plain identifier, not a
quoted string) — the same `Name { field: expr, .. }` grammar `litter`
construction already uses, just with the reserved name `stash` instead of
a real `litter` name (so `stash{ .. }` gets the exact same parsing,
`kittine-compiler fmt` round-tripping, and duplicate-key-shaped lint
checks a real struct literal already has, for free).

A `stash`-typed prop, `purr` param, or return type needs an explicit tag —
`#n{}`/`#w{}`/`#f{}` (a `Num`/`Word`/`Flag`-valued, always `String`-keyed
map) — mirroring an array's own `#n[]`/`#w[]`/`#f[]`: type inference
doesn't reach into a `stash`'s value type any more than it reaches into an
array's element type (see [Type inference](#type-inference)).

```kitty
purr buildPrices() #n{} {
    return (stash{ milk: 2, eggs: 3 })
}
```

**Reading and mutating a `stash`** goes through the same two mechanisms
every other Kittine value does — there's no dedicated `stash`-only syntax
for either:

- **Reading a value** is a plain method-call chain on the map, exactly
  like calling any other Rust API Kittine doesn't have first-class syntax
  for (see [Method calls](#method-calls)): `prices.get('milk')` returns
  Rust's own `Option<&f64>`, chained further with ordinary `HashMap`/
  `Option` methods — `.cloned().unwrap_or_default()` is the idiomatic
  "give me an owned value, or a zero/empty default if the key's missing"
  pattern. Avoid `.unwrap_or(<some literal>)` with a bare whole number for
  a `Num{}`-valued `stash`: a method-call argument gets no forced-`f64`
  coercion the way an arithmetic operand does (see [Method
  calls](#method-calls)), so `unwrap_or(0)` fails to compile
  (`expected f64, found integer`) — `unwrap_or_default()` (no argument at
  all) sidesteps this entirely, and is usually what "no value ⇒ zero" was
  asking for anyway.
- **Mutating** replaces the whole map at once, the same declare-or-replace
  signal semantics every other Kittine value already has: `<{prices}> >>
  stash{ .. }` (or `>> stashVariable`) is the only way to change a
  `stash` signal's *tracked* value. Calling a mutating `HashMap` method
  like `.insert(..)` on a `.get()`-read value works as plain Rust, but
  (like reading any signal into a local) doesn't write back through the
  signal and won't be seen by anything reactive.

**What's intentionally out of scope for now** (not bugs, just not built
yet): a `stash` value type can only be one of the three scalars
(`Num`/`Word`/`Flag`) — no map-of-array or map-of-`litter`, matching
arrays' own scalar-only element type. Keys are always `String` (no
`Num`-keyed map). There's no `spin` support for iterating a `stash`'s
entries in a view (`spin` assumes a plain `IntoIterator<Item = T>`;
`HashMap`'s own iterator yields `(&K, &V)` pairs, and Kittine has no
tuple-index (`.0`/`.1`) field-access syntax to destructure that inside a
`spin` body yet) — read individual keys with `.get(..)` instead.

## Type tags

```kitty
<{count}> >> #n 0
<{label}> >> #w 'hi'
<{ready}> >> #f yes>
<{moment}> >> #d now>
```

`#t value` is an explicit type tag — the idiomatic Kittine way to annotate
a value's type. It's a two-character sigil (one for the `#`, one for the
type's initial) with **no closing delimiter** — shorter than every Rust
type it can stand for (`f64`, `String`, `bool`, `chrono::DateTime<Utc>`),
by design (see [Brevity by design](#brevity-by-design)). There are four
scalar sigils, plus an array form for each:

| Tag | Matches |
|---|---|
| `#n` | number literals |
| `#w` | string literals |
| `#f` | boolean literals |
| `#d` | [the `now>` literal](#date-and-time) |
| `#n[]` / `#w[]` / `#f[]` / `#d[]` | array literals of the matching element type |
| `#n{}` / `#w{}` / `#f{}` / `#d{}` | [`stash`](#stashes) literals of the matching value type |

```kitty
<{scores}> >> #n[] [10, 20, 30]
<{prices}> >> #n{} stash{ milk: 2 }
```

When the tagged value is a literal, the compiler checks it against the tag
at compile time and rejects a mismatch (`#n 'oops'` is a parse error; for
an array tag, every literal *element* is checked too — `#n[] ['a', 'b']`
is also a parse error). A `stash` tag is the one exception, checked no
more strictly than a variable read is — see [Stashes](#stashes). When the
tagged value is a variable read or a computed expression (its static type
isn't known at parse time), the annotation is trusted rather than
checked. Either way, the tag itself is erased during code generation —
Rust's own type inference already gives the underlying value the right
type, so `#n 0` and a bare `0` generate identical Rust.

Type tags are optional on a value (`<{count}> >> 0` and `<{count}> >> #n
0` compile identically) — they exist for readability and for catching
literal type mistakes early, not because Kittine has (or needs) a full
static type system. They're also optional on a prop or `purr`
param/return type now — see [Type inference](#type-inference) — with one
exception: an array tag (`#n[]`/`#w[]`/`#f[]`) in a signature position is
still required, since inference doesn't reach into array element types.

## Type inference

```kitty
purr greet(name) {
    return ('Hello, ' + name)
}
```
```rust
pub fn greet(name: String) -> String {
    format!("Hello, {name}")
}
```

A [prop](#props)'s or [`purr`](#functions-purr) scalar param/return type
tag can be omitted entirely — no `#n`/`#w`/`#f`, no placeholder, just the
bare name. `kittine-compiler` fills the concrete type in by looking at how
the name is actually used inside that function or component's own body,
the same way a human would read the code to figure out what `name` in
`greet` above has to be: it's concatenated with a string literal (`'Hello,
' + name`), so it's a `Word`. This is real inference — the tag isn't
merely hidden, it's *derived*, and everything downstream (the generated
Rust signature, the `Word`-parameter string-literal `.to_string()`
coercion at call sites — see [Calling functions](#calling-functions)) sees
exactly the type a hand-written tag would have produced. Regenerating
`greet('World')` this way and with the old `purr greet(#w name) #w { .. }`
form produces byte-identical Rust output — inference is purely front-end
sugar, same as the type tags it's now optional in front of.

The rules, applied to how a name is used anywhere in its own function's
body or return expression (a component prop also looks at the `return (
... )` view):

| Usage | Inferred type |
|---|---|
| `+` against a number literal, or `-`/`*`/`/`/`<`/`<=`/`>`/`>=` at all | `Num` |
| `+` against a string literal (concatenation) | `Word` |
| `==`/`!=` against a number literal | `Num` |
| `==`/`!=` against a string literal | `Word` |
| `==`/`!=` against `yes>`/`no>` | `Flag` |
| either side of `&&`/`\|\|` | `Flag` |
| none of the above (e.g. passed straight through with no operator touching it) | `Word` (default) |

`-`/`*`/`/` and the four ordering comparisons infer `Num` unconditionally
(whatever the *other* operand looks like) — `+` is the one operator
genuinely ambiguous between arithmetic and string concatenation, so its
inference looks at whether the *other* side is a number or string literal
before deciding.

An explicit tag always wins outright — inference only ever fills a gap
left by omitting one, never overrides a tag actually written. Two real
scoping limits, both intentional rather than bugs:

- **Local to one function/component.** Inference never looks at *other*
  functions, same file or not — passing an untyped param straight into a
  same-file `purr` whose own param is `Num`-typed doesn't propagate that
  type backward. Give the param an explicit tag if the default guess
  (`Word`) is wrong for a passthrough case like that.
- **Scalars only.** An array-typed prop/param/return
  (`#n[]`/`#w[]`/`#f[]`) still needs its tag written out in full —
  inference doesn't look inside a `spin` loop body to guess an array's
  element type.

## Litters

```kitty
litter Point {
    x #n,
    y #n
}
```

`litter Name { field type, .. }` declares a plain data record — Kittine's
term for what Rust calls a struct (a "litter" of related fields, matching
the cat theme every other keyword follows: `purr`, `craft`, `spin`,
`hold`). Each field is a `name type` pair, comma-separated, using the same
[type tags](#type-tags) as a prop or `purr` param — `#n`/`#w`/`#f`, an
array form, [`#t`](#generics) for a generic litter's own type parameter,
or another `litter`/`breed`'s name for a nested custom type. `private
litter ..` opts it out of being importable, same as [`private
func`/`purr`](#visibility).

A litter value is constructed with a struct literal — `Name { field:
expr, .. }` — and a field is read with a bare `.field` (no parens, unlike
a [method call](#method-calls)):

```kitty
hold p >> Point { x: 3, y: 4 }
craft<p.x>
```

Every `litter` (and [`breed`](#breeds)) also derives `serde::Serialize`/
`serde::Deserialize`, so a value round-trips through JSON — or any other
serde-backed format (YAML, CSV, ...) — via a plain [path-qualified
call](#path-qualified-expressions), no dedicated Kittine syntax needed:

```kitty
craft<serde_json::to_string(&p).unwrap_or_default()>
```

This is unconditional (the same as the `Clone, Debug` derive already is)
— a project with even one `litter`/`breed` now needs `serde` (with the
`derive` feature enabled) as a real Cargo dependency, whether or not it
actually serializes anything.

### A `litter`/`breed` name as a prop or `purr` param/return type

A `litter` or `breed`'s own name — optionally `[]`-suffixed for an array
of it — is a real `func`/`purr` param or return type too, not just a
[field's type](#litters): the same bare-name convention a litter field
already uses, just in the type-first position a scalar tag (`#n`/`#w`/
`#f`) takes:

```kitty
litter DocEntry {
    title #w,
    category #w
}

purr firstCategory(DocEntry[] entries) #w {
    return (entries.first().map(|e| e.category.clone()).unwrap_or_default())
}

func DocCard(DocEntry entry) {
    return ( <p>{ entry.title }</p> )
}
```

`DocEntry[] entries` mirrors `#n[]`/`#w[]`/`#f[]`'s array convention —
`entries` is a real `Vec<DocEntry>` once generated (see
[Compilation](#compilation) below) — and `DocEntry entry` is the scalar
(non-array) form, needed to accept or return a single struct value. A
param position tells the two apart from an ordinary untyped param by
lookahead: `DocEntry entry` (a capitalized identifier followed by another
identifier) names a type, while a lone `entries` with nothing following it
is just an untyped param name, [inferred](#type-inference) as usual.
Unlike a scalar tag, there's no inference fallback for a custom type here
— it always has to be written out.

### Compilation

`litter` becomes a plain Rust `struct`, `#[derive(Clone, Debug,
serde::Serialize, serde::Deserialize)]` so a litter-typed value can be
read from more than one reactive closure the same way a `Word` prop
already can — a litter is never `Copy` — and round-trips through JSON (or
another serde format) with no extra ceremony. A struct literal becomes
the same shape verbatim, with each field value getting the same coercion
its declared type would give a `purr` argument (a `Word` field's bare
string literal becomes an owned `String`, a `Num` field's bare number
becomes an unambiguous `f64`):

```kitty
litter Point {
    x #n,
    y #n
}
```
```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
```
```kitty
Point { x: 3, y: 4 }
```
```rust
Point { x: 3f64, y: 4f64 }
```

An array-of-`litter`/`breed` param/return (`DocEntry[]`) becomes a real
`Vec<T>` the exact same way a scalar array does (`#n[]` -> `Vec<f64>`),
just with the element type spelled out instead of one of the three
scalars:

```kitty
purr passthrough(DocEntry[] entries) DocEntry[] {
    return (entries)
}
```
```rust
pub fn passthrough(entries: Vec<DocEntry>) -> Vec<DocEntry> {
    entries
}
```

## Closures

```kitty
entries.iter().filter(|e| e.category >> query).cloned().collect()
```

`|param, ..| expr` is a closure literal — it lowers verbatim to a Rust
closure (`|param, ..| expr`), the missing piece that makes a real Rust
iterator method (`.filter()`, `.map()`, ...) usable as a [method
call](#method-calls) with an actual predicate/transform, not just a bare
argument. The zero-param form (`|| expr`) works too, sharing the lexer's
`||` token with the logical-or operator — never actually ambiguous, since
`||` only reaches a closure literal in a *primary* (prefix) position,
never the infix position the real `||` operator is parsed in.

A param is untyped — Rust infers it from how the closure is actually used
(e.g. `Vec<T>::filter`'s `&T`) — and the body is a single expression, the
same "one expression, no block" shape every other Kittine construct that
computes rather than acts already has (a `purr`'s `return (expr)`, a
[`pounce>` arm](#pattern-matching)).

### Compilation

```kitty
|e| e.title.contains(&query)
```
```rust
|e| e.title.contains(&query.clone())
```

A closure param reads bare (never `.clone()`d/`.get()`d) inside its own
body — it's a fresh value handed to that one invocation, not a name
captured from an enclosing reactive closure that might run more than
once — while a name from the *enclosing* scope (`query` above) still gets
whatever treatment its own binding kind already calls for.

## Breeds

```kitty
breed Shape {
    Circle(#n),
    Square(#n),
    Idle
}
```

`breed Name { Variant (type)?, .. }` declares a closed set of named
variants — Kittine's term for what Rust calls an enum (a "breed" is one
of several kinds of cat, matching a variant being one of several kinds of
value). A variant carries **at most one** payload value — `Circle(#n)` —
or none at all — `Idle`. `private breed ..` works the same as `private
litter`/`func`/`purr`.

A payload-carrying variant is constructed exactly like a function call —
`Circle(5)` — and a unit variant is referenced bare — `Idle`. Both are
told apart from an ordinary `purr` call or variable read by
`kittine-compiler` already knowing every reachable `breed`'s variants
(the same whole-`import`-graph signature collection [calling
functions](#calling-functions) already relies on for the `Word`-parameter
string-literal coercion):

```kitty
hold shape >> Circle(5)
hold idle >> Idle
```

### Compilation

`breed` becomes a plain Rust `enum`, same `Clone, Debug,
serde::Serialize, serde::Deserialize` derive as `litter`. A variant
construction becomes a fully-qualified Rust variant constructor, with the
same payload coercion a `litter` field or `purr` argument gets:

```kitty
breed Shape {
    Circle(#n),
    Idle
}
```
```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Shape {
    Circle(f64),
    Idle,
}
```
```kitty
Circle(5)
```
```rust
Shape::Circle(5f64)
```

## Pattern matching

```kitty
pounce> shape
    Circle(r) >> craft<r * 2>
    Square(s) >> craft<s>
    else> craft<'not a circle or square'>
```

`pounce> subject` matches `subject` (a [`breed`](#breeds) value) against
each arm in turn, running the first arm whose variant matches. Each arm —
`Variant(binding)? >> stmt` — is exactly one statement; `binding` is only
written for a payload-carrying variant (`Circle(r)`, not `Idle`), and is
in scope for that arm's statement, holding the payload value. An optional
final `else> stmt` arm catches every variant not named explicitly above
it — required unless every variant is listed, since the generated Rust
`match` needs to be exhaustive one way or the other (Rust's own compiler
enforces this — `kittine-compiler` doesn't duplicate the check, the same
trust model an unknown [method call](#method-calls) already gets).

Arms are indented **one level under `pounce>` itself**, not beside it —
unlike [`orif>`/`else>`, which sit at the same column as their `if>`](#control-flow):

```kitty
pounce> shape          // col 1
    Circle(r) >> ..     // col 5 -- arms' shared column
    Square(s) >> ..     // col 5
    else> ..             // col 5 -- also an arm, not a sibling of pounce>
```

### `pounce>` as an expression

```kitty
purr describe(shape) #w {
    return (
        pounce> shape
            Circle(r) >> ('circle r=' + r)
            Square(s) >> ('square s=' + s)
            else> 'idle'
    )
}
```

`pounce>` also works as an **expression** — computing a value instead of
running a statement — anywhere an expression is expected (a `purr`'s
`return (...)`, a [`hold`](#plain-local-bindings-hold) binding's value, a
call argument, ...), not just at the start of a statement. The grammar is
identical to the statement form above — same column-indented arms, same
`Variant(binding)? >> ..` shape — except each arm's `..` is a single
*expression* (a value), not a statement, matching Rust's own `match`
arm (`Pattern => expr`) instead of `Pattern => { stmt }`. This closes the
specific gap the statement form left in Kittine's error-handling story: a
function can now unwrap a `breed Result { Ok(#t), Err(#w) }`-shaped value
and `return` the unwrapped payload in one step, the way Rust's own `match`
(or `?`) can — no need to declare a signal, branch imperatively into it,
and read it back out.

Whether a given `pounce>` is the statement or the expression form is
decided entirely by *where* the parser encounters it: at the start of a
statement, it's always the statement form (branch and act); anywhere else,
it's the expression form (branch and compute). There's no ambiguity
between the two — a bare `pounce>` can never appear where an expression is
expected only by coincidence, since the statement form is never itself a
valid expression.

### Compilation

`pounce>` becomes a plain Rust `match`, each pattern qualified with its
variant's owning `breed` name (looked up the same way a variant
construction is — see [Breeds § Compilation](#breeds)), and an `else>`
arm becomes Rust's wildcard `_ =>`:

```kitty
pounce> shape
    Circle(r) >> craft<r>
    else> craft<'other'>
```
```rust
match shape.get() {
    Shape::Circle(r) => {
        leptos::logging::log!("{}", r);
    }
    _ => {
        leptos::logging::log!("other");
    }
}
```

The expression form lowers the same way, just with each arm's `stmt`
replaced by an `expr`, used directly as the surrounding `return (...)`/
`hold`'s value:

```kitty
purr describe(shape) #w {
    return (
        pounce> shape
            Circle(r) >> ('circle r=' + r)
            else> 'idle'
    )
}
```
```rust
pub fn describe(shape: Shape) -> String {
    match shape.clone() {
        Shape::Circle(r) => format!("{}{}", "circle r=", r),
        _ => "idle".to_string(),
    }
}
```

A `Word`-returning `purr` gets one extra piece of care here: a bare
string-literal arm (`else> 'idle'` above) is coerced to an owned `String`
(`.to_string()`), the same coercion an ordinary bare-literal `return
('idle')` already gets — without it, a `match` mixing a computed
`String` arm with an un-coerced `&'static str` arm fails to compile
(`E0308`, arms must all produce the same type). This coercion only
applies to a `pounce>` used directly as a `purr`'s own return value today
— the same coercion for a `pounce>` used as a `hold`/signal value is a
known gap, see [Known limitations](#known-limitations).

## Generics

```kitty
litter Holder<#t> {
    value #t
}
```

A [`litter`](#litters) or [`breed`](#breeds) can declare **at most one**
type parameter — `<#t>` right after its name — minimal groundwork rather
than a full generics system (see [Known limitations](#known-limitations)
below): no multiple parameters, no generic `purr`/`func` (only
`litter`/`breed` can be generic at all). `#t` inside that litter/breed's
own field/variant types then means "whatever concrete type this specific
value was built with":

```kitty
hold numHolder >> Holder { value: 42 }
hold wordHolder >> Holder { value: 'hi' }
```

There's no explicit instantiation syntax at the construction site
(`Holder<#n> { .. }`) — Rust infers the concrete type parameter from the
field value itself, the same way it infers any other generic
constructor's type parameter, so leaving it unwritten is both simpler to
implement and, per [Brevity by design](#brevity-by-design), shorter than
spelling it out would be.

The type parameter can be **bounded** by a [`claw`](#claws) — `<#t:
Named>` — restricting it to types that implement that claw:

```kitty
litter NamedHolder<#t: Named> {
    value #t
}
```

### Compilation

The `<#t>` becomes a single Rust generic parameter, conventionally named
`T`; a bound becomes a real Rust trait bound, checked by Rust's own
compiler at every construction site — Kittine doesn't re-verify it
itself, the same trust model an unknown [method call](#method-calls)
already gets:

```kitty
litter Holder<#t> {
    value #t
}

litter NamedHolder<#t: Named> {
    value #t
}
```
```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Holder<T> {
    pub value: T,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NamedHolder<T: Named> {
    pub value: T,
}
```

## Claws

```kitty
claw Named {
    describe() #w
}
```

`claw Name { method(params) type, .. }` declares a trait — Kittine's term
for a named capability/behavior contract a [`litter`](#litters) or
[`breed`](#breeds) can promise to provide (a "claw" is something a cat
*has*, matching a type *having* a described capability). Each method is a
signature only — a name, a parameter list (using the same [type
tags](#type-tags) as anywhere else), and a return type — **all fully
explicit**: there's no body here for [type inference](#type-inference) to
work from, and a trait's signature has to stay fixed independent of any
one implementation.

A `litter`/`breed` implements a `claw` with a separate `bare` block —
Kittine's term for `impl Claw for Type` (a cat "bares its claws" to show
a capability):

```kitty
bare Named for Point {
    purr describe() #w {
        return ('a point')
    }
}
```

Each method inside a `bare` block is written exactly like an ordinary
[`purr`](#functions-purr) — including the `purr` keyword — with one
difference: the special name `self` is available inside its body with no
declaration needed (the same "no declaration needed" treatment
[`children`](#children) already gets), referring to the value the claw
was implemented for. Read a field off it the same way as any other
[litter field](#litters) — `self.x`.

Calling a claw method on a value needs no new syntax at all — it's just
an ordinary [method call](#method-calls):

```kitty
hold p >> Point { x: 3, y: 4 }
craft<p.describe()>
```

### Compilation

`claw` becomes a plain Rust `trait`, every method taking an implicit
`&self` no `.kitty` author ever writes. `bare Claw for Target { .. }`
becomes `impl Claw for Target { .. }`, reusing the exact same
statement/expression codegen a top-level `purr` gets:

```kitty
claw Named {
    describe() #w
}

bare Named for Point {
    purr describe() #w {
        return ('a point')
    }
}
```
```rust
pub trait Named {
    fn describe(&self) -> String;
}

impl Named for Point {
    fn describe(&self) -> String {
        "a point".to_string()
    }
}
```

## Printing (`craft<...>` / `warn<...>` / `error<...>`)

```kitty
craft<'hello world'>
warn<'careful'>
error<'oh no'>
```

`craft<expr>` logs `expr` to the browser console. `warn<expr>` and
`error<expr>` are the same statement at a different severity — three
levels of the exact same thing, each mapping to Leptos's own
`leptos::logging::log!`/`warn!`/`error!` macro (`warn<...>` prints to
`console.warn`, `error<...>` to `console.error`, browser-devtools-visible
distinctions `craft<...>` alone can't make). String literals are inlined
directly; arrays/`litter`/`breed`/[`stash`](#stashes) values are
formatted with `{:?}`; everything else is formatted with Rust's `{}`
formatter:

| Kittine | Generated Rust |
|---|---|
| `craft<'hello'>` | `leptos::logging::log!("hello");` |
| `warn<'careful'>` | `leptos::logging::warn!("careful");` |
| `error<'oh no'>` | `leptos::logging::error!("oh no");` |
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

### Rendering a list in a view

`spin` can also appear *inside* `return ( ... )` — as a child of an
element, or as the whole view — to render one element per item, instead of
just running logic:

```kitty
return (
    <ul>
        spin<{item}> in <{items}> }{
            <li>{ item }</li>
        }{
    </ul>
)
```

This is a different lowering from the statement form above: instead of an
imperative `for` loop, it becomes a reactive Leptos
[`<For>`](https://leptos.dev), so the list re-renders (efficiently — Leptos
diffs by key) whenever the underlying signal changes:

```rust
<For each=move || items.get() key=|item| format!("{item:?}") let:item>
    <li>
        {{ let item = item.clone(); move || item.clone() }}
    </li>
</For>
```

The key defaults to `format!("{item:?}")` — `Debug` formatting, not
`Display` (`{item}`): every array element type implements `Debug`
(`Num`/`Word`/`Flag`, and now a [`litter`/`breed`
array](#a-litterbreed-name-as-a-prop-or-purr-paramreturn-type) too, since
`gen_litter`/`gen_breed` only ever derive `Debug`, never `Display`, which
isn't something `#[derive(..)]` can produce for an arbitrary struct
anyway), so this default works uniformly across every element type
Kittine can produce, at the cost of a quoted `"like this"`/`5.0`-shaped
key string instead of a bare one — harmless, since a `<For>` key only
needs to be unique and stable, never user-visible. An optional `key(expr)`
clause right before the `}{` fence overrides it — `item` is in scope while
evaluating `expr`, same as in the body:

```kitty
spin<{item}> in items key(item.to_uppercase()) }{
    <li>{ item }</li>
}{
```

```rust
<For each=move || items.get() key=|item| item.clone().to_uppercase() let:item>
    <li>
        {{ let item = item.clone(); move || item.clone() }}
    </li>
</For>
```

`key` isn't a reserved word — it's recognized contextually only in this
one position (immediately after `list`, before `}{`), the same way `in`
is recognized right after `<{item}>`. It stays available as an ordinary
identifier everywhere else — `<{key}> >> 5` declares an ordinary signal
named `key`, unaffected.

`item` is always read as `item.clone()` inside the body, regardless of its
element type — a `{move || ..}` reactive closure needs to be callable more
than once (Leptos re-runs it), which means it can't *move* a non-`Copy`
`item` (a `Word`) out of itself; only `FnOnce` closures can do that. Cloning
a `Copy` type like `Num` costs nothing extra, so there's no reason to only
clone conditionally. Each interpolation also pre-clones `item` into its
own local *before* the closure — needed the moment `item` appears in more
than one place inside the same iteration's body, so sibling closures don't
fight over moving the same original (see [Props](#props) for the same
pre-clone behavior spelled out in full) — harmless overhead when there's
only one use. A `spin` body inside a view can contain more than one child element/text node, just
like any other JSX position.

## Expressions and operators

Precedence, lowest to highest:

1. `||` (logical or — left-associative)
2. `&&` (logical and — left-associative, binds tighter than `||`)
3. Comparison — `>>` (equality), `<`, `<=`, `>`, `>=`, `!=`. Valid as the
   top-level operator of a `<{name}> >> value` assignment/mutation, inside
   an `if>`/`orif>` condition, and generally anywhere an expression is
   valid (a `purr` return, a `craft<...>` argument, a JSX `{ expr }`).
4. `+` `-` (addition, subtraction — left-associative)
5. `*` `/` (multiplication, division — left-associative)
6. unary `-` (negation)
7. primary: numbers, strings, booleans, arrays, calls, type tags,
   identifiers, `<{name}>` reads, parenthesized expressions

```kitty
purr isAdult(#n age) #f {
    return (age >= 18)
}

if><{age}> >= 18
    craft<'adult'>
orif><{age}> < 13
    craft<'child'>
else>
    craft<'teen'>
```

All five comparisons (`<`, `<=`, `>`, `>=`, `!=`) lower to Rust's own
operators of the same name, plus `>>` for `==`; they work between any two
comparable values (numbers, and — via Rust's lexicographic `Ord` for
`String` — words too).

### Logical `&&` / `||`

Combine two or more comparisons into one condition — `&&` binds tighter
than `||`, so `a || b && c` reads as `a || (b && c)`, same as most
languages:

```kitty
purr isWorkingAge(#n age) #f {
    return (age >= 18 && age <= 65)
}

if><{age}> >= 18 && <{status}> >> 'active'
    craft<'eligible'>
orif><{age}> < 13 || <{age}> >= 65
    craft<'discount age'>
```

Each `if>`/`orif>` condition atom (the part on either side of `&&`/`||`)
still needs to start with a `<{name}>` read, same as a single-comparison
condition today — `<{age}> >= 18 && <{status}> >> 'active'` works,
but combining a condition with a bare function call (`isAdult(age) &&
..`) doesn't parse as a condition atom. Outside of `if>`/`orif>`
conditions — a `purr` return, a `craft<...>` argument, a JSX `{ expr }` —
`&&`/`||` work between any two expressions with no such restriction, since
that grammar doesn't require a leading `<{name}>` at all.

Both lower to Rust's own `&&`/`||`, short-circuiting exactly as they do in
Rust.

> **Watch out:** a bare `>` (greater-than) at the top level of
> `craft<expr>` is ambiguous with `craft<...>`'s own closing `>` — wrap it
> in parens to disambiguate: `craft<(age > 18)>`, not `craft<age > 18>`.
> `<`, `<=`, `>=`, and `!=` don't have this problem (only a bare `>` is
> craft's own delimiter); this is the same class of greedy-lexing caveat
> as the `>>`-merging one below, just at the parser level instead of the
> lexer level.

Numbers are floating point (`f64` internally); integer-valued literals are
rendered back as bare Rust integer literals (`0`, `1`, `42`) in most
positions, so generated code reads naturally — except where the literal is
an operand next to an already-`f64`-typed value (a signal initializer, an
arithmetic or comparison operand, a function-call argument), where it's
spelled with an explicit `f64` suffix (`0f64`, `18f64`) instead. This isn't
cosmetic — see [Variables and state § Compilation](#compilation) for why
the bare form doesn't always compile.

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
  the event fires. The reserved identifier `event`, used anywhere inside
  that same handler expression, reads the fired event's string value
  (`event_target_value`) — the closure binds it as `move |ev| ...` instead
  of discarding it, so a text `<input>`'s `onInput={<{query}> >> event}`
  can mutate a signal to whatever the user actually typed, not just a
  fixed literal. `event` is only special inside an `on<Event>` handler's
  own expression; a signal/param/`hold` binding named `event` anywhere
  else in the program is unaffected.
- **Other attributes**: rendered as-is; string literals stay strings,
  `{expr}` attribute values are wrapped in a `move || ...` closure so they
  stay reactive.
- **Kebab-case attribute names**: `data-*`/`aria-*` (and any other
  hyphenated HTML/ARIA attribute — `data-kittine-component`,
  `aria-hidden`, ...) are supported directly; a `-` in this position is
  always parsed as part of the attribute name, never as subtraction,
  since attribute-name position isn't an expression context.

### Compilation

- Elements become literal Leptos `view!{}` markup: `<div>...</div>`.
- `<{name}>` as a child becomes `{move || name.get()}`.
- `{expr}` as a child becomes `{move || <expr>}`.
- `onClick={...}` becomes `on:click=move |_| <mutation>`.
- `onInput={<{query}> >> event}` becomes `on:input=move |ev|
  set_query.update(|n| *n = event_target_value(&ev))` — `event` anywhere in
  the handler expression becomes `event_target_value(&ev)`, and the
  closure binds `ev` instead of discarding it as `_`.
- Any other `attr={expr}` becomes `attr=move || <expr>`.

## Full grammar summary

A quick-reference sketch — see [GRAMMAR.md](GRAMMAR.md) for the complete,
formal EBNF spec (every token, every production, unambiguous precedence)
this summary is deliberately a shorter, denser version of.

```
program      := import* item*
item         := "private"? (component | function | litter | breed | claw)
              | wear                        // never "private" -- see "Claws"
import       := "import" "{" IDENT ("," IDENT)* "}" "from" STRING
component    := "func" IDENT param_list "{" stmt* return_stmt? "}"
function     := "purr" IDENT param_list return_type?
                "{" stmt* return_stmt? "}"
param_list   := "(" (param ("," param)*)? ")"
param        := (type_tag_name | custom_type) IDENT | IDENT | "children"
return_type  := type_tag_name | custom_type
custom_type  := IDENT ("[" "]")?                // a litter/breed name,
                                                 // optionally an array of
                                                 // it -- see "A litter/
                                                 // breed name as a prop
                                                 // or purr param/return
                                                 // type"
type_tag_name:= "#" ("n" | "w" | "f") (("[" "]") | ("{" "}"))?
// An omitted type_tag_name on a param/return type is filled in by
// inference after parsing -- see "Type inference". Array tags ("[]")
// and stash/map tags ("{}") are the cases inference doesn't cover, so
// they're always explicit -- so is a custom_type, scalar or array.
return_stmt  := "return" "(" jsx_node | expr ")"

litter       := "litter" IDENT type_param? "{" litter_field ("," litter_field)* ","? "}"
litter_field := IDENT field_type
breed        := "breed" IDENT type_param? "{" variant ("," variant)* ","? "}"
variant      := IDENT ("(" field_type ")")?
type_param   := "<" "#t" (":" IDENT)? ">"      // at most one, optionally
                                                // bounded by a claw -- see
                                                // "Generics"
field_type   := type_tag_name | "#t" | custom_type   // scalar/array, the
                                                // litter's own generic
                                                // param, or a custom
                                                // litter/breed name
                                                // (optionally an array)

claw         := "claw" IDENT "{" claw_method ("," claw_method)* ","? "}"
claw_method  := IDENT "(" (claw_param ("," claw_param)*)? ")" field_type
claw_param   := field_type IDENT               // always explicit -- no body
                                                // for inference to work from
wear         := "bare" IDENT "for" IDENT "{" function* "}"
// "bare Claw for Target { .. }" == Rust's "impl Claw for Target { .. }" --
// each function reuses the "function" production verbatim (still starts
// with "purr"), with the implicit `self` available in its body with no
// declaration needed, same treatment "children" already gets in a func.

stmt         := var_stmt | craft_stmt | if_stmt | spin_stmt | pounce_stmt | expr_stmt
var_stmt     := "<{" IDENT "}>" ">>" expr
craft_stmt   := ("craft<" | "warn<" | "error<") craft_expr ">"
// three levels of the same statement -- see "Printing"
if_stmt      := "if>" condition INDENT_BLOCK
                ("orif>" condition INDENT_BLOCK)*
                ("else>" INDENT_BLOCK)?
condition    := cond_or
cond_or      := cond_and ("||" cond_and)*
cond_and     := cond_atom ("&&" cond_atom)*
cond_atom    := "<{" IDENT "}>" cmp_op expr
cmp_op       := ">>" | "<" | "<=" | ">" | ">=" | "!="
spin_stmt    := "spin" "<{" IDENT "}>" "in" expr "}{" stmt* "}{"
pounce_stmt  := "pounce>" expr pounce_arm+ pounce_else?
                // pounce_arm/pounce_else are indented one level *under*
                // pounce>'s own column, all sharing one column together
                // (unlike orif>/else>, which sit beside if> itself)
pounce_arm   := IDENT ("(" IDENT ")")? ">>" stmt
pounce_else  := "else>" stmt
expr_stmt    := expr

pounce_expr  := "pounce>" expr pounce_expr_arm+ pounce_expr_else?
                // pounce_stmt's value-producing sibling -- reachable from
                // `primary` below, so it can appear anywhere an
                // expression is expected (a return_stmt's expr, a
                // hold_stmt's value, ...), not just at the start of a
                // statement. Same column-indentation as pounce_stmt --
                // see "Pattern matching" § pounce> as an expression.
pounce_expr_arm  := IDENT ("(" IDENT ")")? ">>" expr
pounce_expr_else := "else>" expr

closure_expr := ("|" (IDENT ("," IDENT)*)? "|" | "||") expr
                // a closure literal -- lowers verbatim to a Rust closure;
                // see "Closures". The zero-param form shares the lexer's
                // "||" token with logic_or's own "||" -- unambiguous,
                // since it only reaches `primary` (never `logic_or`'s
                // infix position) there.

expr         := logic_or
logic_or     := logic_and ("||" logic_and)*
logic_and    := equality ("&&" equality)*
equality     := additive (cmp_op additive)?
craft_expr   := craft_or
craft_or     := craft_and ("||" craft_and)*
craft_and    := craft_equality ("&&" craft_equality)*
craft_equality := additive ((">>" | "<" | "<=" | ">=" | "!=") additive)?  // no bare ">"; see Expressions and operators
additive     := term (("+" | "-") term)*
term         := unary (("*" | "/") unary)*
unary        := "-" unary | "&" unary | postfix
// "&" is a real Rust reference (Expr::Ref) -- see "Reference operator"
postfix      := primary ( ("." IDENT arg_list?) | arg_list )*
// ".IDENT" with an arg_list is a method call; with none, a litter field
// read (Expr::FieldAccess) -- see "Litters". A bare arg_list calls the
// result of an expression.
arg_list     := "(" (expr ("," expr)*)? ")"
primary      := NUMBER | STRING | BOOL | ARRAY | TYPE_TAG | CALL
              | STRUCT_INIT | IDENT | pounce_expr | closure_expr
              | "<{" IDENT "}>" (">>" expr)?  // >> here is inline mutation, not comparison
              | "(" expr ")" | tuple

array        := "[" (expr ("," expr)*)? "]"
type_tag     := type_tag_name unary
call         := IDENT "(" (expr ("," expr)*)? ")"
// A `breed` variant construction (`Circle(5)`) and a bare unit-variant
// reference (`Idle`, via the plain IDENT primary) share this same syntax
// with a purr call/variable read -- told apart by which one `IDENT`
// actually names, via the whole-import-graph Signatures map (see
// "Compilation model").
struct_init  := IDENT "{" (struct_field ("," struct_field)* ","?)? "}"
struct_field := IDENT ":" expr
// IDENT == "stash" is the one reserved exception: same grammar, but a
// String-keyed map literal (Expr::StructInit lowered to a HashMap), not
// a real litter -- "stash" is never itself a declared litter name. See
// "Stashes".
tuple        := "(" expr "," expr ("," expr)* ","? ")"  // a lone "(" expr ")" is just grouping

jsx_node     := jsx_element | STRING | "<{" IDENT "}>" | "{" expr "}"
              | jsx_spin
jsx_spin     := "spin" "<{" IDENT "}>" "in" expr ("key" "(" expr ")")? "}{" jsx_node* "}{"
jsx_element  := "<" IDENT jsx_attr* ("/>" | ">" jsx_node* "</" IDENT ">")
jsx_attr     := attr_name "=" (STRING | "{" expr "}")
attr_name    := IDENT ("-" IDENT)*  // data-*/aria-* kebab-case names

STRING       := "'" char* "'" | '"' char* '"'
BOOL         := "yes>" | "no>"
```

## Compilation model

Every generated Rust file starts with:

```rust
// Generated by kittine-compiler. Do not edit by hand.
#![allow(unused_braces, unused_variables, dead_code, unused_imports)]

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::*;
use leptos_router::hooks::*;
use leptos_meta::*;
```

followed by one `mod` + `use` pair per `import`, and then one item per
`func`/`purr`/`litter`/`breed`/`claw`/`bare` in the source file, in
source order: a `func` becomes `#[component] pub fn Name(..) -> impl
IntoView { ... }`; a `purr` becomes a plain `pub fn name(..) ->
ReturnType { ... }`; a `litter` becomes a `#[derive(Clone, Debug,
serde::Serialize, serde::Deserialize)] pub struct Name { ... }`; a `breed`
becomes the same derive on a `pub enum Name { ... }`; a `claw` becomes a
`pub trait Name { ... }`; a `bare
Claw for Target { .. }` becomes an `impl Claw for Target { ... }` (see
[Litters](#litters), [Breeds](#breeds), [Claws](#claws)). The
`leptos_router` and
`leptos_meta` imports are unconditional (see [Routing](#routing)) —
`unused_imports` is allowed at the crate level so files that don't route
or set page metadata stay warning-free. `leptos_meta` brings `<Title>`,
`<Meta>`, `<Link>`, `<Stylesheet>`, etc. into scope as plain Leptos
components (composed exactly like `<Router>`/`<Route>`, no dedicated
Kittine syntax) — wired in, but not yet exercised by a real `.kitty` page
in this repo; using one requires calling Leptos's own
`provide_meta_context()` somewhere in the app root first (not yet a
documented Kittine pattern).

`kittine-compiler build <entry>.kitty` compiles the whole reachable
`import` graph, not just `<entry>` itself, in two passes: first a
lex+parse-only walk collects every reachable file's `purr`/`litter`/
`breed` signatures into one `Signatures` (`purr` param/return types,
`litter` field types, `breed` variant payload types) — this is what makes
the `Word`-parameter string-literal coercion above work across `import`s
(for a `purr` argument *or* a `litter` field *or* a `breed` variant
payload), not just within one file — then each file is actually generated
using that whole-graph map. `claw`/`bare` deliberately aren't part of
this map — a claw method is only ever called via `receiver.method(args)`,
which already renders verbatim regardless of any other type information
(see [Method calls](#method-calls)), so there's nothing for a whole-graph
signature lookup to add there. Every file gets parsed twice across a full
build — real, but cheap next to what `cargo`/`wasm-bindgen` cost
downstream.

`kittine-compiler build` also only actually *rewrites* a dependency's
`.rs` file when the freshly generated content differs from what's already
there. A `.kitty` file whose only edit doesn't change codegen (a comment,
for instance — comments carry no codegen effect at all) leaves its output
file's mtime untouched. This matters because `cargo`, `wasm-bindgen`, and
`vite-plugin-kittine`'s own build-freshness check all key off file mtimes
to decide whether to redo work; unconditionally rewriting every reachable
file on every build made
all of them look freshly modified regardless of what actually changed.

## Known limitations

These are intentional scope boundaries of the current prototype, not bugs:

- **Type inference is local to one function/component and scalars-only.**
  See [Type inference](#type-inference) — it doesn't propagate a type
  through a call to another `purr`, and array-typed
  props/params/returns (scalar or [custom-type](#a-litterbreed-name-as-a-prop-or-purr-paramreturn-type))
  still need an explicit tag written out. Using a name only as a
  [`pounce>`](#pattern-matching) subject gives inference no clue either —
  a name only ever pattern-matched against a `breed`'s variants, never
  used arithmetically/in a comparison, still falls back to `Word`, which
  then fails to compile against the pattern's real `breed` type
  (`E0308`) — found wiring `pounce>`-as-expression into a real
  `example-app` function; give the param an explicit `breed` name tag
  ([see above](#a-litterbreed-name-as-a-prop-or-purr-paramreturn-type))
  instead of leaving it untyped.
- **A `pounce>` expression's bare-string-literal-arm coercion only covers
  a `purr`'s own `return (...)` value.** See [`pounce>` as an
  expression](#pounce-as-an-expression): a `Word`-returning `purr` whose
  `pounce>` mixes a computed arm (e.g. `format!(..)`) with a bare literal
  arm (`else> 'idle'`) gets the literal coerced to an owned `String` so
  the generated `match`'s arms agree on a type. The same mixed-arm shape
  used as a [`hold`](#plain-local-bindings-hold) binding or `<{name}> >>
  ..` signal value instead doesn't get this coercion yet (neither has a
  declared type for codegen to coerce *toward* the way a `purr`'s
  `return_type` does) — avoid a bare string-literal arm there, or wrap it
  in an explicit `#w` tag, until this is closed.
- **Generics are groundwork, not a full system.** See
  [Generics](#generics): a `litter`/`breed` may have at most one type
  parameter (optionally bounded by a [`claw`](#claws)), but still no
  multiple parameters and no generic `purr`/`func` (only `litter`/`breed`
  can be generic at all).
- **A `litter`/`breed`/`claw` field/variant/method type is always
  explicit — there's no inference for these positions.** Unlike a `purr`
  param or prop (see [Type inference](#type-inference)), a `litter`
  field, `breed` variant payload, or `claw` method signature always needs
  its own `#n`/`#w`/`#f`/`#t`/custom-type-name written out. A `bare`
  block's own method *bodies* do get ordinary `purr`-style inference —
  only the signature positions above don't.
- **A claw method call gets no argument type coercion.** [Method
  calls](#method-calls) already render an argument bare, with no lookup
  against the callee's actual parameter types (Kittine doesn't track a
  method-call receiver's type) — this applies to a `claw` method the same
  way it applies to any other method call, so a bare string-literal
  argument to a `Word`-typed claw-method parameter doesn't get the
  `.to_string()` coercion a same-file `purr` call or `litter`
  field/`breed` variant construction would. Give the argument as a
  `Word`-typed signal/prop instead of a bare literal to avoid this.
- **A `litter`/`breed`/`claw` name isn't reserved from colliding with a
  real Rust type/trait.** Naming one `Box`, `String`, `Vec`, `Clone`,
  etc. shadows the real Rust prelude item within that generated file —
  it happens to still compile in every case tried so far (a local item
  takes precedence over a prelude import in Rust), but produces a
  confusing generated file. Kittine doesn't warn about this.
- **No cross-check between a `claw`'s declared signatures and a `bare`
  block's method bodies.** A `bare Claw for Target { .. }` isn't verified
  against `Claw`'s own method list at all by Kittine — a missing method,
  an extra one, or a mismatched signature is caught by Rust's own
  trait-impl type checking once generated (`E0046`/`E0050`/etc.), the
  same trust model an unknown method call already gets.
- **Routing is CSR-only, and has no dedicated Kittine syntax.** [Routing](#routing)
  works via `leptos_router`'s own components composed as-is — real, but
  client-side-rendered only (no SSR/SSG integration yet). Reading a
  dynamic segment (`use_params_map().get().get("id")`) and [programmatic
  navigation](#programmatic-navigation) both work, verified end-to-end.
- **A Leptos hook (`use_navigate`, `use_context`, and others) called
  *inside* a lazily-evaluated position — a JSX event handler, for
  instance — resolves its context wrong, silently at compile time and
  loudly at runtime.** `onClick={use_navigate()('/', ..)}` compiles fine
  but panics the moment it's clicked
  (`You cannot call use_navigate outside a <Router>`), because the
  hook's context lookup runs against whatever reactive owner is active
  *when the closure body executes* — correct during a component's
  synchronous setup, not by the time a click fires later from the
  browser's event loop, even though the element is still physically
  inside `<Router>` in the DOM. Call context-dependent hooks eagerly, with
  [`hold`](#plain-local-bindings-hold), not from inside an event handler
  — see [Programmatic navigation](#programmatic-navigation). This is a
  real Leptos behavior, not Kittine-specific, but Kittine's codegen
  doesn't (yet) distinguish "eager" from "lazy" expression positions to
  warn about it.
- **Whole-graph `purr` signature lookup (see [Calling
  functions](#calling-functions)) is a flat, unnamespaced map by bare
  name.** Two different files defining a `purr` with the same name is
  already an ambiguity Kittine has no namespacing to resolve — this just
  means the `Word`-parameter string-literal coercion could theoretically
  pick up the wrong one's signature in that exact collision case. Rust's
  own `use` resolution is what actually binds a call site to a specific
  function either way; this map only ever informs a codegen *hint*, never
  which function actually gets called.
- **`craft<...>` inside `if>`/`orif>`/`else>`/(statement-position) `spin` runs once,
  at component setup,
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
- **A [closure](#closures)'s body is a single expression, and its params
  are always untyped.** `|param, ..| expr` has no block-body form (no
  `{ stmt* expr }`) and no way to write an explicit param type — matching
  every other value-computing construct's "one expression" shape (a
  `purr` return, a `pounce>` arm), but narrower than a real Rust closure,
  which allows both. Rust infers each param's type from how the closure
  is actually used (e.g. `Vec<T>::filter`'s `&T`), so this is rarely a
  practical limit for the interop use case closures exist for.
- **An `if>`/`orif>` condition atom combined with `&&`/`||` still needs to
  start with `<{name}>`.** `<{age}> >= 18 && <{status}> >> 'active'` works;
  combining with a bare function call or computed expression as one side
  of the `&&`/`||` (`isAdult(age) && ..`) doesn't parse as a condition —
  only the general expression grammar (`purr` returns, `craft<...>`, JSX
  `{ expr }`) allows arbitrary expressions on either side of `&&`/`||`.
- **`kittine-compiler fmt`/`lint` diagnostics carry no source position.**
  `ast::Stmt`/`ast::Expr` keep no line/column at all once parsed, so a
  `lint` warning names the construct ("unused parameter 'x' in purr
  'f'"), not a location — and `fmt` cannot preserve `//` comments (the
  lexer discards them before the parser ever sees them), so it refuses to
  reformat a file containing one unless `--force` is passed, accepting
  they'll be lost. `lint`'s unused-import/-item checks are a whole-scope
  name-occurrence walk, not a reachability analysis — conservative by
  design (a `private purr` that only calls itself, with no other caller,
  won't be flagged), biased toward missing real dead code over ever
  flagging live code as dead.
- **`craft<...>` only knows to use `{:?}` (`Debug`) instead of `{}`
  (`Display`) for an array/`litter`/`breed`/`stash` *literal* written
  directly inside the `craft<...>`.** `craft<[1, 2, 3]>` and
  `craft<stash{ a: 1 }>` both work; `craft<myArraySignal>` or
  `craft<myStash>` (a bare identifier *naming* an array/`litter`/`breed`/
  [`stash`](#stashes)-typed signal or prop, rather than a literal) still
  renders with `{}`, which fails to compile (`E0277`, "doesn't implement
  `Display`") since none of those types implement it. Kittine doesn't
  track a bare identifier's type at `craft<...>` codegen time to know
  which format specifier it needs — write `craft<[..]>`/
  `craft<Litter{..}>`/`craft<stash{..}>` directly, or convert the value to
  a `Word` first (e.g. a debug-formatting `purr` helper), to work around
  this until a real fix lands.
- **[`.toDate(pattern)`](#date-and-time) needs a full date+time pattern
  and panics on a bad parse.** `chrono::NaiveDateTime::parse_from_str`
  (what `.toDate` lowers to) requires the pattern to describe both a date
  *and* a time — a date-only pattern like `"%Y-%m-%d"` alone doesn't parse
  via this path even against a date-only string, and there's no
  `.toDate`-equivalent for constructing a fixed calendar date some other
  way (see [`now>`](#date-and-time)'s own note on this). The generated
  `.unwrap()` also panics on a malformed input rather than returning a
  `Result` — the same "no un-modeled-failure story yet" limitation every
  other interop escape hatch already has (see [ROADMAP.md § Production
  readiness](ROADMAP.md#production-readiness)), not a new gap this method
  introduces.
