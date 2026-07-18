//! Abstract Syntax Tree definitions for the Kittine language.

/// A full parsed `.kitty` source file: an optional run of `import`
/// declarations, followed by any number of components/functions.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

/// `import { Name, Name2 } from 'path/to/file.kitty'`
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub names: Vec<String>,
    pub path: String,
}

/// A top-level declaration: either a view-rendering component (`func`) or a
/// plain value-returning function (`purr`).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Component(Component),
    Function(Function),
}

/// A typed parameter in a `func`/`purr` signature: `<<Type>> name`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: String,
    pub name: String,
}

/// A single `func Name(<<Type>> prop, ..) { ... }` component definition.
/// Params become the component's props; there is no implicit state tied to
/// them (unlike `<{name}> >> value` signals).
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub return_view: Option<JsxNode>,
}

/// A single `purr name(<<Type>> param, ..) <<ReturnType>> { .. return (expr) }`
/// plain function definition — computes and returns a value, does not
/// render a view.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub body: Vec<Stmt>,
    pub return_expr: Expr,
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
    /// `<<Type>> expr` — an explicit `Num` / `Word` / `Flag` type tag
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
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
    /// `spin<{item}> in list }{ jsx_node* }{` inside a view — renders
    /// `body` once per element of `list`, binding each element to `item`.
    /// Lowers to a Leptos `<For>`, unlike the statement-position `Stmt::Spin`
    /// (which lowers to a plain imperative `for` loop).
    Spin {
        item: String,
        list: Expr,
        body: Vec<JsxNode>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsxAttrValue {
    Str(String),
    Expr(Expr),
}
