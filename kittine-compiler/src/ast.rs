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

/// A top-level declaration: either a view-rendering component (`func`) or a
/// plain value-returning function (`purr`).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Component(Component),
    Function(Function),
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
