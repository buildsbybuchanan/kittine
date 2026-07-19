//! Abstract Syntax Tree definitions for the Kittine language.

/// A full parsed `.kitty` source file: an optional run of `import`
/// declarations, followed by any number of components/functions.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

/// `(export)? import { Name, Name2 } from 'path/to/file.kitty'`
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub names: Vec<String>,
    pub path: String,
    /// `true` for `export import ..` — re-exports `names` under this
    /// file's own name, so a *third* file can `import` them straight from
    /// here instead of reaching all the way back to where they're
    /// actually defined. Lowers to `pub use` instead of a plain `use`.
    pub is_export: bool,
}

/// A top-level declaration: a view-rendering component (`func`), a plain
/// value-returning function (`purr`), a data record (`litter`), a closed
/// variant type (`breed`), a trait (`claw`), or a `claw` implementation
/// for a `litter`/`breed` (`bare .. for ..`).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Component(Component),
    Function(Function),
    Litter(Litter),
    Breed(Breed),
    Claw(Claw),
    Wear(Wear),
}

/// `"<" "#t" (":" IDENT)? ">"` — a `litter`/`breed`'s own generic type
/// parameter declaration: minimal groundwork (at most one parameter, no
/// multiple params), but a real bound (`bound: Some(claw_name)`) is
/// allowed, checked by Rust's own trait system once generated (Kittine
/// doesn't re-verify it). See `docs/LANGUAGE.md` § Generics.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub bound: Option<String>,
}

/// `(private)? litter Name type_param? "{" field ("," field)* ","? "}"`
/// — a plain data record, Kittine's term for what Rust calls a struct.
/// Fields are declared the same `name type` shape as a function
/// [`Param`], just comma-separated instead of parenthesized.
#[derive(Debug, Clone, PartialEq)]
pub struct Litter {
    pub name: String,
    pub type_param: Option<TypeParam>,
    pub fields: Vec<LitterField>,
    /// `true` for `private litter ..` — see `Component::is_private`.
    pub is_private: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LitterField {
    pub name: String,
    /// `Num`/`Word`/`Flag`, an array form, `"T"` (this litter's own type
    /// parameter — only valid when `type_param` is `Some`), or another
    /// `litter`/`breed`'s name (a nested custom type, rendered verbatim
    /// as the Rust type name).
    pub ty: String,
}

/// `(private)? breed Name type_param? "{" variant ("," variant)* ","? "}"`
/// — a closed set of named variants, Kittine's term for what Rust calls
/// an enum. A variant carries at most one payload value (same minimal-
/// groundwork scope as `Litter`'s single type parameter).
#[derive(Debug, Clone, PartialEq)]
pub struct Breed {
    pub name: String,
    pub type_param: Option<TypeParam>,
    pub variants: Vec<BreedVariant>,
    pub is_private: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BreedVariant {
    pub name: String,
    /// `Some(ty)` for `Variant(ty)`, `None` for a bare unit variant.
    pub payload: Option<String>,
}

/// `(private)? claw Name "{" claw_method ("," claw_method)* ","? "}"` — a
/// trait: a named set of method signatures a `litter`/`breed` can promise
/// to implement (see [`Wear`]). Every signature is fully explicit (no
/// inference — there's no body here for `infer::apply` to look at, and a
/// trait's signature has to be fixed independent of any one
/// implementation).
#[derive(Debug, Clone, PartialEq)]
pub struct Claw {
    pub name: String,
    pub methods: Vec<ClawMethod>,
    /// `true` for `private claw ..` — see `Component::is_private`.
    pub is_private: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClawMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
}

/// `bare ClawName "for" TypeName "{" ("purr" method)* "}"` — implements
/// `claw` for `litter`/`breed` `target`, Kittine's term for what Rust
/// calls `impl Claw for Target`. Each method reuses the ordinary `purr`
/// grammar/AST (`Function`) verbatim; `self` is available inside a
/// method's body with no declaration needed (see
/// `codegen::method_param_list_rust`), the same way `children` needs
/// none in a component. There's no cross-check here that `methods`
/// actually matches `claw`'s own signatures — a mismatch is caught by
/// Rust's own trait-impl type checking once generated, the same trust
/// model an unknown method call already gets (see `Expr::MethodCall`).
#[derive(Debug, Clone, PartialEq)]
pub struct Wear {
    pub claw: String,
    pub target: String,
    pub methods: Vec<Function>,
}

/// A typed parameter in a `func`/`purr` signature: `#t name`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: String,
    pub name: String,
}

/// A single `func Name(#t prop, ..) { ... }` component definition.
/// Params become the component's props; there is no implicit state tied to
/// them (unlike `<{name}> >> value` signals).
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub return_view: Option<JsxNode>,
    /// `true` for `private func ..` — emitted as a plain (non-`pub`) Rust
    /// `fn`, so `import`ing it from another file is a Rust-enforced
    /// compile error (E0603), not something Kittine has to check itself.
    pub is_private: bool,
}

/// A single `purr name(#t param, ..) #t { .. return (expr) }`
/// plain function definition — computes and returns a value, does not
/// render a view.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub body: Vec<Stmt>,
    pub return_expr: Expr,
    /// `true` for `private purr ..` — see `Component::is_private`.
    pub is_private: bool,
}

/// A statement inside a component body or an `if>`/`orif>`/`else>` block.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `<{name}> >> expr`
    ///
    /// The code generator decides whether this lowers to a signal
    /// declaration (`let (x, set_x) = signal(..)`) or a mutation
    /// (`set_x.update(..)`) based on whether `name` has already been
    /// declared in the current component.
    VarAssign { name: String, value: Expr },
    /// `craft<expr>` — a console/log statement.
    Craft { value: Expr },
    /// `if> cond { .. } orif> cond { .. } else> { .. }`
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
    /// A bare expression statement (rare, but kept for completeness).
    Expr(Expr),
    /// `spin<{item}> in list }{ .. }{` — iterates `list`, binding each
    /// element to `item` for the duration of `body`.
    Spin {
        item: String,
        list: Expr,
        body: Vec<Stmt>,
    },
    /// `hold name >> expr` — a plain (non-reactive) local binding, unlike
    /// `<{name}> >> value`'s signal. Lowers to a bare `let name = expr;`,
    /// evaluated once, at that point. For forcing a Leptos hook
    /// (`use_navigate()`, ...) to run eagerly at component setup instead of
    /// lazily inside an event handler — see `docs/LANGUAGE.md` §
    /// Programmatic navigation.
    Hold { name: String, value: Expr },
    /// `pounce> subject` `(Variant(binding)? ">>" stmt)*` `(else> stmt)?`
    /// — pattern-matches `subject` (a `breed` value) against each arm's
    /// variant in turn, arms indented one level under `pounce>` itself
    /// (not siblings of it, unlike `orif>`/`else>` beside `if>`). Each
    /// arm's body is exactly one statement. Statement-only: there's no
    /// way yet to use a `pounce>`'s result as a value (see
    /// `docs/LANGUAGE.md` § Known limitations) — this is for branching on
    /// which variant a value is and taking action, not computing one.
    Pounce {
        subject: Expr,
        arms: Vec<PounceArm>,
        catch_all: Option<Vec<Stmt>>,
    },
}

/// One arm of a [`Stmt::Pounce`]: `Variant(binding)? >> stmt`. `binding` is
/// `Some` only when the variant carries a payload (`Circle(r)`) — a unit
/// variant's arm (`Idle >> ..`) has none. `body` always holds exactly one
/// statement (see `Parser::parse_pounce_stmt`), but stays a `Vec` to reuse
/// `gen_stmt`'s existing per-statement codegen unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct PounceArm {
    pub variant: String,
    pub binding: Option<String>,
    pub body: Vec<Stmt>,
}

/// An expression in Kittine source.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(String),
    Number(f64),
    Str(String),
    /// `<{name}>` used as a value (e.g. inside JSX or another expression).
    VarRead(String),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    /// `<{name}> >> expr` used inline as an expression, e.g. inside a JSX
    /// event handler: `onClick={<{count}> >> count + 1}`.
    InlineAssign {
        name: String,
        value: Box<Expr>,
    },
    /// `yes>` / `no>` boolean literal.
    Bool(bool),
    /// `[expr, expr, ..]` array literal.
    Array(Vec<Expr>),
    /// `#t expr` — an explicit `Num` / `Word` / `Flag` type tag
    /// wrapping a value. Checked against literal values at parse time;
    /// erased (the inner value is emitted as-is) at codegen time.
    Typed {
        ty: String,
        value: Box<Expr>,
    },
    /// `name(arg, arg, ..)` — a call to a `purr` function (or, in principle,
    /// any function in scope, e.g. one brought in via `import`).
    Call {
        name: String,
        args: Vec<Expr>,
    },
    /// `receiver.method(arg, arg, ..)` — a method call on an arbitrary
    /// expression, for interop with real Rust/Leptos APIs that aren't a
    /// Kittine `purr` (e.g. `use_params_map().get()`). Kittine doesn't
    /// track receiver types, so this renders verbatim and lets Rust's own
    /// type checker validate it — same trust model as `Expr::Call` for an
    /// unknown function name.
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    /// `callee(arg, arg, ..)` where `callee` is itself an arbitrary
    /// expression rather than a bare name — e.g. calling the closure
    /// `use_navigate()` returns immediately: `use_navigate()('/', ..)`.
    /// Distinct from `Expr::Call` (a *named* function/`purr`), which keeps
    /// its own same-file signature lookup (see `codegen::render_call`)
    /// unaffected by this more general, untyped form.
    CallResult {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// `(expr, expr, ..)` — a tuple literal, needed to combine multiple
    /// `leptos_router` path segments into one route (`(StaticSegment('user'),
    /// ParamSegment('id'))`). A single parenthesized expression with no
    /// comma is just grouping, not a 1-tuple — see `parser::parse_primary`.
    Tuple(Vec<Expr>),
    /// `Segment::Segment(::Segment)*` — a path-qualified expression, e.g.
    /// `NavigateOptions::default` or `std::cmp::max`. Renders as-is
    /// (`segments.join("::")`); combines with the existing postfix chain
    /// (`Expr::CallResult`, `Expr::MethodCall`) to express
    /// `Type::method(args)` and `Type::CONST` without any dedicated call
    /// syntax of its own.
    Path(Vec<String>),
    /// `receiver.field` — reads a [`Litter`] field. Unlike `Expr::MethodCall`
    /// (which always has an `arg_list`, even an empty `()`), this has no
    /// parens at all — that's what distinguishes a field read from a
    /// zero-argument method call at parse time.
    FieldAccess {
        receiver: Box<Expr>,
        field: String,
    },
    /// `Name { field: expr, .. }` — a [`Litter`] struct literal. For a
    /// generic litter (`Litter::has_type_param`), there's no explicit
    /// turbofish-style instantiation — Rust infers the type parameter from
    /// the field values themselves, the same way it infers any other
    /// generic constructor call.
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
    Ne,
    And,
    Or,
}

/// A node in the JSX-like `return ( ... )` tree.
#[derive(Debug, Clone, PartialEq)]
pub enum JsxNode {
    Element {
        tag: String,
        attrs: Vec<(String, JsxAttrValue)>,
        children: Vec<JsxNode>,
        self_closing: bool,
    },
    Text(String),
    /// `<{name}>` interpolated directly into JSX children.
    VarInterp(String),
    /// `{ expr }` interpolated directly into JSX children.
    ExprInterp(Expr),
    /// `spin<{item}> in list (key(expr))? }{ jsx_node* }{` inside a view —
    /// renders `body` once per element of `list`, binding each element to
    /// `item`. Lowers to a Leptos `<For>`, unlike the statement-position
    /// `Stmt::Spin` (which lowers to a plain imperative `for` loop). `key`
    /// is optional; `item` is in scope while evaluating it, same as in
    /// `body`. Defaults to keying by `format!("{item}")` when omitted.
    Spin {
        item: String,
        list: Expr,
        key: Option<Expr>,
        body: Vec<JsxNode>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsxAttrValue {
    Str(String),
    Expr(Expr),
}
