//! A canonical pretty-printer for `.kitty` source, driving
//! `kittine-compiler fmt`.
//!
//! This is an AST -> text printer, not a whitespace-preserving formatter:
//! `kittine-compiler`'s lexer discards `//` comments before the parser ever
//! sees them (see `lexer::skip_whitespace_and_comments`), and `ast::Stmt`/
//! `ast::Expr` carry no source position at all, so there is nothing here to
//! preserve blank-line grouping or comments from. Given that, this module
//! defines its own fixed canonical style (4-space indent, one blank line
//! between top-level items, a blank line bracketing `if>`/`spin`/`pounce>`
//! blocks) rather than pretending to reproduce arbitrary hand-formatting.
//!
//! The safety net that makes this trustworthy despite hand-writing a whole
//! printer: [`format_and_verify`] reparses its own output and compares the
//! resulting `ast::Program` against the original via `PartialEq` before
//! ever returning it — a real, mechanical check, not an assumption. A
//! mismatch is treated as an internal formatter bug and refused, never
//! silently written.

use crate::ast::*;
use crate::lexer;
use crate::parser;

const INDENT: &str = "    ";

fn ind(level: usize) -> String {
    INDENT.repeat(level)
}

// ---- precedence tiers, matching parser.rs's actual grammar precedence
// (see the module doc comment on `Parser` in parser.rs) ----------------

const P_TOP: u8 = 0; // a full `parse_expr()` position
const P_OR: u8 = 1;
const P_AND: u8 = 2;
const P_CMP: u8 = 3;
const P_ADD: u8 = 4; // also the ceiling for `parse_additive()`-only positions
const P_MUL: u8 = 5;
const P_UNARY: u8 = 6; // also the ceiling for `parse_unary()`-only positions
const P_POSTFIX: u8 = 7; // a postfix-chain receiver/callee position

fn bin_prec(op: BinOp) -> u8 {
    match op {
        BinOp::Or => P_OR,
        BinOp::And => P_AND,
        BinOp::Eq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge | BinOp::Ne => P_CMP,
        BinOp::Add | BinOp::Sub => P_ADD,
        BinOp::Mul | BinOp::Div => P_MUL,
    }
}

fn op_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Eq => ">>",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::Ne => "!=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

fn is_zero_literal(e: &Expr) -> bool {
    matches!(e, Expr::Number(n) if *n == 0.0)
}

/// Prints `e`, adding parens only when `e`'s own precedence is lower than
/// `min_prec` demands — i.e. only when omitting them would change how the
/// output reparses. Every non-`Binary` node is treated as maximal
/// precedence (primary/postfix, per the grammar), so it's never wrapped.
fn print_expr(e: &Expr, min_prec: u8) -> String {
    match e {
        Expr::Binary { left, op, right } => {
            // `parse_unary` desugars `-x` into `Binary{0, Sub, x}` — there is
            // no separate `Neg` node, and no way to tell `-5` from `0 - 5`
            // apart in the AST at all (both parse to the identical tree).
            // Printing the unary form is just the nicer of two equally
            // correct choices.
            if *op == BinOp::Sub && is_zero_literal(left) {
                let inner = format!("-{}", print_expr(right, P_UNARY));
                return wrap_if(inner, P_UNARY, min_prec);
            }
            let my_prec = bin_prec(*op);
            let l = print_expr(left, my_prec);
            let r = print_expr(right, my_prec + 1);
            let inner = format!("{l} {} {r}", op_str(*op));
            wrap_if(inner, my_prec, min_prec)
        }
        Expr::Ident(s) => s.clone(),
        Expr::Number(n) => format!("{n}"),
        Expr::Str(s) => quote_string(s),
        Expr::VarRead(name) => format!("<{{{name}}}>"),
        // `parse_var_bracket_expr` parses this value via `parse_additive`,
        // not the full expression grammar — a bare `&&`/`||`/comparison
        // here needs parens to round-trip.
        Expr::InlineAssign { name, value } => {
            format!("<{{{name}}}> >> {}", print_expr(value, P_ADD))
        }
        Expr::Bool(b) => (if *b { "yes>" } else { "no>" }).to_string(),
        Expr::Array(items) => format!("[{}]", join_exprs(items, P_TOP)),
        // `parse_type_tag` reads its value via `parse_unary`.
        Expr::Typed { ty, value } => format!("{} {}", sig_type_str(ty), print_expr(value, P_UNARY)),
        Expr::Call { name, args } => format!("{name}({})", join_exprs(args, P_TOP)),
        Expr::MethodCall {
            receiver,
            method,
            args,
        } => format!(
            "{}.{method}({})",
            print_expr(receiver, P_POSTFIX),
            join_exprs(args, P_TOP)
        ),
        Expr::CallResult { callee, args } => {
            format!("{}({})", print_expr(callee, P_POSTFIX), join_exprs(args, P_TOP))
        }
        Expr::Tuple(items) => {
            if items.len() == 1 {
                // A single parenthesized expr with no comma is *grouping*,
                // not a tuple (see parser.rs's `parse_primary`) — a trailing
                // comma is what disambiguates a real one-element tuple.
                format!("({},)", print_expr(&items[0], P_TOP))
            } else {
                format!("({})", join_exprs(items, P_TOP))
            }
        }
        Expr::Path(segments) => segments.join("::"),
        Expr::FieldAccess { receiver, field } => {
            format!("{}.{field}", print_expr(receiver, P_POSTFIX))
        }
        Expr::StructInit { name, fields } => {
            if fields.is_empty() {
                format!("{name} {{}}")
            } else {
                let body = fields
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", print_expr(v, P_TOP)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name} {{ {body} }}")
            }
        }
    }
}

fn wrap_if(inner: String, my_prec: u8, min_prec: u8) -> String {
    if my_prec < min_prec {
        format!("({inner})")
    } else {
        inner
    }
}

fn join_exprs(items: &[Expr], min_prec: u8) -> String {
    items
        .iter()
        .map(|e| print_expr(e, min_prec))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `craft<expr>`'s argument is parsed with a bare trailing `>` disallowed
/// (it would be ambiguous with craft's own closing `>` — see
/// `Parser::parse_equality_no_trailing_gt`), *anywhere* it would be
/// reachable without hitting parens first — i.e. through an `&&`/`||` chain
/// too, not just at the very root. This walks exactly that reachable set.
fn contains_bare_gt(e: &Expr) -> bool {
    match e {
        Expr::Binary { left, op, right } => match op {
            BinOp::Or | BinOp::And => contains_bare_gt(left) || contains_bare_gt(right),
            BinOp::Gt => true,
            _ => false,
        },
        _ => false,
    }
}

fn escape_for_quote(s: &str, quote: char) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

/// Kittine string literals are quote-interchangeable (`'...'` and `"..."`),
/// so the formatter picks a canonical quote per string: single quotes
/// unless the content has one (then double), falling back to single quotes
/// with `\'` escaping if it has both.
fn quote_string(s: &str) -> String {
    if !s.contains('\'') {
        format!("'{}'", escape_for_quote(s, '\''))
    } else if !s.contains('"') {
        format!("\"{}\"", escape_for_quote(s, '"'))
    } else {
        format!("'{}'", escape_for_quote(s, '\''))
    }
}

/// Maps a param/return-type-position type string (as `parse_signature_type`
/// produces it: `Num`/`Word`/`Flag`, optionally `[]`-suffixed) back to its
/// sigil. Falls back to the input verbatim for anything else (defensive —
/// this position never holds a custom type name in practice).
fn sig_type_str(ty: &str) -> String {
    match ty {
        "Num" => "#n".to_string(),
        "Word" => "#w".to_string(),
        "Flag" => "#f".to_string(),
        "Num[]" => "#n[]".to_string(),
        "Word[]" => "#w[]".to_string(),
        "Flag[]" => "#f[]".to_string(),
        other => other.to_string(),
    }
}

/// Like [`sig_type_str`], but also covers `field_type`'s wider grammar:
/// `#t` (a litter/breed's own generic placeholder, stored as `"T"`) and a
/// custom litter/breed name (stored verbatim already).
fn field_type_str(ty: &str) -> String {
    if ty == "T" {
        "#t".to_string()
    } else {
        sig_type_str(ty)
    }
}

fn print_param(p: &Param) -> String {
    if p.ty.is_empty() || p.ty == "Children" {
        p.name.clone()
    } else {
        format!("{} {}", sig_type_str(&p.ty), p.name)
    }
}

fn print_params(params: &[Param]) -> String {
    params.iter().map(print_param).collect::<Vec<_>>().join(", ")
}

fn print_claw_param(p: &Param) -> String {
    format!("{} {}", field_type_str(&p.ty), p.name)
}

fn return_type_suffix(ty: &str) -> String {
    if ty.is_empty() {
        String::new()
    } else {
        format!(" {}", sig_type_str(ty))
    }
}

fn print_type_param(tp: &Option<TypeParam>) -> String {
    match tp {
        None => String::new(),
        Some(TypeParam { bound: None }) => "<#t>".to_string(),
        Some(TypeParam { bound: Some(b) }) => format!("<#t: {b}>"),
    }
}

// ---- statements ---------------------------------------------------------

/// A statement is "block-shaped" (spans multiple lines, reads like its own
/// paragraph) if it's `If`/`Spin`/`Pounce`. Used to decide where the
/// canonical style inserts a blank line — see the module doc comment.
fn is_block_stmt(s: &Stmt) -> bool {
    matches!(s, Stmt::If { .. } | Stmt::Spin { .. } | Stmt::Pounce { .. })
}

fn print_stmts(stmts: &[Stmt], level: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut prev_was_block = false;
    for (i, s) in stmts.iter().enumerate() {
        let this_is_block = is_block_stmt(s);
        if i > 0 && (this_is_block || prev_was_block) {
            lines.push(String::new());
        }
        lines.extend(print_stmt(s, level));
        prev_was_block = this_is_block;
    }
    lines
}

fn print_stmt(s: &Stmt, level: usize) -> Vec<String> {
    match s {
        Stmt::VarAssign { name, value } => {
            vec![format!("{}<{{{name}}}> >> {}", ind(level), print_expr(value, P_ADD))]
        }
        Stmt::Craft { value } => {
            let inner = if contains_bare_gt(value) {
                format!("({})", print_expr(value, P_TOP))
            } else {
                print_expr(value, P_TOP)
            };
            vec![format!("{}craft<{inner}>", ind(level))]
        }
        Stmt::Hold { name, value } => {
            vec![format!("{}hold {name} >> {}", ind(level), print_expr(value, P_TOP))]
        }
        Stmt::Expr(e) => vec![format!("{}{}", ind(level), print_expr(e, P_TOP))],
        Stmt::If {
            branches,
            else_body,
        } => {
            let mut lines = Vec::new();
            for (i, (cond, body)) in branches.iter().enumerate() {
                let kw = if i == 0 { "if>" } else { "orif>" };
                lines.push(format!("{}{kw}{}", ind(level), print_expr(cond, P_TOP)));
                lines.extend(print_stmts(body, level + 1));
            }
            if let Some(eb) = else_body {
                lines.push(format!("{}else>", ind(level)));
                lines.extend(print_stmts(eb, level + 1));
            }
            lines
        }
        Stmt::Spin { item, list, body } => {
            let mut lines = vec![format!(
                "{}spin<{{{item}}}> in {} }}{{",
                ind(level),
                print_expr(list, P_TOP)
            )];
            lines.extend(print_stmts(body, level + 1));
            lines.push(format!("{}}}{{", ind(level)));
            lines
        }
        Stmt::Pounce {
            subject,
            arms,
            catch_all,
        } => {
            let mut lines = vec![format!("{}pounce> {}", ind(level), print_expr(subject, P_TOP))];
            for arm in arms {
                let mut header = String::new();
                header.push_str(&arm.variant);
                if let Some(b) = &arm.binding {
                    header.push('(');
                    header.push_str(b);
                    header.push(')');
                }
                header.push_str(" >> ");
                lines.push(print_arm_body(&header, &arm.body[0], level));
            }
            if let Some(ca) = catch_all {
                lines.push(print_arm_body("else> ", &ca[0], level));
            }
            lines
        }
    }
}

/// Prints a `pounce>` arm's (or `else>` catch-all's) single statement,
/// inlined after `header` on the arm's own line. `header` already excludes
/// indentation; the inner statement is printed one level deeper than the
/// arm itself so a multi-line inner statement's continuation lines (rare —
/// no arm in the whole codebase needs one today) still satisfy the
/// off-side indentation rule, even though they won't align under `header`'s
/// own text the way a hand-written file might.
fn print_arm_body(header: &str, inner: &Stmt, level: usize) -> String {
    let inner_lines = print_stmt(inner, level + 2);
    let mut out = format!("{}{header}{}", ind(level + 1), inner_lines[0].trim_start());
    for extra in &inner_lines[1..] {
        out.push('\n');
        out.push_str(extra);
    }
    out
}

// ---- JSX ------------------------------------------------------------------

fn print_jsx_attrs(attrs: &[(String, JsxAttrValue)]) -> String {
    let mut out = String::new();
    for (name, value) in attrs {
        out.push(' ');
        out.push_str(name);
        out.push('=');
        match value {
            JsxAttrValue::Str(s) => out.push_str(&quote_string(s)),
            JsxAttrValue::Expr(e) => {
                out.push('{');
                out.push_str(&print_expr(e, P_TOP));
                out.push('}');
            }
        }
    }
    out
}

fn jsx_children_all_simple(children: &[JsxNode]) -> bool {
    children
        .iter()
        .all(|c| matches!(c, JsxNode::Text(_) | JsxNode::VarInterp(_) | JsxNode::ExprInterp(_)))
}

fn print_jsx_inline(node: &JsxNode) -> String {
    match node {
        JsxNode::Text(s) => quote_string(s),
        JsxNode::VarInterp(name) => format!("<{{{name}}}>"),
        JsxNode::ExprInterp(e) => format!("{{ {} }}", print_expr(e, P_TOP)),
        // Unreachable from `jsx_children_all_simple`-gated call sites.
        other => print_jsx_node(other, 0).join("\n"),
    }
}

fn print_jsx_node(node: &JsxNode, level: usize) -> Vec<String> {
    match node {
        JsxNode::Element {
            tag,
            attrs,
            children,
            self_closing,
        } => {
            let open = format!("<{tag}{}{}", print_jsx_attrs(attrs), if *self_closing { "/>" } else { ">" });
            if *self_closing {
                vec![format!("{}{open}", ind(level))]
            } else if children.is_empty() {
                vec![format!("{}{open}</{tag}>", ind(level))]
            } else if children.len() <= 2 && jsx_children_all_simple(children) {
                let inline = children.iter().map(print_jsx_inline).collect::<Vec<_>>().join(" ");
                vec![format!("{}{open}{inline}</{tag}>", ind(level))]
            } else {
                let mut lines = vec![format!("{}{open}", ind(level))];
                for c in children {
                    lines.extend(print_jsx_node(c, level + 1));
                }
                lines.push(format!("{}</{tag}>", ind(level)));
                lines
            }
        }
        JsxNode::Text(s) => vec![format!("{}{}", ind(level), quote_string(s))],
        JsxNode::VarInterp(name) => vec![format!("{}<{{{name}}}>", ind(level))],
        JsxNode::ExprInterp(e) => vec![format!("{}{{ {} }}", ind(level), print_expr(e, P_TOP))],
        JsxNode::Spin {
            item,
            list,
            key,
            body,
        } => {
            let mut header = format!("spin<{{{item}}}> in {}", print_expr(list, P_TOP));
            if let Some(k) = key {
                header.push_str(&format!(" key({})", print_expr(k, P_TOP)));
            }
            header.push_str(" }{");
            let mut lines = vec![format!("{}{header}", ind(level))];
            for c in body {
                lines.extend(print_jsx_node(c, level + 1));
            }
            lines.push(format!("{}}}{{", ind(level)));
            lines
        }
    }
}

// ---- top-level items --------------------------------------------------

fn print_import(imp: &Import) -> String {
    let prefix = if imp.is_export { "export " } else { "" };
    format!(
        "{prefix}import {{ {} }} from {}",
        imp.names.join(", "),
        quote_string(&imp.path)
    )
}

fn print_component(c: &Component) -> String {
    let mut out = String::new();
    if c.is_private {
        out.push_str("private ");
    }
    out.push_str(&format!("func {}({}) {{\n", c.name, print_params(&c.params)));
    let body_lines = print_stmts(&c.body, 1);
    for l in &body_lines {
        out.push_str(l);
        out.push('\n');
    }
    if let Some(view) = &c.return_view {
        if !body_lines.is_empty() {
            out.push('\n');
        }
        out.push_str(&ind(1));
        out.push_str("return (\n");
        for l in print_jsx_node(view, 2) {
            out.push_str(&l);
            out.push('\n');
        }
        out.push_str(&ind(1));
        out.push_str(")\n");
    }
    out.push('}');
    out
}

fn print_function(f: &Function, level: usize) -> String {
    let base = ind(level);
    let mut out = base.clone();
    if f.is_private {
        out.push_str("private ");
    }
    out.push_str(&format!(
        "purr {}({}){} {{\n",
        f.name,
        print_params(&f.params),
        return_type_suffix(&f.return_type)
    ));
    let body_lines = print_stmts(&f.body, level + 1);
    for l in &body_lines {
        out.push_str(l);
        out.push('\n');
    }
    if !body_lines.is_empty() {
        out.push('\n');
    }
    out.push_str(&ind(level + 1));
    out.push_str(&format!("return ({})\n", print_expr(&f.return_expr, P_TOP)));
    out.push_str(&base);
    out.push('}');
    out
}

fn print_litter(l: &Litter) -> String {
    let mut out = String::new();
    if l.is_private {
        out.push_str("private ");
    }
    out.push_str(&format!("litter {}{} {{\n", l.name, print_type_param(&l.type_param)));
    for (i, f) in l.fields.iter().enumerate() {
        out.push_str(&ind(1));
        out.push_str(&f.name);
        out.push(' ');
        out.push_str(&field_type_str(&f.ty));
        if i + 1 < l.fields.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push('}');
    out
}

fn print_breed(b: &Breed) -> String {
    let mut out = String::new();
    if b.is_private {
        out.push_str("private ");
    }
    out.push_str(&format!("breed {}{} {{\n", b.name, print_type_param(&b.type_param)));
    for (i, v) in b.variants.iter().enumerate() {
        out.push_str(&ind(1));
        out.push_str(&v.name);
        if let Some(payload) = &v.payload {
            out.push('(');
            out.push_str(&field_type_str(payload));
            out.push(')');
        }
        if i + 1 < b.variants.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push('}');
    out
}

fn print_claw(c: &Claw) -> String {
    let mut out = String::new();
    if c.is_private {
        out.push_str("private ");
    }
    out.push_str(&format!("claw {} {{\n", c.name));
    for (i, m) in c.methods.iter().enumerate() {
        let params = m.params.iter().map(print_claw_param).collect::<Vec<_>>().join(", ");
        out.push_str(&ind(1));
        out.push_str(&format!("{}({params}) {}", m.name, field_type_str(&m.return_type)));
        if i + 1 < c.methods.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push('}');
    out
}

fn print_wear(w: &Wear) -> String {
    let mut out = format!("bare {} for {} {{\n", w.claw, w.target);
    for (i, m) in w.methods.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&print_function(m, 1));
        out.push('\n');
    }
    out.push('}');
    out
}

fn print_item(item: &Item) -> String {
    match item {
        Item::Component(c) => print_component(c),
        Item::Function(f) => print_function(f, 0),
        Item::Litter(l) => print_litter(l),
        Item::Breed(b) => print_breed(b),
        Item::Claw(c) => print_claw(c),
        Item::Wear(w) => print_wear(w),
    }
}

/// Pretty-prints a whole parsed program in the canonical style. Pure — does
/// not reparse or verify anything; see [`format_and_verify`] for the
/// checked entry point every real caller (the `fmt` CLI command) should
/// use instead.
pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    for imp in &program.imports {
        out.push_str(&print_import(imp));
        out.push('\n');
    }
    if !program.imports.is_empty() && !program.items.is_empty() {
        out.push('\n');
    }
    for (i, item) in program.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&print_item(item));
        out.push('\n');
    }
    out
}

/// `true` if `source` contains a `// ..` line comment anywhere outside a
/// string literal. `kittine-compiler`'s lexer discards comments before the
/// parser ever sees them (see the module doc comment), so [`format_program`]
/// has no way to preserve one — callers use this to warn/refuse rather than
/// silently deleting a comment a real file happens to have.
pub fn has_line_comments(source: &str) -> bool {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut in_string: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = in_string {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == q {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if c == '\'' || c == '"' {
            in_string = Some(c);
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            return true;
        }
        i += 1;
    }
    false
}

/// The real entry point: formats `source`, then reparses its own output and
/// refuses to return it unless the resulting [`Program`] is exactly equal
/// (via `PartialEq`, recursively over the whole AST) to the one parsed from
/// `source` itself. This is what makes trusting a hand-written printer
/// reasonable — every call is self-verifying, not just tested in CI.
pub fn format_and_verify(source: &str) -> Result<String, String> {
    // Deliberately `Parser::parse_program`, not `parser::parse` -- the
    // latter also runs `infer::apply`, which would materialize omitted
    // type tags into the AST before the printer ever sees it, silently
    // adding tags the user chose to leave out. Both the initial parse and
    // the round-trip check below stay on this same raw path.
    let tokens = lexer::tokenize(source).map_err(|e| e.to_string())?;
    let program = parser::Parser::new(tokens)
        .parse_program()
        .map_err(|e| e.to_string())?;
    let formatted = format_program(&program);

    let tokens2 = lexer::tokenize(&formatted)
        .map_err(|e| format!("internal formatter error: reformatted output failed to lex: {e}"))?;
    let program2 = parser::Parser::new(tokens2)
        .parse_program()
        .map_err(|e| format!("internal formatter error: reformatted output failed to reparse: {e}"))?;

    if program != program2 {
        return Err(
            "internal formatter error: reformatted output does not parse back to an \
             equivalent program — refusing to write (this is a formatter bug, not a \
             property of your source; please report it)"
                .to_string(),
        );
    }
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Formats `source`, asserts it round-trips (equal AST), and returns the
    /// formatted text for further assertions.
    fn fmt_ok(source: &str) -> String {
        format_and_verify(source).unwrap_or_else(|e| panic!("format_and_verify failed: {e}"))
    }

    fn assert_idempotent(source: &str) {
        let once = fmt_ok(source);
        let twice = fmt_ok(&once);
        assert_eq!(once, twice, "fmt is not idempotent");
    }

    #[test]
    fn formats_a_simple_function() {
        let src = "purr   isAdult(age)   {\nreturn(age>=18)\n}\n";
        let out = fmt_ok(src);
        assert_eq!(out, "purr isAdult(age) {\n    return (age >= 18)\n}\n");
        assert_idempotent(src);
    }

    #[test]
    fn preserves_omitted_type_tags() {
        // No `#w`/`#n` tags written -- fmt must not materialize inferred
        // ones; that's `infer::apply`'s job, deliberately not run here.
        let src = "purr greet(name) {\n    return ('Hello, ' + name)\n}\n";
        let out = fmt_ok(src);
        assert!(!out.contains('#'), "fmt must not insert inferred type tags: {out}");
        assert_idempotent(src);
    }

    #[test]
    fn quote_normalization_round_trips() {
        let src = "purr f() #w { return (\"it's fine\") }";
        let out = fmt_ok(src);
        assert!(out.contains("it's fine"));
        assert_idempotent(src);
    }

    #[test]
    fn craft_gt_gets_wrapped_in_parens() {
        let src = "func F() {\n    craft<(age > 18)>\n}\n";
        let out = fmt_ok(src);
        assert!(out.contains("craft<(age > 18)>"), "got: {out}");
        assert_idempotent(src);
    }

    #[test]
    fn craft_gt_inside_and_chain_still_wrapped() {
        let src = "func F() {\n    craft<(age > 18 && ready)>\n}\n";
        fmt_ok(src); // would fail format_and_verify's safety check if unwrapped
        assert_idempotent(src);
    }

    #[test]
    fn preserves_operator_precedence() {
        let src = "purr f() #n { return ((a + b) * c) }";
        let out = fmt_ok(src);
        assert!(out.contains("(a + b) * c"), "got: {out}");
        assert_idempotent(src);
    }

    #[test]
    fn right_associative_grouping_round_trips() {
        // `a - (b - c)` is NOT the same tree as `a - b - c` -- the printer
        // must keep the parens or the safety check would reject this.
        let src = "purr f() #n { return (a - (b - c)) }";
        let out = fmt_ok(src);
        assert!(out.contains("a - (b - c)"), "got: {out}");
        assert_idempotent(src);
    }

    #[test]
    fn single_element_tuple_keeps_trailing_comma() {
        let src = "func F() {\n    <{x}> >> (1,)\n}\n";
        let out = fmt_ok(src);
        assert!(out.contains("(1,)"), "got: {out}");
        assert_idempotent(src);
    }

    #[test]
    fn unary_minus_round_trips() {
        let src = "purr f() #n { return (-5) }";
        let out = fmt_ok(src);
        assert!(out.contains("-5"));
        assert_idempotent(src);
    }

    #[test]
    fn litter_breed_claw_wear_round_trip() {
        let src = "litter Point {\nx #n,\ny #n\n}\n\nbreed Shape {\nCircle(#n),\nIdle\n}\n\nclaw Named {\ndescribe() #w\n}\n\nbare Named for Point {\npurr describe() #w {\nreturn ('a point')\n}\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn generic_litter_round_trips() {
        let src = "litter Holder<#t> {\n    value #t\n}\n\nlitter NamedHolder<#t: Named> {\n    value #t\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn pounce_round_trips() {
        let src = "func F() {\n    <{shape}> >> Circle(5)\n\n    pounce> <{shape}>\n        Circle(r) >> <{description}> >> 'circle'\n        else> <{description}> >> 'idle'\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn if_orif_else_round_trips() {
        let src = "func F() {\n    if><{age}> >= 18 && <{ready}> >> yes>\n        craft<'a'>\n    orif><{age}> < 13 || <{age}> >= 65\n        craft<'b'>\n    else>\n        craft<'c'>\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn spin_statement_and_view_round_trip() {
        let src = "func F() {\n    spin<{n}> in [1, 2, 3] }{\n        craft<n>\n    }{\n\n    return (\n        <ul>\n            spin<{item}> in items key(item.to_uppercase()) }{\n                <li>{ item }</li>\n            }{\n        </ul>\n    )\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn jsx_with_component_children_expands() {
        let src = "func F() {\n    return (\n        <div>\n            <Nav active='Counter'/>\n            <button onClick={<{count}> >> count + 1}>\n                \"Clicks: \"\n                <{count}>\n            </button>\n        </div>\n    )\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn empty_element_round_trips() {
        let src = "func F() {\n    return (\n        <div></div>\n    )\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn imports_and_private_and_export_round_trip() {
        let src = "import { A, B } from './x.kitty'\nexport import { C } from './y.kitty'\n\nprivate purr helper() #n {\n    return (1)\n}\n\nfunc F() {\n    craft<helper()>\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn method_call_field_access_path_struct_init_round_trip() {
        let src = "func F() {\n    <{o}> >> Point { x: 3, y: 4 }\n    craft<o.x>\n    craft<o.describe()>\n    craft<use_navigate()('/', NavigateOptions::default())>\n}\n";
        assert_idempotent(src);
    }

    #[test]
    fn already_formatted_file_is_a_no_op() {
        let src = "purr isAdult(age) {\n    return (age >= 18)\n}\n";
        let out = fmt_ok(src);
        assert_eq!(src, out);
    }

    #[test]
    fn detects_line_comments_outside_strings() {
        assert!(has_line_comments("// hi\nfunc F() {}\n"));
        assert!(has_line_comments("func F() {} // trailing\n"));
    }

    #[test]
    fn does_not_false_positive_on_slashes_in_strings() {
        assert!(!has_line_comments("purr f() #w { return ('http://example.com') }"));
        assert!(!has_line_comments("purr f() #w { return (\"a // not a comment\") }"));
    }
}
