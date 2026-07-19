//! Fills in the parameter and return types a `.kitty` author omitted (no
//! `#n`/`#w`/`#f` tag) by looking at how each name is actually used inside
//! its own function/component body — run once, as the last step of
//! [`crate::parser::parse`], so every later pass (`codegen`,
//! `collect_function_signatures`) always sees a fully-typed [`Program`],
//! exactly as if the tag had been written by hand.
//!
//! This is deliberately a *local*, single-item pass: it only looks at the
//! function/component's own body, params, and return expression/view, never
//! at other files or even other items in the same file. That keeps the
//! rules small enough to state plainly (see the `docs/LANGUAGE.md § Type
//! inference` table this module's tests mirror) instead of growing into a
//! real cross-function type checker. An explicit `#n`/`#w`/`#f` tag always
//! wins outright — inference only ever fills a gap, never overrides one.

use crate::ast::*;
use std::collections::HashMap;

/// The type inference falls back to when a param's body gives no usable
/// clue at all (e.g. it's only ever passed straight through). `Word` is
/// Kittine's "opaque value" type — the safest default for something that's
/// read but never operated on arithmetically or logically.
const DEFAULT_PARAM_TYPE: &str = "Word";
/// Same reasoning as `DEFAULT_PARAM_TYPE`, but for a `purr`'s return type:
/// kept as `Num` to match the historical silent fallback `codegen::rust_type`
/// already had for an unrecognized type string, so a `.kitty` file that
/// happened to rely on that fallback still compiles to the same Rust.
const DEFAULT_RETURN_TYPE: &str = "Num";

pub fn apply(program: &mut Program) {
    for item in &mut program.items {
        match item {
            Item::Function(f) => infer_function(f),
            Item::Component(c) => infer_component(c),
            // A `litter` field's or `breed` variant's type is always
            // explicit (`#n`/`#w`/`#f`/`#t`/a custom type name) — there's
            // no untyped-field syntax for this pass to fill in.
            Item::Litter(_) | Item::Breed(_) => {}
        }
    }
}

fn infer_function(f: &mut Function) {
    let mut resolved: HashMap<String, String> = HashMap::new();
    for p in &mut f.params {
        if p.ty.is_empty() {
            p.ty = infer_param_type(&p.name, &f.body, Some(&f.return_expr), None);
        }
        resolved.insert(p.name.clone(), p.ty.clone());
    }
    if f.return_type.is_empty() {
        f.return_type = infer_return_type(&f.return_expr, &resolved);
    }
}

fn infer_component(c: &mut Component) {
    for p in &mut c.params {
        if p.ty.is_empty() {
            p.ty = infer_param_type(&p.name, &c.body, None, c.return_view.as_ref());
        }
    }
}

/// Looks for the first usage of `name` in `body`, then `return_expr`, then
/// `return_view` (in that order — statements run before the return, so a
/// clue found there is at least as reliable) that pins down a concrete
/// type, falling back to [`DEFAULT_PARAM_TYPE`] if none of them do.
fn infer_param_type(
    name: &str,
    body: &[Stmt],
    return_expr: Option<&Expr>,
    return_view: Option<&JsxNode>,
) -> String {
    for stmt in body {
        if let Some(ty) = walk_stmt(stmt, name) {
            return ty.to_string();
        }
    }
    if let Some(e) = return_expr
        && let Some(ty) = walk_expr(e, name, false)
    {
        return ty.to_string();
    }
    if let Some(view) = return_view
        && let Some(ty) = walk_jsx(view, name)
    {
        return ty.to_string();
    }
    DEFAULT_PARAM_TYPE.to_string()
}

/// Infers a `purr`'s return type from the shape of its `return (expr)`,
/// using `param_types` (this function's own params, already resolved) to
/// resolve the common `return (paramName)` passthrough case.
fn infer_return_type(expr: &Expr, param_types: &HashMap<String, String>) -> String {
    match expr {
        Expr::Str(_) => "Word".to_string(),
        Expr::Number(_) => "Num".to_string(),
        Expr::Bool(_) => "Flag".to_string(),
        Expr::Typed { ty, .. } => ty.clone(),
        Expr::Ident(n) => param_types
            .get(n)
            .cloned()
            .unwrap_or_else(|| DEFAULT_RETURN_TYPE.to_string()),
        Expr::Binary { op, left, right } => match op {
            BinOp::Add => {
                if is_string_like(left) || is_string_like(right) {
                    "Word".to_string()
                } else if ident_type(left, param_types).as_deref() == Some("Word")
                    || ident_type(right, param_types).as_deref() == Some("Word")
                {
                    "Word".to_string()
                } else {
                    "Num".to_string()
                }
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div => "Num".to_string(),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne | BinOp::And
            | BinOp::Or => "Flag".to_string(),
        },
        _ => DEFAULT_RETURN_TYPE.to_string(),
    }
}

fn ident_type(expr: &Expr, param_types: &HashMap<String, String>) -> Option<String> {
    match expr {
        Expr::Ident(n) => param_types.get(n).cloned(),
        _ => None,
    }
}

// ---- body/expression walkers ------------------------------------------
//
// Each walker returns the first concrete type it finds evidence for, or
// `None` to keep looking. `in_bool_context` tracks whether the expression
// currently being visited must itself evaluate to `Flag` — true for an
// `if>`/`orif>` condition and for either operand of `&&`/`||` — so a bare
// `Expr::Ident(name)` appearing there (e.g. `if> ready { .. }`, or `ready
// && age >= 18`) is recognized as boolean usage even with no comparison
// operator directly touching it.

fn walk_stmt(stmt: &Stmt, name: &str) -> Option<&'static str> {
    match stmt {
        Stmt::VarAssign { value, .. } => walk_expr(value, name, false),
        Stmt::Craft { value } => walk_expr(value, name, false),
        Stmt::If {
            branches,
            else_body,
        } => {
            for (cond, body) in branches {
                if let Some(t) = walk_expr(cond, name, true) {
                    return Some(t);
                }
                for s in body {
                    if let Some(t) = walk_stmt(s, name) {
                        return Some(t);
                    }
                }
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    if let Some(t) = walk_stmt(s, name) {
                        return Some(t);
                    }
                }
            }
            None
        }
        Stmt::Expr(e) => walk_expr(e, name, false),
        Stmt::Spin { list, body, .. } => {
            if let Some(t) = walk_expr(list, name, false) {
                return Some(t);
            }
            for s in body {
                if let Some(t) = walk_stmt(s, name) {
                    return Some(t);
                }
            }
            None
        }
        Stmt::Hold { value, .. } => walk_expr(value, name, false),
        Stmt::Pounce {
            subject,
            arms,
            catch_all,
        } => {
            if let Some(t) = walk_expr(subject, name, false) {
                return Some(t);
            }
            for arm in arms {
                for s in &arm.body {
                    if let Some(t) = walk_stmt(s, name) {
                        return Some(t);
                    }
                }
            }
            if let Some(catch_all) = catch_all {
                for s in catch_all {
                    if let Some(t) = walk_stmt(s, name) {
                        return Some(t);
                    }
                }
            }
            None
        }
    }
}

fn walk_expr(expr: &Expr, name: &str, in_bool_context: bool) -> Option<&'static str> {
    match expr {
        Expr::Ident(n) if n == name && in_bool_context => Some("Flag"),
        Expr::Binary { left, op, right } => {
            if let Some(t) = infer_from_binary(*op, left, right, name) {
                return Some(t);
            }
            let child_bool = matches!(op, BinOp::And | BinOp::Or);
            walk_expr(left, name, child_bool).or_else(|| walk_expr(right, name, child_bool))
        }
        Expr::InlineAssign { value, .. } => walk_expr(value, name, false),
        Expr::Array(items) => items.iter().find_map(|e| walk_expr(e, name, false)),
        Expr::Typed { value, .. } => walk_expr(value, name, false),
        Expr::Call { args, .. } => args.iter().find_map(|e| walk_expr(e, name, false)),
        Expr::MethodCall { receiver, args, .. } => walk_expr(receiver, name, false)
            .or_else(|| args.iter().find_map(|e| walk_expr(e, name, false))),
        Expr::CallResult { callee, args } => walk_expr(callee, name, false)
            .or_else(|| args.iter().find_map(|e| walk_expr(e, name, false))),
        Expr::Tuple(items) => items.iter().find_map(|e| walk_expr(e, name, false)),
        Expr::FieldAccess { receiver, .. } => walk_expr(receiver, name, false),
        Expr::StructInit { fields, .. } => {
            fields.iter().find_map(|(_, e)| walk_expr(e, name, false))
        }
        _ => None,
    }
}

fn walk_jsx(node: &JsxNode, name: &str) -> Option<&'static str> {
    match node {
        JsxNode::Element {
            attrs, children, ..
        } => {
            for (_, v) in attrs {
                if let JsxAttrValue::Expr(e) = v
                    && let Some(t) = walk_expr(e, name, false)
                {
                    return Some(t);
                }
            }
            children.iter().find_map(|c| walk_jsx(c, name))
        }
        JsxNode::ExprInterp(e) => walk_expr(e, name, false),
        JsxNode::Spin {
            list, key, body, ..
        } => {
            if let Some(t) = walk_expr(list, name, false) {
                return Some(t);
            }
            if let Some(k) = key
                && let Some(t) = walk_expr(k, name, false)
            {
                return Some(t);
            }
            body.iter().find_map(|c| walk_jsx(c, name))
        }
        _ => None,
    }
}

/// The core rule table: given a `Binary` node whose left or right side is
/// exactly `Ident(name)`, what does the operator (plus the *other* side's
/// shape, for `+`, which is genuinely ambiguous between string-concat and
/// arithmetic) say about `name`'s type? Returns `None` if `name` isn't
/// directly one of the two operands, or (for `+`) if the other side gives
/// no clue either — the caller then keeps walking deeper.
fn infer_from_binary(op: BinOp, left: &Expr, right: &Expr, name: &str) -> Option<&'static str> {
    let other = if is_ident(left, name) {
        right
    } else if is_ident(right, name) {
        left
    } else {
        return None;
    };
    match op {
        BinOp::Add => {
            if is_string_like(other) {
                Some("Word")
            } else if is_number_like(other) {
                Some("Num")
            } else {
                None
            }
        }
        BinOp::Sub | BinOp::Mul | BinOp::Div => Some("Num"),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => Some("Num"),
        BinOp::Eq | BinOp::Ne => {
            if is_string_like(other) {
                Some("Word")
            } else if is_bool_like(other) {
                Some("Flag")
            } else if is_number_like(other) {
                Some("Num")
            } else {
                None
            }
        }
        BinOp::And | BinOp::Or => Some("Flag"),
    }
}

fn is_ident(e: &Expr, name: &str) -> bool {
    matches!(e, Expr::Ident(n) if n == name)
}

fn is_number_like(e: &Expr) -> bool {
    matches!(e, Expr::Number(_)) || matches!(e, Expr::Typed { ty, .. } if ty == "Num")
}

fn is_string_like(e: &Expr) -> bool {
    matches!(e, Expr::Str(_)) || matches!(e, Expr::Typed { ty, .. } if ty == "Word")
}

fn is_bool_like(e: &Expr) -> bool {
    matches!(e, Expr::Bool(_)) || matches!(e, Expr::Typed { ty, .. } if ty == "Flag")
}
