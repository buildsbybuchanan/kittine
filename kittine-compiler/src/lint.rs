//! A real, AST-driven linter for `.kitty` source, powering
//! `kittine-compiler lint`.
//!
//! `ast::Stmt`/`ast::Expr` carry no source position (see `fmt.rs`'s module
//! doc comment for the same limitation on the formatter side), so every
//! warning here names the construct it's about rather than a line/column —
//! a real, honest constraint, not an oversight.
//!
//! What's checked, and why each one is a genuine issue rather than a style
//! opinion:
//!
//! - **Unused import**: dead weight, and a sign the import list has drifted
//!   from what the file actually uses.
//! - **Unused `private` item**: `private` means "not importable from
//!   another file" (see `ast::Component::is_private`); if nothing in *this*
//!   file uses it either, it's unreachable dead code.
//! - **Unused parameter**: a prop/`purr` param nothing in the body or
//!   returned view/expr ever reads.
//! - **Unused `hold` binding**: a `hold name >> expr` local nothing after
//!   it reads — the whole point of `hold` is to bind a value for later use
//!   (see `ast::Stmt::Hold`); if nothing reads it, the binding (and
//!   whatever side effect `expr` runs) is doing that for no reason anyone
//!   observes.
//! - **Duplicate field/variant/method/param names**: these become
//!   duplicate Rust struct fields / enum variants / fn params once
//!   generated — a real `cargo build` error (e.g. E0124), caught here
//!   before ever reaching `cargo`.
//!
//! Usage detection (see `collect_names`) is a conservative, whole-scope
//! name-occurrence walk: a name counts as "used" if it appears in *any*
//! read/reference position anywhere in the relevant scope, with no
//! reachability/liveness analysis (unlike, say, rustc's own `dead_code`
//! lint). This can under-warn (e.g. a `private purr` that only calls
//! itself recursively, with no other caller, won't be flagged) but is
//! designed to never over-warn — a false "unused" on real, live code would
//! be far more costly to a user's trust in the tool than an occasional
//! miss.

use crate::ast::*;
use std::collections::HashSet;

pub fn check(program: &Program) -> Vec<String> {
    let mut warnings = Vec::new();
    check_unused_imports(program, &mut warnings);
    check_unused_private_items(program, &mut warnings);
    check_duplicate_names(program, &mut warnings);
    check_unused_params(program, &mut warnings);
    check_unused_holds(program, &mut warnings);
    warnings
}

// ---- name-usage collection ---------------------------------------------

/// Scalar/placeholder type strings that never name a real declared item —
/// see `fmt::sig_type_str`/`fmt::field_type_str` for the same mapping used
/// on the printing side.
fn is_builtin_ty(ty: &str) -> bool {
    matches!(ty, "Num" | "Word" | "Flag" | "T" | "Children" | "")
}

/// Marks `ty` as a used type name, first stripping an array (`[]`) or
/// `stash` (`{}`) suffix so `DocEntry[]` notes `DocEntry` itself as used —
/// not the literal `"DocEntry[]"` string, which would never match the bare
/// `litter DocEntry` name `check_unused_private_items` looks for, a false
/// "unused" positive on any `litter`/`breed` only ever referenced through
/// an array-typed prop/return (see `codegen::rust_type`'s matching `[]`/
/// `{}` recursion on the codegen side).
fn note_ty(ty: &str, set: &mut HashSet<String>) {
    let base = ty.strip_suffix("[]").or_else(|| ty.strip_suffix("{}")).unwrap_or(ty);
    if !is_builtin_ty(base) {
        set.insert(base.to_string());
    }
}

fn collect_names_program(program: &Program, set: &mut HashSet<String>) {
    for item in &program.items {
        collect_names_item(item, set);
    }
}

fn collect_names_item(item: &Item, set: &mut HashSet<String>) {
    match item {
        Item::Component(c) => {
            for p in &c.params {
                note_ty(&p.ty, set);
            }
            collect_names_stmts(&c.body, set);
            if let Some(v) = &c.return_view {
                collect_names_jsx(v, set);
            }
        }
        Item::Function(f) => {
            for p in &f.params {
                note_ty(&p.ty, set);
            }
            note_ty(&f.return_type, set);
            collect_names_stmts(&f.body, set);
            collect_names_expr(&f.return_expr, set);
        }
        Item::Litter(l) => {
            for field in &l.fields {
                note_ty(&field.ty, set);
            }
        }
        Item::Breed(b) => {
            for v in &b.variants {
                if let Some(p) = &v.payload {
                    note_ty(p, set);
                }
            }
        }
        Item::Claw(c) => {
            for m in &c.methods {
                for p in &m.params {
                    note_ty(&p.ty, set);
                }
                note_ty(&m.return_type, set);
            }
        }
        Item::Wear(w) => {
            set.insert(w.claw.clone());
            set.insert(w.target.clone());
            for m in &w.methods {
                for p in &m.params {
                    note_ty(&p.ty, set);
                }
                note_ty(&m.return_type, set);
                collect_names_stmts(&m.body, set);
                collect_names_expr(&m.return_expr, set);
            }
        }
    }
}

fn collect_names_stmts(stmts: &[Stmt], set: &mut HashSet<String>) {
    for s in stmts {
        collect_names_stmt(s, set);
    }
}

fn collect_names_stmt(s: &Stmt, set: &mut HashSet<String>) {
    match s {
        // The declared name itself is a write/binding target, not a
        // "usage" of anything -- only its value expression is scanned.
        Stmt::VarAssign { value, .. } | Stmt::Hold { value, .. } | Stmt::Craft { value, .. } => {
            collect_names_expr(value, set);
        }
        Stmt::Expr(e) => collect_names_expr(e, set),
        Stmt::If {
            branches,
            else_body,
        } => {
            for (cond, body) in branches {
                collect_names_expr(cond, set);
                collect_names_stmts(body, set);
            }
            if let Some(eb) = else_body {
                collect_names_stmts(eb, set);
            }
        }
        Stmt::Spin { list, body, .. } => {
            collect_names_expr(list, set);
            collect_names_stmts(body, set);
        }
        Stmt::Pounce {
            subject,
            arms,
            catch_all,
        } => {
            collect_names_expr(subject, set);
            for arm in arms {
                set.insert(arm.variant.clone());
                collect_names_stmts(&arm.body, set);
            }
            if let Some(ca) = catch_all {
                collect_names_stmts(ca, set);
            }
        }
    }
}

fn collect_names_expr(e: &Expr, set: &mut HashSet<String>) {
    match e {
        Expr::Ident(name) | Expr::VarRead(name) => {
            set.insert(name.clone());
        }
        Expr::Number(_) | Expr::Str(_) | Expr::Bool(_) => {}
        Expr::Binary { left, right, .. } => {
            collect_names_expr(left, set);
            collect_names_expr(right, set);
        }
        Expr::InlineAssign { name, value } => {
            set.insert(name.clone());
            collect_names_expr(value, set);
        }
        Expr::Array(items) | Expr::Tuple(items) => {
            for i in items {
                collect_names_expr(i, set);
            }
        }
        Expr::Typed { value, .. } => collect_names_expr(value, set),
        Expr::Call { name, args } => {
            set.insert(name.clone());
            for a in args {
                collect_names_expr(a, set);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            collect_names_expr(receiver, set);
            for a in args {
                collect_names_expr(a, set);
            }
        }
        Expr::CallResult { callee, args } => {
            collect_names_expr(callee, set);
            for a in args {
                collect_names_expr(a, set);
            }
        }
        Expr::Path(segments) => {
            for s in segments {
                set.insert(s.clone());
            }
        }
        Expr::FieldAccess { receiver, .. } => collect_names_expr(receiver, set),
        Expr::StructInit { name, fields } => {
            set.insert(name.clone());
            for (_, v) in fields {
                collect_names_expr(v, set);
            }
        }
        Expr::Ref(inner) => collect_names_expr(inner, set),
        Expr::Pounce {
            subject,
            arms,
            catch_all,
        } => {
            collect_names_expr(subject, set);
            for arm in arms {
                set.insert(arm.variant.clone());
                collect_names_expr(&arm.body, set);
            }
            if let Some(ca) = catch_all {
                collect_names_expr(ca, set);
            }
        }
        Expr::Closure { body, .. } => collect_names_expr(body, set),
    }
}

fn collect_names_jsx(node: &JsxNode, set: &mut HashSet<String>) {
    match node {
        JsxNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            set.insert(tag.clone());
            for (_, v) in attrs {
                if let JsxAttrValue::Expr(e) = v {
                    collect_names_expr(e, set);
                }
            }
            for c in children {
                collect_names_jsx(c, set);
            }
        }
        JsxNode::Text(_) => {}
        JsxNode::VarInterp(name) => {
            set.insert(name.clone());
        }
        JsxNode::ExprInterp(e) => collect_names_expr(e, set),
        JsxNode::Spin {
            list, key, body, ..
        } => {
            collect_names_expr(list, set);
            if let Some(k) = key {
                collect_names_expr(k, set);
            }
            for c in body {
                collect_names_jsx(c, set);
            }
        }
    }
}

// ---- unused imports / private items ------------------------------------

fn check_unused_imports(program: &Program, warnings: &mut Vec<String>) {
    let mut used = HashSet::new();
    collect_names_program(program, &mut used);
    for imp in &program.imports {
        // `export import ..` is a re-export (`pub use`) -- its whole point
        // is to be used by a *third* file reaching through this one, not by
        // this file itself (see `ast::Import::is_export`), so "unused
        // here" isn't a meaningful signal for it.
        if imp.is_export {
            continue;
        }
        for name in &imp.names {
            if !used.contains(name) {
                warnings.push(format!(
                    "unused import '{name}' (from '{}')",
                    imp.path
                ));
            }
        }
    }
}

fn check_unused_private_items(program: &Program, warnings: &mut Vec<String>) {
    let mut used = HashSet::new();
    collect_names_program(program, &mut used);
    for item in &program.items {
        let (kind, name, is_private, variants) = match item {
            Item::Component(c) => ("component", c.name.as_str(), c.is_private, None),
            Item::Function(f) => ("purr", f.name.as_str(), f.is_private, None),
            Item::Litter(l) => ("litter", l.name.as_str(), l.is_private, None),
            Item::Breed(b) => (
                "breed",
                b.name.as_str(),
                b.is_private,
                Some(b.variants.iter().map(|v| v.name.as_str()).collect::<Vec<_>>()),
            ),
            Item::Claw(c) => ("claw", c.name.as_str(), c.is_private, None),
            Item::Wear(_) => continue, // an impl block is never itself importable
        };
        if !is_private {
            continue;
        }
        // A breed's own name rarely appears at a use site directly (values
        // are built/matched through its *variants* instead) -- also treat
        // it as used if any variant name shows up anywhere.
        let used_directly = used.contains(name);
        let used_via_variant = variants
            .as_ref()
            .is_some_and(|vs| vs.iter().any(|v| used.contains(*v)));
        if !used_directly && !used_via_variant {
            warnings.push(format!("unused private {kind} '{name}'"));
        }
    }
}

// ---- duplicate names ----------------------------------------------------

fn check_dupes(names: &[String], owner: &str, kind: &str, warnings: &mut Vec<String>) {
    let mut seen = HashSet::new();
    let mut reported = HashSet::new();
    for n in names {
        if !seen.insert(n.clone()) && reported.insert(n.clone()) {
            warnings.push(format!("duplicate {kind} '{n}' in {owner}"));
        }
    }
}

fn check_duplicate_names(program: &Program, warnings: &mut Vec<String>) {
    for item in &program.items {
        match item {
            Item::Litter(l) => {
                let names: Vec<String> = l.fields.iter().map(|f| f.name.clone()).collect();
                check_dupes(&names, &format!("litter '{}'", l.name), "field", warnings);
            }
            Item::Breed(b) => {
                let names: Vec<String> = b.variants.iter().map(|v| v.name.clone()).collect();
                check_dupes(&names, &format!("breed '{}'", b.name), "variant", warnings);
            }
            Item::Claw(c) => {
                let names: Vec<String> = c.methods.iter().map(|m| m.name.clone()).collect();
                check_dupes(&names, &format!("claw '{}'", c.name), "method", warnings);
                for m in &c.methods {
                    let pnames: Vec<String> = m.params.iter().map(|p| p.name.clone()).collect();
                    check_dupes(
                        &pnames,
                        &format!("claw method '{}' in claw '{}'", m.name, c.name),
                        "parameter",
                        warnings,
                    );
                }
            }
            Item::Component(comp) => {
                let names: Vec<String> = comp.params.iter().map(|p| p.name.clone()).collect();
                check_dupes(&names, &format!("component '{}'", comp.name), "parameter", warnings);
            }
            Item::Function(f) => {
                let names: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
                check_dupes(&names, &format!("purr '{}'", f.name), "parameter", warnings);
            }
            Item::Wear(w) => {
                let mnames: Vec<String> = w.methods.iter().map(|m| m.name.clone()).collect();
                check_dupes(
                    &mnames,
                    &format!("'bare {} for {}'", w.claw, w.target),
                    "method",
                    warnings,
                );
                for m in &w.methods {
                    let pnames: Vec<String> = m.params.iter().map(|p| p.name.clone()).collect();
                    check_dupes(
                        &pnames,
                        &format!("purr '{}' in 'bare {} for {}'", m.name, w.claw, w.target),
                        "parameter",
                        warnings,
                    );
                }
            }
        }
    }
}

// ---- unused params / holds ----------------------------------------------

fn local_used_names(body: &[Stmt], tail_expr: Option<&Expr>, tail_view: Option<&JsxNode>) -> HashSet<String> {
    let mut used = HashSet::new();
    collect_names_stmts(body, &mut used);
    if let Some(e) = tail_expr {
        collect_names_expr(e, &mut used);
    }
    if let Some(v) = tail_view {
        collect_names_jsx(v, &mut used);
    }
    used
}

fn check_unused_params(program: &Program, warnings: &mut Vec<String>) {
    for item in &program.items {
        match item {
            Item::Component(c) => {
                let used = local_used_names(&c.body, None, c.return_view.as_ref());
                for p in &c.params {
                    if !used.contains(&p.name) {
                        warnings.push(format!(
                            "unused parameter '{}' in component '{}'",
                            p.name, c.name
                        ));
                    }
                }
            }
            Item::Function(f) => {
                let used = local_used_names(&f.body, Some(&f.return_expr), None);
                for p in &f.params {
                    if !used.contains(&p.name) {
                        warnings.push(format!("unused parameter '{}' in purr '{}'", p.name, f.name));
                    }
                }
            }
            Item::Wear(w) => {
                for m in &w.methods {
                    let used = local_used_names(&m.body, Some(&m.return_expr), None);
                    for p in &m.params {
                        if !used.contains(&p.name) {
                            warnings.push(format!(
                                "unused parameter '{}' in purr '{}' in 'bare {} for {}'",
                                p.name, m.name, w.claw, w.target
                            ));
                        }
                    }
                }
            }
            Item::Litter(_) | Item::Breed(_) | Item::Claw(_) => {}
        }
    }
}

fn find_holds(stmts: &[Stmt]) -> Vec<String> {
    let mut out = Vec::new();
    for s in stmts {
        match s {
            Stmt::Hold { name, .. } => out.push(name.clone()),
            Stmt::If {
                branches,
                else_body,
            } => {
                for (_, body) in branches {
                    out.extend(find_holds(body));
                }
                if let Some(eb) = else_body {
                    out.extend(find_holds(eb));
                }
            }
            Stmt::Spin { body, .. } => out.extend(find_holds(body)),
            Stmt::Pounce {
                arms, catch_all, ..
            } => {
                for arm in arms {
                    out.extend(find_holds(&arm.body));
                }
                if let Some(ca) = catch_all {
                    out.extend(find_holds(ca));
                }
            }
            _ => {}
        }
    }
    out
}

fn check_unused_holds(program: &Program, warnings: &mut Vec<String>) {
    for item in &program.items {
        let (owner, body, tail_expr, tail_view) = match item {
            Item::Component(c) => (
                format!("component '{}'", c.name),
                &c.body,
                None,
                c.return_view.as_ref(),
            ),
            Item::Function(f) => (format!("purr '{}'", f.name), &f.body, Some(&f.return_expr), None),
            _ => continue,
        };
        let used = local_used_names(body, tail_expr, tail_view);
        for name in find_holds(body) {
            if !used.contains(&name) {
                warnings.push(format!("unused 'hold {name}' in {owner}"));
            }
        }
    }
    for item in &program.items {
        if let Item::Wear(w) = item {
            for m in &w.methods {
                let used = local_used_names(&m.body, Some(&m.return_expr), None);
                for name in find_holds(&m.body) {
                    if !used.contains(&name) {
                        warnings.push(format!(
                            "unused 'hold {name}' in purr '{}' in 'bare {} for {}'",
                            m.name, w.claw, w.target
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;

    fn lint(src: &str) -> Vec<String> {
        let tokens = lexer::tokenize(src).expect("lex");
        let program = parser::parse(tokens).expect("parse");
        check(&program)
    }

    #[test]
    fn flags_unused_import() {
        let src = "import { Foo } from './x.kitty'\n\nfunc F() {}\n";
        let w = lint(src);
        assert!(w.iter().any(|m| m.contains("unused import 'Foo'")), "{w:?}");
    }

    #[test]
    fn does_not_flag_export_import_as_unused() {
        // A barrel-file re-export is meant to go unused *locally* -- found
        // as a real false positive against example-app's components.kitty.
        let src = "export import { Nav } from './Nav.kitty'\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused import")), "{w:?}");
    }

    #[test]
    fn does_not_flag_used_import() {
        let src = "import { Foo } from './x.kitty'\n\nfunc F() {\n    return (\n        <Foo/>\n    )\n}\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused import")), "{w:?}");
    }

    #[test]
    fn flags_unused_private_purr() {
        let src = "private purr helper() #n {\n    return (1)\n}\n\nfunc F() {}\n";
        let w = lint(src);
        assert!(w.iter().any(|m| m.contains("unused private purr 'helper'")), "{w:?}");
    }

    #[test]
    fn does_not_flag_used_private_purr() {
        let src = "private purr helper() #n {\n    return (1)\n}\n\nfunc F() {\n    craft<helper()>\n}\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused private")), "{w:?}");
    }

    #[test]
    fn does_not_flag_public_unused_item() {
        // Not `private` -- may be used by an importer, so no warning.
        let src = "purr helper() #n {\n    return (1)\n}\n\nfunc F() {}\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused")), "{w:?}");
    }

    #[test]
    fn breed_used_only_via_variant_is_not_flagged() {
        let src = "private breed Shape {\n    Circle(#n),\n    Idle\n}\n\nfunc F() {\n    <{s}> >> Circle(5)\n}\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused private breed")), "{w:?}");
    }

    #[test]
    fn flags_duplicate_litter_field() {
        let src = "litter Point {\n    x #n,\n    x #n\n}\n";
        let w = lint(src);
        assert!(w.iter().any(|m| m.contains("duplicate field 'x'")), "{w:?}");
    }

    #[test]
    fn flags_duplicate_param() {
        let src = "purr f(a, a) #n {\n    return (1)\n}\n";
        let w = lint(src);
        assert!(w.iter().any(|m| m.contains("duplicate parameter 'a'")), "{w:?}");
    }

    #[test]
    fn flags_unused_parameter() {
        let src = "purr f(unused) #n {\n    return (1)\n}\n";
        let w = lint(src);
        assert!(w.iter().any(|m| m.contains("unused parameter 'unused'")), "{w:?}");
    }

    #[test]
    fn does_not_flag_used_parameter() {
        let src = "purr f(n) #n {\n    return (n + 1)\n}\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused parameter")), "{w:?}");
    }

    #[test]
    fn flags_unused_hold() {
        let src = "func F() {\n    hold x >> 1\n}\n";
        let w = lint(src);
        assert!(w.iter().any(|m| m.contains("unused 'hold x'")), "{w:?}");
    }

    #[test]
    fn does_not_flag_used_hold() {
        let src = "func F() {\n    hold navigate >> use_navigate()\n\n    craft<navigate('/')>\n}\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused 'hold")), "{w:?}");
    }

    #[test]
    fn clean_file_has_no_warnings() {
        let src = "func F() {\n    <{count}> >> #n 0\n\n    return (\n        <div>{ count }</div>\n    )\n}\n";
        assert!(lint(src).is_empty());
    }

    /// A `private litter` only ever referenced through an array-typed prop
    /// (`DocEntry[]`) must not be flagged "unused" — regression test for
    /// `note_ty` stripping the `[]` suffix before checking, so it notes
    /// `DocEntry` as used, not the literal string `"DocEntry[]"` (which
    /// would never match the bare `litter DocEntry` name this check looks
    /// for).
    #[test]
    fn private_litter_used_only_as_array_type_is_not_flagged() {
        let src = "private litter DocEntry {\n    title #w\n}\n\nfunc List(DocEntry[] entries) {\n    return ( <p>\"hi\"</p> )\n}\n";
        let w = lint(src);
        assert!(!w.iter().any(|m| m.contains("unused private litter")), "{w:?}");
    }
}
