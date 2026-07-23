# Kittine Formal Grammar

This is the complete, formal grammar for Kittine (`.kitty`), derived directly
from `kittine-compiler`'s lexer (`src/lexer.rs`) and parser (`src/parser.rs`)
— every production here corresponds to an actual parser function, not an
idealized or aspirational syntax. [LANGUAGE.md § Full grammar
summary](LANGUAGE.md#full-grammar-summary) is a shorter, denser version of
this same grammar for quick reference; this document is the authoritative
one when the two ever appear to disagree (they shouldn't — file an issue if
they do).

This is a **grammar freeze target for Phase 6** ([ROADMAP.md § Full
vision](ROADMAP.md#full-vision-phased-honest)), not a promise that nothing
here will change before then — Kittine has already had several breaking
syntax changes in rapid succession (see `CHANGELOG.md`). It's the spec for
what's real *today*.

## Notation

EBNF-style, close to what the parser actually implements:

- `"literal"` — an exact token (a keyword, punctuation, or fused
  keyword+punctuation form like `if>`).
- `UPPER_CASE` — a lexical token class (see [Lexical
  grammar](#lexical-grammar)).
- `lower_snake_case` — a syntactic (parser-level) production.
- `A B` — sequence.
- `A | B` — alternation.
- `A?` — zero or one.
- `A*` — zero or more.
- `A+` — one or more.
- `( ... )` — grouping.
- `// comment` — a note about the production, not part of the grammar.

Whitespace and `// line comments` are insignificant everywhere **except**
inside `if>`/`orif>`/`else>` and `pounce>` bodies, where a statement's
starting column is significant (see [Indentation
rules](#indentation-rules)).

## Lexical grammar

```
IDENT     := [a-zA-Z_][a-zA-Z0-9_]*
            // minus the reserved words below when they appear in a
            // keyword position; unreserved elsewhere (e.g. a litter
            // field or breed variant may be named `func` in principle,
            // though not recommended)
NUMBER    := [0-9]+ ("." [0-9]+)?
STRING    := "'" CHAR* "'" | '"' CHAR* '"'
            // fully interchangeable quote styles; \n \t \\ and \<quote>
            // are the only recognized escapes
BOOL      := "yes>" | "no>"

// Type-tag sigils -- lexed as a single fused token (`#` + one letter),
// never `#` and the letter as two separate tokens.
TYPE_NUM     := "#n"
TYPE_WORD    := "#w"
TYPE_FLAG    := "#f"
TYPE_GENERIC := "#t"

// Fused keyword+punctuation forms -- also single tokens, not a keyword
// followed by a separate punctuation token.
KEYWORD_IF     := "if>"
KEYWORD_ORIF   := "orif>"
KEYWORD_ELSE   := "else>"
KEYWORD_CRAFT  := "craft<"
KEYWORD_POUNCE := "pounce>"

// Plain keywords
KEYWORD_FUNC    := "func"
KEYWORD_RETURN  := "return"
KEYWORD_SPIN    := "spin"
KEYWORD_PURR    := "purr"
KEYWORD_PRIVATE := "private"
KEYWORD_IMPORT  := "import"
KEYWORD_FROM    := "from"
KEYWORD_EXPORT  := "export"
KEYWORD_HOLD    := "hold"
KEYWORD_LITTER  := "litter"
KEYWORD_BREED   := "breed"
KEYWORD_CLAW    := "claw"
KEYWORD_BARE    := "bare"
KEYWORD_FOR     := "for"

// Operators and punctuation
"<{"  ">>"  "<"  ">"  "/>"  "</"  "<="  ">="  "!="  "&&"  "||"  "|"
"("  ")"  "{"  "}"  "["  "]"  ","  "."  ":"  "::"  "="
"+"  "-"  "*"  "/"  "!"
```

A line comment (`// ...`, to end of line) is stripped by the lexer and
produces no token. There is no block-comment form.

## Indentation rules

`if>`/`orif>`/`else>` and `pounce>` are the only indentation-sensitive
constructs — everything else is fully delimited by explicit punctuation
(`{`/`}`, `(`/`)`, `<{`/`}>`, `}{`, and so on) and whitespace-insensitive.

- **`if>` / `orif>` / `else>`**: `orif>`/`else>` are *siblings* of `if>` —
  each must start at exactly `if>`'s own column. Each branch's body is
  every statement indented *further* than that shared column, up to (not
  including) the next sibling or a dedent below it.
- **`pounce>`**: arms (and an optional trailing `else>`) are *children* of
  `pounce>`, one level further indented — their shared column is
  whatever column the very first arm actually starts at, which must be
  greater than `pounce>`'s own column. Unlike `if>`'s branches, each arm's
  body is exactly one statement, not an indented block. The same rule
  applies verbatim to `pounce>` used as an expression (see `pounce_expr`
  under [Expressions](#expressions) below, and [LANGUAGE.md § `pounce>` as
  an expression](LANGUAGE.md#pounce-as-an-expression)) — the only
  difference is each arm's body is `expr`, not `stmt`.

## Syntactic grammar

### Program structure

```
program   := import* item*
import    := "export"? "import" "{" IDENT ("," IDENT)* "}" "from" STRING
item      := "private"? (component | function | litter | breed | claw)
           | wear
           // "wear" (bare .. for ..) is never "private" -- an impl block
           // isn't itself an importable name
```

Imports must appear before any `item`. `export import ..` re-exports the
imported names (`pub use` in the generated Rust) so a third file can import
them through this one. `private` on an `item` opts it out of being
importable from another file (enforced by Rust's own visibility rules, not
re-checked by Kittine).

### Components and functions

```
component   := "func" IDENT param_list "{" stmt* return_view? "}"
function    := "purr" IDENT param_list return_type?
               "{" stmt* return_expr "}"
return_view := "return" "(" jsx_node ")"
return_expr := "return" "(" expr ")"
return_type := type_tag_name | custom_type
             // a scalar tag, or a bare litter/breed name (optionally
             // "[]"-suffixed) -- unambiguous here (nothing but a return
             // type or the body's opening "{" can appear in this
             // position), unlike `param` below

param_list  := "(" (param ("," param)*)? ")"
param       := (type_tag_name | custom_type) IDENT | IDENT | "children"
             // `custom_type IDENT` (`DocEntry entry`) is told apart from
             // a lone untyped `IDENT` (`entry`, to be inferred) by
             // lookahead: a capitalized identifier immediately followed
             // by another identifier (with an optional "[" "]" in
             // between) names a type; one with nothing following is
             // just the param's own name
custom_type := IDENT ("[" "]")?
             // a litter/breed name, optionally an array of it -- same
             // "[]" convention type_tag_name uses for a scalar array
```

A `component`'s `return_view` is optional (an omitted one compiles to
`view! { <></> }`); a `function`'s `return_expr` is mandatory. `children`
is a reserved param name with no type tag of its own — see [LANGUAGE.md §
Children](LANGUAGE.md#children).

A `param`'s type (and a `function`'s own `return_type`) is optional when
it's a scalar (`type_tag_name`) — an omitted one is filled in after
parsing by the type-inference pass (`src/infer.rs`; see [LANGUAGE.md §
Type inference](LANGUAGE.md#type-inference)), which never runs on
`litter` fields, `breed` variant payloads, or a `custom_type` (those are
always explicit) — see [LANGUAGE.md § A litter/breed name as a prop or
purr param/return type](LANGUAGE.md#a-litterbreed-name-as-a-prop-or-purr-paramreturn-type).

### Litters and breeds

```
litter       := "litter" IDENT type_param? "{" litter_field ("," litter_field)* ","? "}"
litter_field := IDENT field_type

breed        := "breed" IDENT type_param? "{" variant ("," variant)* ","? "}"
variant      := IDENT ("(" field_type ")")?

type_param   := "<" TYPE_GENERIC (":" IDENT)? ">"
               // at most one type parameter; the optional ":" IDENT is a
               // claw bound (Rust's own trait system checks it once
               // generated, not re-verified here) -- see
               // LANGUAGE.md § Generics
field_type   := type_tag_name | TYPE_GENERIC | custom_type
               // a scalar/array tag, this litter/breed's own generic
               // parameter, or another litter/breed's name (see
               // custom_type under "Components and functions" above --
               // same "[]" convention for an array of it)
```

A `breed` variant carries at most one payload value (`Circle(#n)`) or none
(`Idle`). See [LANGUAGE.md § Litters](LANGUAGE.md#litters) and [§
Breeds](LANGUAGE.md#breeds).

### Claws

```
claw       := "claw" IDENT "{" claw_method ("," claw_method)* ","? "}"
claw_method := IDENT "(" (claw_param ("," claw_param)*)? ")" field_type
claw_param := field_type IDENT
            // every claw_method's params/return are mandatory -- there's
            // no body here for type inference to work from

wear       := "bare" IDENT "for" IDENT "{" function* "}"
            // == Rust's "impl <claw IDENT> for <target IDENT> { .. }";
            // each function reuses the "function" production verbatim
            // (starts with "purr"), plus an implicit `self` available in
            // its body -- never declared, same treatment "children"
            // already gets in a component
```

`wear`'s `<claw IDENT>` and `<target IDENT>` aren't cross-checked against
`claw`'s own declared method list by this grammar or by
`kittine-compiler` — a missing/extra/mismatched method surfaces as a
plain Rust trait-impl error once generated. See [LANGUAGE.md §
Claws](LANGUAGE.md#claws).

### Type tags

```
type_tag_name := ("#n" | "#w" | "#f") (("[" "]") | ("{" "}"))?
               // "[]" is an array of that scalar type; "{}" is a `stash`
               // (a String-keyed map) of it -- see LANGUAGE.md § Stashes
type_tag      := type_tag_name unary
               // wraps a *value*, e.g. `#n 0` -- distinct from
               // type_tag_name alone, which annotates a signature
               // position and carries no value
```

### Statements

```
stmt        := var_stmt | craft_stmt | if_stmt | spin_stmt
             | hold_stmt | pounce_stmt | expr_stmt

var_stmt    := "<{" IDENT "}>" ">>" expr
hold_stmt   := "hold" IDENT ">>" expr
craft_stmt  := ("craft<" | "warn<" | "error<") craft_expr ">"
            // three levels of the same statement -- see LANGUAGE.md
            // § Printing
expr_stmt   := expr

if_stmt     := "if>" condition INDENT_BLOCK
               ("orif>" condition INDENT_BLOCK)*
               ("else>" INDENT_BLOCK)?
condition   := cond_or
cond_or     := cond_and ("||" cond_and)*
cond_and    := cond_atom ("&&" cond_atom)*
cond_atom   := ("<{" IDENT "}>" cmp_op expr) | (expr cmp_op expr) | expr
cmp_op      := ">>" | "<" | "<=" | ">" | ">=" | "!="
             // ">>" here means equality, not assignment/mutation

spin_stmt   := "spin" "<{" IDENT "}>" "in" expr "}{" stmt* "}{"
             // "}{" is a plain "}" immediately followed by "{", read
             // contextually as the loop-body fence -- not a brace block

pounce_stmt := "pounce>" expr pounce_arm+ pounce_else?
pounce_arm  := IDENT ("(" IDENT ")")? ">>" stmt
pounce_else := "else>" stmt
```

`INDENT_BLOCK` is one or more `stmt`, each starting at a column greater
than the block's own base column (see [Indentation
rules](#indentation-rules)).

### Expressions

Precedence, low to high:

```
expr            := logic_or
logic_or        := logic_and ("||" logic_and)*
logic_and       := equality ("&&" equality)*
equality        := additive (cmp_op additive)?
additive        := term (("+" | "-") term)*
term            := unary (("*" | "/") unary)*
unary           := "-" unary | "&" unary | postfix
                 // "&" is a real Rust reference (Expr::Ref), an interop
                 // escape hatch -- see LANGUAGE.md § Reference operator
postfix         := primary postfix_suffix*
postfix_suffix  := "." IDENT arg_list        // method call
                 | "." IDENT                 // field read (litter)
                 | arg_list                  // calling the result of an expr
arg_list        := "(" (expr ("," expr)*)? ")"

primary := NUMBER | STRING | BOOL | array_literal | type_tag
         | call | struct_init | path | tuple_or_group
         | var_bracket_expr | pounce_expr | closure_expr
         | IDENT   // a name -- a variable, a purr call target (with
                   // arg_list via postfix), a breed unit variant, or a
                   // litter/breed name (with arg_list/"{" via postfix)

array_literal  := "[" (expr ("," expr)*)? "]"
call            := IDENT arg_list
               // a `purr` call *or* a breed variant construction --
               // told apart by which one IDENT names, via the
               // whole-import-graph Signatures map, not by syntax
struct_init    := IDENT "{" (struct_field ("," struct_field)* ","?)? "}"
struct_field   := IDENT ":" expr
               // IDENT == "stash" is a reserved exception: same grammar,
               // but lowers to a HashMap literal, not a struct -- "stash"
               // is never itself a declared litter name. See
               // LANGUAGE.md § Stashes
path           := IDENT ("::" IDENT)+
               // Type::method, Type::CONST, multi-segment paths
tuple_or_group := "(" expr ("," expr)* ","? ")"
               // a lone parenthesized expr with no comma is grouping,
               // not a 1-tuple
var_bracket_expr := "<{" IDENT "}>" (">>" expr)?
               // a bare "<{name}>" is a signal read; with ">>" it's an
               // inline assign (a mutation used as an expression value,
               // e.g. inside a JSX event handler)

pounce_expr := "pounce>" expr pounce_expr_arm+ pounce_expr_else?
               // the value-producing sibling of `pounce_stmt` -- reached
               // from `primary`, not `stmt`, so it can appear anywhere an
               // expression is expected (a return_expr, a hold_stmt's
               // value, an arg_list element, ...), not just at the start
               // of a statement. Same column-indentation rule as
               // pounce_stmt (see Indentation rules) -- the only grammar
               // difference is each arm's body is `expr`, not `stmt`
pounce_expr_arm  := IDENT ("(" IDENT ")")? ">>" expr
pounce_expr_else := "else>" expr

closure_expr := ("|" (IDENT ("," IDENT)*)? "|" | "||") expr
               // a closure literal -- lowers verbatim to a Rust closure.
               // The zero-param form shares the lexer's "||" token with
               // the logical-or operator; unambiguous in practice, since
               // "||" only reaches here (a primary/prefix position),
               // never the infix position `logic_or` consumes it in
```

`craft<...>`'s argument uses the same precedence chain, with one
difference: its `equality` step never consumes a bare trailing `>`
(craft's own closing bracket would otherwise be ambiguous with a
greater-than comparison) — wrap a `>` comparison in parens
(`craft<(age > 18)>`) to use one there.

### The view syntax (JSX)

```
jsx_node    := jsx_element | STRING | "<{" IDENT "}>" | "{" expr "}" | jsx_spin
jsx_element := "<" IDENT jsx_attr* ("/>" | ">" jsx_node* "</" IDENT ">")
jsx_attr    := attr_name "=" (STRING | "{" expr "}")
attr_name   := IDENT ("-" IDENT)*     // data-*/aria-* kebab-case names
jsx_spin    := "spin" "<{" IDENT "}>" "in" expr ("key" "(" expr ")")?
               "}{" jsx_node* "}{"
```

A tag whose first letter is uppercase is a component reference (`<Nav
.../>`); lowercase is a plain HTML element — the same rule Leptos's own
`view!` macro uses. An attribute named `on<Event>` (`onClick`, ...) becomes
a Leptos event binding instead of a plain attribute.

## What's deliberately not in this grammar

Constructs planned but not yet real — see [ROADMAP.md § Full
vision](ROADMAP.md#full-vision-phased-honest) for the full list, and
[LANGUAGE.md § Known limitations](LANGUAGE.md#known-limitations) for the
precise day-to-day boundary:

- Multiple type parameters or generic `purr`/`func` — today a
  `litter`/`breed` may have at most one type parameter (optionally
  bounded by one `claw`), and only `litter`/`breed` can be generic at
  all.
- Multi-field breed-variant payloads (`Circle(#n, #n)`) — today a variant
  carries at most one payload value.
- Type-checked cross-referencing between a `claw`'s declared methods and
  a `bare` block's implementation — today a mismatch is only caught by
  Rust's own trait-impl type checking once generated.
- Argument-type coercion for a `claw` method call — a method call's
  arguments render bare regardless of the callee's actual parameter
  types, the same as any other [method
  call](LANGUAGE.md#method-calls).

Module-level visibility is considered closed as of this document's last
update: `private`/importable-by-default now covers `func`, `purr`,
`litter`, `breed`, and `claw` uniformly, and Kittine's compilation model
(one flat Rust module tree per app) has no real crate/package boundary
for `pub(crate)`-style granularity to mean anything.
