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
  - [Method calls](#method-calls)
  - [Tuples](#tuples)
- [Modules and imports](#modules-and-imports)
  - [Visibility](#visibility)
- [Component composition](#component-composition)
  - [Children](#children)
- [Routing](#routing)
  - [Dynamic route segments](#dynamic-route-segments)
  - [Programmatic navigation — a real, current gap](#programmatic-navigation--a-real-current-gap)
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
`<<Word>>`, `<<Flag>>`, or an array of one of those (`<<Num[]>>`,
`<<Word[]>>`, `<<Flag[]>>`). There is no prop-type inference. The one
exception is the special `children` parameter — see
[Children](#children) — which takes no type tag at all.

### Compilation

| Kittine | Generated Rust |
|---|---|
| `func Nav(<<Word>> active) { .. }` | `pub fn Nav(active: String) -> impl IntoView { .. }` |
| `func Card(<<Num>> price, <<Flag>> onSale) { .. }` | `pub fn Card(price: f64, onSale: bool) -> impl IntoView { .. }` |
| `func NavList(<<Word[]>> items) { .. }` | `pub fn NavList(items: Vec<String>) -> impl IntoView { .. }` |

Reading a `Word` prop or any array-typed prop clones it (`items.clone()`)
rather than moving it, since a prop may be read from more than one
reactive closure inside the component body — neither `String` nor `Vec<T>`
is `Copy` the way `Num`/`Flag` are.

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

Calling a known `purr` — one defined in the same file, *or* reached
through the whole `import` graph — also gets one more piece of real type
checking: a bare string-literal argument at a `<<Word>>` parameter
position renders as an owned `String`, not a borrowed `&str`, because
`kittine-compiler build` collects every reachable file's `purr` signatures
before generating any single file's code (see [Compilation
model](#compilation-model)):

| Kittine | Generated Rust |
|---|---|
| `greet('World')` (where `greet(<<Word>> name)`) | `greet("World".to_string())` |

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

### Visibility

```kitty
private purr internalHelper(<<Num>> n) <<Num>> {
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
func Card(<<Word>> title, children) {
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
            <h3>{move || title.clone()}</h3>
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

### Programmatic navigation — a real, current gap

`leptos_router::hooks::use_navigate()` returns a closure you call with
`(&str, NavigateOptions)`. The path half is easy (`use_navigate()('/',
..)` — calling the result of an expression, not a named function — parses
fine as ordinary Kittine syntax), but `NavigateOptions` has no `Default`
value expressible in Kittine: there's no `Type::method()` /
`Path::CONST`-style path-qualified expression, so `NavigateOptions::default()`
or `Default::default()` can't be written. This was discovered by actually
trying to wire up a working example, not assumed — see [Known
limitations](#known-limitations). A `<A>` link (client-side, no full
reload) covers most real navigation needs in the meantime; reach for
`use_navigate()` only once path-qualified calls exist.

Everything else `leptos_router` supports — nested routes (`<ParentRoute>`
+ `<Outlet/>`), wildcard/catch-all segments — is available the same
zero-new-syntax way. Server-side rendering / static generation (Leptos
supports both) is a separate, bigger piece of work — see
[ROADMAP.md](ROADMAP.md#next-up).

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
annotate a value's type. There are three scalar type names, plus an array
form for each:

| Tag | Matches |
|---|---|
| `<<Num>>` | number literals |
| `<<Word>>` | string literals |
| `<<Flag>>` | boolean literals |
| `<<Num[]>>` / `<<Word[]>>` / `<<Flag[]>>` | array literals of the matching element type |

```kitty
<{scores}> >> <<Num[]>> [10, 20, 30]
```

When the tagged value is a literal, the compiler checks it against the tag
at compile time and rejects a mismatch (`<<Num>> 'oops'` is a parse error;
for an array tag, every literal *element* is checked too — `<<Num[]>>
['a', 'b']` is also a parse error). When the tagged value is a variable
read or a computed expression (its static type isn't known at parse time),
the annotation is trusted rather than checked. Either way, the tag itself
is erased during code generation — Rust's own type inference already
gives the underlying value the right type, so `<<Num>> 0` and a bare `0`
generate identical Rust.

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
<For each=move || items.get() key=|item| format!("{item}") let:item>
    <li>
        {move || item.clone()}
    </li>
</For>
```

The key is always `format!("{item}")` — every array element type
(`Num`/`Word`/`Flag`) implements `Display`, so this works uniformly without
needing a separate "what's the identity of this item" concept.

`item` is always read as `item.clone()` inside the body, regardless of its
element type — a `{move || ..}` reactive closure needs to be callable more
than once (Leptos re-runs it), which means it can't *move* a non-`Copy`
`item` (a `Word`) out of itself; only `FnOnce` closures can do that. Cloning
a `Copy` type like `Num` costs nothing extra, so there's no reason to only
clone conditionally. A `spin` body inside a view can contain more than one child element/text node, just
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
purr isAdult(<<Num>> age) <<Flag>> {
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
purr isWorkingAge(<<Num>> age) <<Flag>> {
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
item         := "private"? (component | function)
import       := "import" "{" IDENT ("," IDENT)* "}" "from" STRING
component    := "func" IDENT param_list "{" stmt* return_stmt? "}"
function     := "purr" IDENT param_list type_tag_name
                "{" stmt* return_stmt? "}"
param_list   := "(" (param ("," param)*)? ")"
param        := type_tag_name IDENT | "children"
type_tag_name:= "<<" ("Num" | "Word" | "Flag") ("[" "]")? ">>"
return_stmt  := "return" "(" jsx_node | expr ")"

stmt         := var_stmt | craft_stmt | if_stmt | spin_stmt | expr_stmt
var_stmt     := "<{" IDENT "}>" ">>" expr
craft_stmt   := "craft<" craft_expr ">"
if_stmt      := "if>" condition INDENT_BLOCK
                ("orif>" condition INDENT_BLOCK)*
                ("else>" INDENT_BLOCK)?
condition    := cond_or
cond_or      := cond_and ("||" cond_and)*
cond_and     := cond_atom ("&&" cond_atom)*
cond_atom    := "<{" IDENT "}>" cmp_op expr
cmp_op       := ">>" | "<" | "<=" | ">" | ">=" | "!="
spin_stmt    := "spin" "<{" IDENT "}>" "in" expr "}{" stmt* "}{"
expr_stmt    := expr

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
unary        := "-" unary | postfix
postfix      := primary ( ("." IDENT arg_list) | arg_list )*  // method call, or calling the result of an expression
arg_list     := "(" (expr ("," expr)*)? ")"
primary      := NUMBER | STRING | BOOL | ARRAY | TYPE_TAG | CALL | IDENT
              | "<{" IDENT "}>" (">>" expr)?  // >> here is inline mutation, not comparison
              | "(" expr ")" | tuple

array        := "[" (expr ("," expr)*)? "]"
type_tag     := type_tag_name unary
call         := IDENT "(" (expr ("," expr)*)? ")"
tuple        := "(" expr "," expr ("," expr)* ","? ")"  // a lone "(" expr ")" is just grouping

jsx_node     := jsx_element | STRING | "<{" IDENT "}>" | "{" expr "}"
              | jsx_spin
jsx_spin     := "spin" "<{" IDENT "}>" "in" expr "}{" jsx_node* "}{"
jsx_element  := "<" IDENT jsx_attr* ("/>" | ">" jsx_node* "</" IDENT ">")
jsx_attr     := IDENT "=" (STRING | "{" expr "}")

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
```

followed by one `mod` + `use` pair per `import`, and then one item per
`func`/`purr` in the source file, in source order: a `func` becomes
`#[component] pub fn Name(..) -> impl IntoView { ... }`; a `purr` becomes a
plain `pub fn name(..) -> ReturnType { ... }`. The `leptos_router` imports
are unconditional (see [Routing](#routing)) — `unused_imports` is
allowed at the crate level so files that don't route stay warning-free.

`kittine-compiler build <entry>.kitty` compiles the whole reachable
`import` graph, not just `<entry>` itself, in two passes: first a
lex+parse-only walk collects every reachable file's `purr` signatures into
one map (this is what makes the `Word`-parameter string-literal coercion
above work across `import`s, not just within one file), then each file is
actually generated using that whole-graph map. Every file gets parsed
twice across a full build — real, but cheap next to what `cargo`/
`wasm-bindgen` cost downstream.

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

- **No prop type inference.** Every prop and `purr` return type needs an
  explicit `<<Num>>`/`<<Word>>`/`<<Flag>>` tag — there's no way to omit it
  and have the compiler figure it out.
- **Routing is CSR-only, and has no dedicated Kittine syntax.** [Routing](#routing)
  works via `leptos_router`'s own components composed as-is — real, but
  client-side-rendered only (no SSR/SSG integration yet). Reading a
  dynamic segment (`use_params_map().get().get("id")`) is real and
  verified end-to-end (see [Dynamic route
  segments](#dynamic-route-segments)); programmatic navigation
  (`use_navigate()`) is not yet, for a specific reason — see the next
  item.
- **No path-qualified expressions (`Type::method()`, `Type::CONST`).**
  Kittine's grammar has no `::`. This blocks `NavigateOptions::default()`
  (or `Default::default()`), so `use_navigate()`'s second argument can't
  be constructed — see [Programmatic navigation](#programmatic-navigation--a-real-current-gap).
  Discovered while actually trying to wire up a working example, not
  assumed up front.
- **Whole-graph `purr` signature lookup (see [Calling
  functions](#calling-functions)) is a flat, unnamespaced map by bare
  name.** Two different files defining a `purr` with the same name is
  already an ambiguity Kittine has no namespacing to resolve — this just
  means the `Word`-parameter string-literal coercion could theoretically
  pick up the wrong one's signature in that exact collision case. Rust's
  own `use` resolution is what actually binds a call site to a specific
  function either way; this map only ever informs a codegen *hint*, never
  which function actually gets called.
- **No re-exports.** `private` (see [Visibility](#visibility)) controls
  whether an item can be imported *at all*, but there's no way for a file
  to import something and then re-expose it under its own name for a
  third file to import — every import has to go straight to the file that
  actually defines the item.
- **Mutating a `Word` signal directly to a brand-new literal may not
  compile.** `<{label}> >> 'reset'` as a *mutation* (not the signal's
  first/declaring occurrence) can render a bare `&str` assigned into an
  owned `String` — concatenation (`<{label}> >> 'x' + <{label}>`) is
  unaffected, since `format!(..)` always produces an owned `String`
  regardless of its inputs.
- **A view-position `spin` has no `key` control.** List rendering
  (`spin` inside `return ( ... )`) always keys by `format!("{item}")` —
  there's no way to key by something else (an id field, an index) yet,
  which matters once array elements stop being bare scalars.
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
- **An `if>`/`orif>` condition atom combined with `&&`/`||` still needs to
  start with `<{name}>`.** `<{age}> >= 18 && <{status}> >> 'active'` works;
  combining with a bare function call or computed expression as one side
  of the `&&`/`||` (`isAdult(age) && ..`) doesn't parse as a condition —
  only the general expression grammar (`purr` returns, `craft<...>`, JSX
  `{ expr }`) allows arbitrary expressions on either side of `&&`/`||`.
