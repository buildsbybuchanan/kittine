//! Abstract Syntax Tree definitions for the Kittine language.

/// A full parsed `.kitty` source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub components: Vec<Component>,
}

/// A single `func Name() { ... }` component definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub name: String,
    pub body: Vec<Stmt>,
    pub return_view: Option<JsxNode>,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsxAttrValue {
    Str(String),
    Expr(Expr),
}
