//! Lowers a Kittine [`Program`] into idiomatic Leptos 0.7 Rust source.
//!
//! The mapping this module implements is exactly the one from the language
//! spec:
//!
//! - `<{x}> >> y` the *first* time `x` is seen in a component becomes a
//!   signal declaration: `let (x, set_x) = signal(y);`
//! - `<{x}> >> expr` any *later* time becomes a mutation:
//!   `set_x.update(|n| ..);`, with the `x + 1` / `x - 1` / `x * 1` / `x / 1`
//!   shape special-cased to the idiomatic `*n += 1;` form.
//! - `craft<expr>` becomes `leptos::logging::log!(..)`.
//! - `if>` / `orif>` / `else>` become `if` / `else if` / `else`, with `>>`
//!   conditions becoming `==` comparisons and variable reads becoming
//!   `.get()` calls.
//! - `[a, b, c]` becomes `vec![a, b, c]`; `yes>` / `no>` become `true` /
//!   `false`; a `#t` tag is erased (it's a compile-time-only check —
//!   see `parser::parse_type_tag`), emitting just the inner value.
//! - `spin<{item}> in list }{ .. }{` becomes a plain `for item in
//!   (list).into_iter() { .. }`.
//! - `func Name(#t prop, ..) { .. }` becomes a Leptos `#[component]`
//!   whose props are plain typed Rust function parameters (see
//!   [`rust_type`]) — not signals, since props don't get their own
//!   `<{name}>` declaration site.
//! - `purr name(#t param, ..) #t { .. return (expr) }`
//!   becomes a plain `pub fn name(param: Type) -> ReturnType { .. expr }`
//!   — no view, no `#[component]`.
//! - `import { A, B } from 'path.kitty'` becomes `#[path = "path.rs"] mod
//!   ..; use ..::{A, B};` — see `main.rs` for the recursive compilation of
//!   the imported file into that sibling `.rs`.
//! - The `return ( .. )` JSX tree becomes a Leptos `view! { .. }` block,
//!   with `onX` attributes rewritten to Leptos's `on:x=` event bindings —
//!   *unless* the tag is a component reference (`<Name .. />`, matched by a
//!   leading uppercase letter exactly like Leptos's own `view!` macro
//!   does), in which case attributes are passed as plain prop values
//!   instead of the reactive `move || ..` / `move |_| ..` closures used for
//!   real DOM attributes/events.

use crate::ast::*;
use std::collections::{HashMap, HashSet};

const INDENT: &str = "    ";

/// A `breed` variant's owning type and payload shape — see
/// `Signatures::variants`.
#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub breed: String,
    /// `Some(ty)` for a payload-carrying variant (`Circle(#n)`), `None`
    /// for a unit variant (`Idle`).
    pub payload: Option<String>,
}

/// Every `purr`/`litter`/`breed` signature reachable from a compile,
/// collected before any single file is generated — see `main.rs`'s
/// `collect_all_signatures`, which walks the whole `import` graph to build
/// one whole-program `Signatures` before generating any individual file's
/// code. Threaded through every `Scope`, this is what lets a call site
/// (`Expr::Call`), a struct literal (`Expr::StructInit`), or a bare name
/// (`Expr::Ident`) render correctly even when the `purr`/`litter`/`breed`
/// it refers to is defined in another file. Kittine has no namespacing, so
/// every map here is keyed by bare name, assuming names are unique across
/// the whole reachable graph — the same simplifying assumption Rust's own
/// `use` resolution already makes when two files import the same name
/// (Rust's own type checker is the source of truth on whether a *call* is
/// actually valid; these maps only ever inform a codegen coercion hint).
#[derive(Debug, Default)]
pub struct Signatures {
    /// Every `purr`'s parameter types, by function name, in declaration
    /// order — lets a call site tell whether a bare string literal
    /// argument needs `.to_string()` (see `render_call`).
    pub functions: HashMap<String, Vec<String>>,
    /// Every `litter`'s field names and types, in declaration order — lets
    /// a struct-literal field value get the same coercion a `purr`
    /// argument does (see `render_struct_init`).
    pub litters: HashMap<String, Vec<(String, String)>>,
    /// Every `breed` variant's owning type and payload shape, by variant
    /// name — lets a bare `Idle` or a call-shaped `Circle(5)` render as
    /// `Shape::Idle`/`Shape::Circle(5f64)` instead of being mistaken for an
    /// ordinary variable read or `purr` call (see `render_ident`,
    /// `render_call_or_variant`).
    pub variants: HashMap<String, VariantInfo>,
}

impl Signatures {
    /// Merges `other` into `self` — used to fold one file's signatures
    /// into the whole-import-graph map (see `main.rs`'s
    /// `collect_all_signatures`).
    pub fn merge(&mut self, other: Signatures) {
        self.functions.extend(other.functions);
        self.litters.extend(other.litters);
        self.variants.extend(other.variants);
    }
}

/// Threads both "which signals have been declared so far" (mutated as
/// statements are lowered) and "which names are typed parameters" (fixed
/// for the lifetime of a single component/function) through codegen.
#[derive(Clone)]
struct Scope<'a> {
    declared: HashSet<String>,
    params: HashMap<String, String>,
    /// Names bound by an enclosing view-position `spin` (`JsxNode::Spin`)
    /// or a `pounce>` arm's pattern binding. Reads of these are always
    /// `.clone()`d — see `render_var_read`.
    spin_items: HashSet<String>,
    /// Names bound by `hold name >> expr` (see `ast::Stmt::Hold`) — a
    /// plain, non-reactive local. Reads of these are always `.clone()`d
    /// too, same reasoning as `spin_items`: a held value may need to be
    /// captured by more than one reactive closure (e.g. two different
    /// event handlers), and moving a non-`Copy` value out of the first one
    /// would make it unavailable to the second.
    hold_items: HashSet<String>,
    /// Names bound by a `|param, ..| ..` closure literal (see
    /// `ast::Expr::Closure`) — unlike `spin_items`/`hold_items`, these read
    /// *bare*, not `.clone()`d: a closure param is a fresh value handed to
    /// that one invocation, never a name captured from an enclosing
    /// reactive closure that might run more than once, so there's no
    /// move-across-invocations hazard to guard against. Checked before
    /// `spin_items`/`hold_items`/`params` in `render_var_read` so a closure
    /// param correctly shadows an outer name of the same spelling instead
    /// of picking up that outer name's `.clone()`/`.get()` treatment.
    closure_params: HashSet<String>,
    /// Every reachable `purr`/`litter`/`breed` signature — see
    /// `Signatures`. Shared (not owned) across every `Scope` in a file,
    /// since it's the same program-wide map regardless of which
    /// component/function is being lowered right now.
    signatures: &'a Signatures,
    /// `true` only while rendering an `on<Event>` handler expression that
    /// actually reads the reserved `event` identifier (see
    /// `expr_references_event`) — makes a bare `Expr::Ident("event")`
    /// inside that one expression tree render as `event_target_value(&ev)`
    /// instead of an ordinary variable read. Scoped this narrowly (not a
    /// blanket reserved word) so a real signal/param/`hold` binding named
    /// `event` elsewhere in the program keeps rendering normally.
    event_bound: bool,
}

impl<'a> Scope<'a> {
    fn for_params(params: &[Param], signatures: &'a Signatures) -> Self {
        Scope {
            declared: HashSet::new(),
            params: params
                .iter()
                .map(|p| (p.name.clone(), p.ty.clone()))
                .collect(),
            spin_items: HashSet::new(),
            hold_items: HashSet::new(),
            closure_params: HashSet::new(),
            signatures,
            event_bound: false,
        }
    }

    /// A copy of this scope with `item` additionally marked as a
    /// spin-bound loop variable, for rendering a `JsxNode::Spin`'s body.
    fn with_spin_item(&self, item: &str) -> Self {
        let mut scope = self.clone();
        scope.spin_items.insert(item.to_string());
        scope
    }

    /// A copy of this scope with every name in `params` additionally
    /// marked as a closure-bound param — see `closure_params`.
    fn with_closure_params(&self, params: &[String]) -> Self {
        let mut scope = self.clone();
        for p in params {
            scope.closure_params.insert(p.clone());
        }
        scope
    }

    /// A copy of this scope with the reserved `event` identifier bound to
    /// the current `on<Event>` handler's event object — see `event_bound`.
    fn with_event_bound(&self) -> Self {
        let mut scope = self.clone();
        scope.event_bound = true;
        scope
    }
}

/// Collects a single file's own `purr`/`litter`/`breed` signatures — used
/// both to build a whole-import-graph `Signatures` (see `main.rs`'s
/// `collect_all_signatures`) and by `tests.rs`'s single-file `compile()`
/// helper.
pub fn collect_signatures(items: &[Item]) -> Signatures {
    let mut signatures = Signatures::default();
    for item in items {
        match item {
            Item::Function(f) => {
                signatures.functions.insert(
                    f.name.clone(),
                    f.params.iter().map(|p| p.ty.clone()).collect(),
                );
            }
            Item::Litter(l) => {
                signatures.litters.insert(
                    l.name.clone(),
                    l.fields
                        .iter()
                        .map(|field| (field.name.clone(), field.ty.clone()))
                        .collect(),
                );
            }
            Item::Breed(b) => {
                for v in &b.variants {
                    signatures.variants.insert(
                        v.name.clone(),
                        VariantInfo {
                            breed: b.name.clone(),
                            payload: v.payload.clone(),
                        },
                    );
                }
            }
            // A `claw`'s method signatures never need collecting here: a
            // method is only ever called via `receiver.method(args)`
            // (`Expr::MethodCall`), which already renders verbatim,
            // trusting Rust's own type checker — same reasoning as any
            // other method call. A `bare .. for ..` block has no name of
            // its own to key a map by, and its methods aren't callable
            // by bare name either.
            Item::Component(_) | Item::Claw(_) | Item::Wear(_) => {}
        }
    }
    signatures
}

/// `signatures` should cover every `purr`/`litter`/`breed` reachable from
/// `program` through its `import`s, not just `program`'s own items — see
/// `main.rs`'s `collect_all_signatures`, which builds exactly that map by
/// walking the whole import graph before any file is actually generated.
pub fn generate(program: &Program, signatures: &Signatures) -> String {
    let mut out = String::new();
    out.push_str("// Generated by kittine-compiler. Do not edit by hand.\n");
    // `unused_parens`: a closure body (see `render_closure`) always renders
    // a `Binary` operand through the same parenthesizing path an ordinary
    // (non-tail-position) expression gets — real, harmless noise (`|p| (p.x
    // > 2)`), not worth a `render_top_level`-style special case just to
    // silence a lint on generated code nobody hand-edits.
    out.push_str(
        "#![allow(unused_braces, unused_variables, dead_code, unused_imports, unused_parens)]\n\n",
    );
    out.push_str("use leptos::prelude::*;\n");
    // Kittine doesn't have (or need) its own routing syntax: `<Router>`,
    // `<Routes>`, `<Route>`, and `<A>` are just ordinary Leptos components,
    // composed exactly like any Kittine-defined one (see `is_component_tag`),
    // and `StaticSegment("path")` / `ParamSegment("id")` are plain function
    // calls Kittine already supports. Bringing leptos_router into scope
    // everywhere — instead of only where it's used — costs nothing but an
    // `unused_imports` allowance for files that don't route.
    out.push_str("use leptos_router::components::*;\n");
    out.push_str("use leptos_router::*;\n");
    // `leptos_router::hooks` (use_params_map, use_navigate, use_query_map,
    // ..) isn't re-exported at the crate root (`pub mod hooks;`, not `pub
    // use hooks::*;`), unlike `components` and `matching` — bring it into
    // scope the same unconditional way so a dynamic route's page can read
    // its segment via `use_params_map().get().get("id")` (see
    // `Expr::MethodCall`) with no new Kittine syntax at all.
    out.push_str("use leptos_router::hooks::*;\n");
    // Same reasoning as leptos_router above: `<Title>`, `<Meta>`, `<Link>`,
    // `<Stylesheet>` etc. are plain Leptos components (see
    // `is_component_tag`), not Kittine syntax, needed by any page that
    // wants to set its own per-route `<title>`/meta description for SEO.
    out.push_str("use leptos_meta::*;\n\n");
    out.push_str(&gen_imports(&program.imports));
    for item in &program.items {
        match item {
            Item::Component(c) => out.push_str(&gen_component(c, signatures)),
            Item::Function(f) => out.push_str(&gen_function(f, signatures)),
            Item::Litter(l) => out.push_str(&gen_litter(l)),
            Item::Breed(b) => out.push_str(&gen_breed(b)),
            Item::Claw(c) => out.push_str(&gen_claw(c)),
            Item::Wear(w) => out.push_str(&gen_wear(w, signatures)),
        }
        out.push('\n');
    }
    out
}

fn push_line(out: &mut String, indent: usize, text: &str) {
    for line in text.lines() {
        out.push_str(&INDENT.repeat(indent));
        out.push_str(line);
        out.push('\n');
    }
}

/// Maps a Kittine type name to its generated Rust type: the three
/// scalars, `Children` (the special untyped `children` param), `T` (a
/// `litter`/`breed`'s own generic type parameter — see
/// `ast::Litter::has_type_param`), a trailing `[]` (an array of whatever
/// the rest of `ty` maps to — `Vec<T>`, recursing so this covers a scalar
/// array (`Num[]` -> `Vec<f64>`) and a custom `litter`/`breed` array
/// (`DocEntry[]` -> `Vec<DocEntry>`) with one rule instead of enumerating
/// every scalar combination by hand), a trailing `{}` (a `String`-keyed
/// map, same recursion), or anything else verbatim — a reference to
/// another `litter`/`breed` by name, since Kittine's type names and Rust's
/// generated type names are the same identifier for a user-declared
/// record/variant type.
fn rust_type(ty: &str) -> String {
    if let Some(elem) = ty.strip_suffix("[]") {
        return format!("Vec<{}>", rust_type(elem));
    }
    if let Some(elem) = ty.strip_suffix("{}") {
        return format!("std::collections::HashMap<String, {}>", rust_type(elem));
    }
    match ty {
        "Num" => "f64".to_string(),
        "Word" => "String".to_string(),
        "Flag" => "bool".to_string(),
        "Children" => "Children".to_string(),
        "T" => "T".to_string(),
        other => other.to_string(),
    }
}

/// `true` for a parameter type that isn't `Copy` in the generated Rust —
/// reading it needs `.clone()` at use sites (see [`render_var_read`]),
/// since a prop may be read from more than one reactive closure. A `litter`/
/// `breed`'s own generic type parameter (`T`) is treated as non-`Copy`
/// (the safe default: a `Word` instantiation, for instance, wouldn't be),
/// and so is any other name Kittine doesn't recognize as one of the three
/// `Copy` scalars — a reference to a user-declared `litter`/`breed`, which
/// `gen_litter`/`gen_breed` never derives `Copy` for.
fn is_non_copy_param_type(ty: &str) -> bool {
    !matches!(ty, "Num" | "Flag" | "Children")
}

fn param_list_rust(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| format!("{}: {}", p.name, rust_type(&p.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---- imports ----------------------------------------------------------------

/// Derives a collision-resistant Rust module identifier from an import's
/// `.kitty` path by replacing every non-alphanumeric character (`.`, `/`,
/// `-`, ..) with `_`. The full relative path is embedded (not just the file
/// stem) so two imports with the same basename in different directories
/// don't collide.
fn module_ident_for_path(path: &str) -> String {
    let stem = path.strip_suffix(".kitty").unwrap_or(path);
    let mut sanitized = String::new();
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() {
            sanitized.extend(c.to_lowercase());
        } else if !sanitized.ends_with('_') {
            sanitized.push('_');
        }
    }
    let sanitized = sanitized.trim_matches('_');
    format!("__kittine_mod_{sanitized}")
}

/// The sibling `.rs` path a given import resolves to: same relative path,
/// `.kitty` swapped for `.rs`. `main.rs`'s recursive build uses this exact
/// convention to know where to write the compiled dependency.
pub fn rs_path_for_import(path: &str) -> String {
    match path.strip_suffix(".kitty") {
        Some(stem) => format!("{stem}.rs"),
        None => format!("{path}.rs"),
    }
}

fn gen_imports(imports: &[Import]) -> String {
    let mut out = String::new();
    for import in imports {
        let rs_path = rs_path_for_import(&import.path);
        let mod_ident = module_ident_for_path(&import.path);
        out.push_str(&format!("#[path = {rs_path:?}]\n"));
        out.push_str(&format!("mod {mod_ident};\n"));
        // `export import` re-exports `names` as part of this file's own
        // public interface (`pub use`) — a third file can then `import`
        // them straight from here without reaching back to where they're
        // actually defined. `mod_ident` itself stays a plain (non-`pub`)
        // declaration either way: a `pub use` re-export doesn't need its
        // source module to be `pub` too, only the item it's re-exporting.
        let visibility = if import.is_export { "pub " } else { "" };
        out.push_str(&format!(
            "{visibility}use {mod_ident}::{{{}}};\n",
            import.names.join(", ")
        ));
    }
    if !imports.is_empty() {
        out.push('\n');
    }
    out
}

// ---- top-level items ---------------------------------------------------------

/// `pub ` unless the item was declared `private` — see
/// `ast::Component::is_private`. A `private` item becomes a plain
/// (non-`pub`) Rust item, so `import`ing it from another file is a Rust
/// compile error (E0603) on its own; Kittine doesn't re-check this itself.
fn visibility(is_private: bool) -> &'static str {
    if is_private { "" } else { "pub " }
}

fn gen_component(component: &Component, signatures: &Signatures) -> String {
    let mut out = String::new();
    out.push_str("#[component]\n");
    out.push_str(&format!(
        "{}fn {}({}) -> impl IntoView {{\n",
        visibility(component.is_private),
        component.name,
        param_list_rust(&component.params)
    ));

    let mut scope = Scope::for_params(&component.params, signatures);
    for stmt in &component.body {
        gen_stmt(stmt, &mut scope, &mut out, 1);
    }

    match &component.return_view {
        Some(view) => {
            push_line(&mut out, 1, "view! {");
            out.push_str(&jsx_to_rust(view, &scope, 2));
            push_line(&mut out, 1, "}");
        }
        None => push_line(&mut out, 1, "view! { <></> }"),
    }

    out.push_str("}\n");
    out
}

fn gen_function(function: &Function, signatures: &Signatures) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}fn {}({}) -> {} {{\n",
        visibility(function.is_private),
        function.name,
        param_list_rust(&function.params),
        rust_type(&function.return_type)
    ));

    let mut scope = Scope::for_params(&function.params, signatures);
    for stmt in &function.body {
        gen_stmt(stmt, &mut scope, &mut out, 1);
    }
    let return_code = render_return_value(&function.return_expr, &function.return_type, &scope);
    push_line(&mut out, 1, &return_code);

    out.push_str("}\n");
    out
}

/// Renders a `litter`/`breed`'s optional `TypeParam` as the Rust generic
/// parameter suffix — `""` (no type param), `"<T>"` (unbounded), or
/// `"<T: Bound>"` (bounded by a `claw`, checked by Rust's own trait
/// system once generated, not re-verified here).
fn generics_suffix(type_param: &Option<TypeParam>) -> String {
    match type_param {
        None => String::new(),
        Some(TypeParam { bound: None }) => "<T>".to_string(),
        Some(TypeParam { bound: Some(b) }) => format!("<T: {b}>"),
    }
}

/// `litter Name type_param? { field type, .. }` becomes a plain Rust
/// `struct` — `#[derive(Clone, Debug)]` so a `litter`-typed value can be
/// read from more than one reactive closure the same way a `Word` prop
/// already can (see `is_non_copy_param_type`), plus `serde::Serialize,
/// serde::Deserialize` (fully path-qualified, so no `use serde::..` is
/// needed in the generated file) so a `litter` value can round-trip
/// through `serde_json::to_string`/`from_str` (or any other serde-backed
/// format — YAML, CSV, ... — the same derive covers all of them) via a
/// plain [path-qualified call](../LANGUAGE.md#path-qualified-expressions),
/// no dedicated Kittine syntax needed. This is unconditional, the same as
/// `Clone, Debug` already are — a project with even one `litter`/`breed`
/// now needs `serde` (with the `derive` feature) as a real Cargo
/// dependency, whether or not it actually serializes anything; see
/// LANGUAGE.md § Litters.
fn gen_litter(litter: &Litter) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]\n");
    let generics = generics_suffix(&litter.type_param);
    out.push_str(&format!(
        "{}struct {}{generics} {{\n",
        visibility(litter.is_private),
        litter.name,
    ));
    for field in &litter.fields {
        out.push_str(&format!(
            "    pub {}: {},\n",
            field.name,
            rust_type(&field.ty)
        ));
    }
    out.push_str("}\n");
    out
}

/// `breed Name type_param? { Variant (type)?, .. }` becomes a plain Rust
/// `enum` — same `Clone, Debug, serde::Serialize, serde::Deserialize`
/// derive reasoning as `gen_litter`.
fn gen_breed(breed: &Breed) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]\n");
    let generics = generics_suffix(&breed.type_param);
    out.push_str(&format!(
        "{}enum {}{generics} {{\n",
        visibility(breed.is_private),
        breed.name,
    ));
    for variant in &breed.variants {
        match &variant.payload {
            Some(ty) => out.push_str(&format!("    {}({}),\n", variant.name, rust_type(ty))),
            None => out.push_str(&format!("    {},\n", variant.name)),
        }
    }
    out.push_str("}\n");
    out
}

/// `claw Name { method(params) type, .. }` becomes a plain Rust `trait`
/// — every method a bare signature (no body, see `ast::Claw`), each
/// taking an implicit `&self` (see `method_param_list_rust`).
fn gen_claw(claw: &Claw) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}trait {} {{\n",
        visibility(claw.is_private),
        claw.name
    ));
    for method in &claw.methods {
        out.push_str(&format!(
            "    fn {}({}) -> {};\n",
            method.name,
            method_param_list_rust(&method.params),
            rust_type(&method.return_type)
        ));
    }
    out.push_str("}\n");
    out
}

/// `bare Claw for Target { purr method(..) .. }*` becomes a plain Rust
/// `impl Claw for Target { .. }`, reusing the exact same statement/
/// expression codegen a top-level `purr` gets (see `gen_function`) —
/// just with an implicit `&self` receiver instead of `pub fn`.
fn gen_wear(wear: &Wear, signatures: &Signatures) -> String {
    let mut out = String::new();
    out.push_str(&format!("impl {} for {} {{\n", wear.claw, wear.target));
    for method in &wear.methods {
        push_line(
            &mut out,
            1,
            &format!(
                "fn {}({}) -> {} {{",
                method.name,
                method_param_list_rust(&method.params),
                rust_type(&method.return_type)
            ),
        );
        let mut scope = Scope::for_params(&method.params, signatures);
        for stmt in &method.body {
            gen_stmt(stmt, &mut scope, &mut out, 2);
        }
        let return_code = render_return_value(&method.return_expr, &method.return_type, &scope);
        push_line(&mut out, 2, &return_code);
        push_line(&mut out, 1, "}");
    }
    out.push_str("}\n");
    out
}

/// Like `param_list_rust`, but for a `claw` method/`bare` impl method,
/// which always takes an implicit `&self` first — never written by a
/// `.kitty` author (the same "no declaration needed" treatment
/// `children` already gets), but always present in the generated Rust.
fn method_param_list_rust(params: &[Param]) -> String {
    let mut parts = vec!["&self".to_string()];
    parts.extend(params.iter().map(|p| format!("{}: {}", p.name, rust_type(&p.ty))));
    parts.join(", ")
}

// ---- statements ----------------------------------------------------------

fn gen_stmt(stmt: &Stmt, scope: &mut Scope, out: &mut String, indent: usize) {
    match stmt {
        Stmt::VarAssign { name, value } => {
            if scope.declared.contains(name) {
                let body = mutation_body(name, value, scope);
                push_line(out, indent, &format!("set_{name}.update(|n| {body});"));
            } else {
                let value_code = render_signal_init(value, scope);
                push_line(
                    out,
                    indent,
                    &format!("let ({name}, set_{name}) = signal({value_code});"),
                );
                scope.declared.insert(name.clone());
            }
        }
        Stmt::Craft { value, level } => {
            let code = match value {
                Expr::Str(s) => {
                    format!("leptos::logging::{level}!(\"{}\");", escape_str(s))
                }
                other if is_array_like(other) => format!(
                    "leptos::logging::{level}!(\"{{:?}}\", {});",
                    expr_to_rust(other, scope)
                ),
                other => format!(
                    "leptos::logging::{level}!(\"{{}}\", {});",
                    expr_to_rust(other, scope)
                ),
            };
            push_line(out, indent, &code);
        }
        Stmt::If {
            branches,
            else_body,
        } => {
            for (i, (cond, body)) in branches.iter().enumerate() {
                let cond_code = render_condition(cond, scope);
                if i == 0 {
                    push_line(out, indent, &format!("if {cond_code} {{"));
                } else {
                    push_line(out, indent, &format!("}} else if {cond_code} {{"));
                }
                for s in body {
                    gen_stmt(s, scope, out, indent + 1);
                }
            }
            if let Some(body) = else_body {
                push_line(out, indent, "} else {");
                for s in body {
                    gen_stmt(s, scope, out, indent + 1);
                }
            }
            push_line(out, indent, "}");
        }
        Stmt::Expr(expr) => {
            push_line(out, indent, &format!("{};", expr_to_rust(expr, scope)));
        }
        Stmt::Spin { item, list, body } => {
            let list_code = expr_to_rust(list, scope);
            push_line(
                out,
                indent,
                &format!("for {item} in ({list_code}).into_iter() {{"),
            );
            for s in body {
                gen_stmt(s, scope, out, indent + 1);
            }
            push_line(out, indent, "}");
        }
        Stmt::Hold { name, value } => {
            let value_code = expr_to_rust(value, scope);
            push_line(out, indent, &format!("let {name} = {value_code};"));
            scope.hold_items.insert(name.clone());
        }
        Stmt::Pounce {
            subject,
            arms,
            catch_all,
        } => {
            let subject_code = expr_to_rust(subject, scope);
            push_line(out, indent, &format!("match {subject_code} {{"));
            for arm in arms {
                // The variant's owning breed isn't spelled out in Kittine
                // source (`Circle(r) >> ..`, not `Shape::Circle(r) >> ..`)
                // — looked up here the same way a call-shaped variant
                // construction is (see `render_call_or_variant`), from the
                // whole-reachable-graph `Signatures` rather than assumed.
                let breed = scope
                    .signatures
                    .variants
                    .get(&arm.variant)
                    .map(|v| v.breed.as_str())
                    .unwrap_or(&arm.variant);
                let pattern = match &arm.binding {
                    Some(binding) => format!("{breed}::{}({binding})", arm.variant),
                    None => format!("{breed}::{}", arm.variant),
                };
                push_line(out, indent + 1, &format!("{pattern} => {{"));
                // The pattern's bound variable gets the same "may be read
                // from more than one place, always `.clone()` it" treatment
                // as a `hold` binding or a `spin` loop variable — arm-local,
                // so this clone of `scope` is discarded once the arm is
                // rendered, exactly like a real Rust match arm's own scope.
                let mut arm_scope = scope.clone();
                if let Some(binding) = &arm.binding {
                    arm_scope.hold_items.insert(binding.clone());
                }
                for s in &arm.body {
                    gen_stmt(s, &mut arm_scope, out, indent + 2);
                }
                push_line(out, indent + 1, "}");
            }
            if let Some(body) = catch_all {
                push_line(out, indent + 1, "_ => {");
                let mut arm_scope = scope.clone();
                for s in body {
                    gen_stmt(s, &mut arm_scope, out, indent + 2);
                }
                push_line(out, indent + 1, "}");
            }
            push_line(out, indent, "}");
        }
    }
}

/// Body of `set_name.update(|n| BODY)` for a mutation of `name`, as a bare
/// expression with no trailing `;` — callers decide whether the surrounding
/// context needs one.
fn mutation_body(name: &str, value: &Expr, scope: &Scope) -> String {
    if let Expr::Binary { left, op, right } = value
        && is_same_var(left, name)
        && let Expr::Number(n) = **right
    {
        let compound_op = match op {
            BinOp::Add => Some("+="),
            BinOp::Sub => Some("-="),
            BinOp::Mul => Some("*="),
            BinOp::Div => Some("/="),
            _ => None,
        };
        if let Some(op_str) = compound_op {
            // Same reasoning as `render_arith_operand`: `*n` is a concrete
            // `f64` (not a still-generic inference variable), so the
            // literal on the other side of `+=`/`-=`/`*=`/`/=` needs to be
            // spelled unambiguously as `f64` too.
            return format!("*n {op_str} {}", fmt_num_unambiguous(n));
        }
    }
    if is_bare_string_literal(value) {
        // Same reasoning as `render_signal_init`, just for a *later*
        // mutation instead of the signal's first/declaring occurrence:
        // `*n = "reset"` doesn't type-check when `*n: &mut String` (a
        // `Word` signal) — a bare string literal is `&'static str`, not
        // `String`. Concatenation (`<{label}> >> 'x' + <{label}>`) is
        // already unaffected, since `render_binary`'s string-`+` case
        // already lowers to an owned `format!(..)` regardless.
        return format!("*n = {}.to_string()", substitute_self(value, name, scope));
    }
    format!("*n = {}", substitute_self(value, name, scope))
}

fn is_same_var(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Ident(n) | Expr::VarRead(n) if n == name)
}

/// Renders a signal's initial value passed to `signal(..)`. Two forms of
/// the same underlying issue get forced to an unambiguous, owned form
/// here:
///
/// - A bare whole number becomes an explicit `f64` (see
///   [`render_arith_operand`]): `signal(0)` leaves `0` as an unconstrained
///   generic-inference literal, free to resolve to some type other than
///   `f64` if nothing *else* in the same function pins it down first.
/// - A string literal becomes an owned `String` via `.to_string()`:
///   `signal("Admin")` alone would make the signal's element type
///   `&'static str`, not `String` — fine until that signal's value is
///   later required to be an owned `String` (passed as a `Word`-typed
///   prop to another component), where it silently fails to compile.
///
/// Both failure modes are the same shape: a literal's type is left to
/// whatever the *first* thing that happens to constrain it decides, which
/// isn't necessarily what a *later*, separately-compiled-looking use
/// requires. Explicit beats implicit here. Recurses into array elements
/// too, so `[1, 2]` / `['a', 'b']` get the same treatment as a bare
/// literal would.
fn render_signal_init(value: &Expr, scope: &Scope) -> String {
    match value {
        Expr::Number(n) => fmt_num_unambiguous(*n),
        Expr::Str(s) => format!("\"{}\".to_string()", escape_str(s)),
        Expr::Array(items) => {
            let rendered: Vec<String> = items.iter().map(|e| render_signal_init(e, scope)).collect();
            format!("vec![{}]", rendered.join(", "))
        }
        Expr::Typed { value: inner, .. } => render_signal_init(inner, scope),
        other => expr_to_rust(other, scope),
    }
}

/// Renders `value` as a Rust expression for use inside `|n| ..`, rewriting
/// self-references (`x` inside the update for `x`) to `*n` instead of
/// `x.get()`.
fn substitute_self(expr: &Expr, name: &str, scope: &Scope) -> String {
    match expr {
        Expr::Ident(n) | Expr::VarRead(n) if n == name => "*n".to_string(),
        Expr::Ident(n) => render_ident(n, scope),
        Expr::VarRead(n) => render_var_read(n, scope),
        Expr::Number(n) => fmt_num(*n),
        Expr::Str(s) => format!("\"{}\"", escape_str(s)),
        Expr::Binary { left, op, right } => {
            render_binary(left, *op, right, &|e| substitute_self(e, name, scope))
        }
        Expr::InlineAssign { name: n, value } => mutation_expr(n, value, scope),
        Expr::Bool(b) => b.to_string(),
        Expr::Array(items) => render_array(items, &|e| substitute_self(e, name, scope)),
        Expr::Typed { value, .. } => substitute_self(value, name, scope),
        Expr::Call { name: fname, args } => {
            render_call_or_variant(fname, args, &|e| substitute_self(e, name, scope), scope)
        }
        Expr::MethodCall { receiver, method, args } => {
            render_method_call(receiver, method, args, &|e| substitute_self(e, name, scope))
        }
        Expr::CallResult { callee, args } => {
            render_call_result(callee, args, &|e| substitute_self(e, name, scope))
        }
        Expr::Tuple(items) => render_tuple(items, &|e| substitute_self(e, name, scope)),
        Expr::Path(segments) => render_path(segments),
        Expr::FieldAccess { receiver, field } => {
            format!("{}.{field}", substitute_self(receiver, name, scope))
        }
        Expr::StructInit {
            name: struct_name,
            fields,
        } => render_struct_init(
            struct_name,
            fields,
            &|e| substitute_self(e, name, scope),
            scope,
        ),
        Expr::Ref(inner) => format!("&{}", substitute_self(inner, name, scope)),
        Expr::Pounce {
            subject,
            arms,
            catch_all,
        } => render_pounce_expr(subject, arms, catch_all, scope, &|e, s| {
            substitute_self(e, name, s)
        }),
        Expr::Closure { params, body } => {
            render_closure(params, body, scope, &|e, s| substitute_self(e, name, s))
        }
    }
}

/// Renders `name(arg, arg, ..)`. Each argument gets the same bare-literal
/// safety as an arithmetic operand (see [`render_arith_operand`]) — a
/// number literal passed directly as a call argument doesn't type-check
/// against an `f64` parameter without an explicit `f64` suffix either.
///
/// Additionally, if `name` is a same-file `purr` whose parameter at this
/// argument's position is `Word` (known via `scope.signatures`), a
/// bare string-literal argument gets `.to_string()` appended — otherwise
/// it renders as `&str`, which doesn't type-check against a `Word`
/// parameter's `String`. This is real type information, not a guess, which
/// is why it's safe here in a way that always-owning every string literal
/// (tried once, reverted — see `expr_to_rust`'s `Expr::Str` arm) wasn't: an
/// import'ed (cross-file) callee's signature isn't known, so a literal
/// passed to one still doesn't get this treatment — see
/// `docs/LANGUAGE.md` § Known limitations.
fn render_call(name: &str, args: &[Expr], render: &dyn Fn(&Expr) -> String, scope: &Scope) -> String {
    let param_types = scope.signatures.functions.get(name);
    let rendered: Vec<String> = args
        .iter()
        .enumerate()
        .map(|(i, arg)| {
            let rendered_arg = render_arith_operand(arg, render);
            let needs_owned_string = is_bare_string_literal(arg)
                && param_types.and_then(|types| types.get(i)).is_some_and(|ty| ty == "Word");
            if needs_owned_string {
                format!("{rendered_arg}.to_string()")
            } else {
                rendered_arg
            }
        })
        .collect();
    // A `hold`-bound local can be a closure (`use_navigate()`'s result,
    // for instance) — calling it needs the same `.clone()` any other read
    // of it gets (see `render_var_read`), since it may need to be called
    // from more than one reactive closure and moving it out of the first
    // would make it unavailable to the second. A same-file `purr`/unknown
    // function name is never in `hold_items`, so this doesn't affect the
    // ordinary named-call case at all.
    let callee = if scope.hold_items.contains(name) {
        format!("{name}.clone()")
    } else {
        name.to_string()
    };
    format!("{callee}({})", rendered.join(", "))
}

/// `true` for a string literal, optionally wrapped in a `#w` type
/// tag — used by [`render_call`] to decide whether an argument needs
/// `.to_string()` for a known `Word` parameter.
fn is_bare_string_literal(expr: &Expr) -> bool {
    match expr {
        Expr::Str(_) => true,
        Expr::Typed { ty, value } => ty == "Word" && is_bare_string_literal(value),
        _ => false,
    }
}

/// Renders a bare `Expr::Ident` — either a known unit `breed` variant (no
/// payload), rendered `Breed::Variant`, or an ordinary name via
/// [`render_var_read`]. A *payload*-carrying variant is never bare like
/// this — `Circle(5)` parses as `Expr::Call`, handled by
/// [`render_call_or_variant`] instead, so there's no ambiguity between "an
/// `Ident` that happens to share a variant's name" and an actual
/// zero-payload variant reference.
fn render_ident(name: &str, scope: &Scope) -> String {
    if scope.event_bound && name == "event" {
        return "event_target_value(&ev)".to_string();
    }
    match scope.signatures.variants.get(name) {
        Some(v) if v.payload.is_none() => format!("{}::{name}", v.breed),
        _ => render_var_read(name, scope),
    }
}

/// Renders `name(arg, arg, ..)` where `name` might be a `breed` variant
/// constructor (`Circle(5)` → `Shape::Circle(5f64)`) rather than a `purr`
/// call — checked first via `scope.signatures.variants`, since a variant
/// and a same-file `purr` share the same call-argument syntax and are only
/// told apart by which one `name` actually names. A payload argument gets
/// the same `Word`-parameter string-literal `.to_string()` coercion
/// `render_call` gives a `purr` argument (see its own doc comment) — real
/// type information from the variant's own declaration, not a guess.
fn render_call_or_variant(
    name: &str,
    args: &[Expr],
    render: &dyn Fn(&Expr) -> String,
    scope: &Scope,
) -> String {
    match scope.signatures.variants.get(name) {
        Some(v) => {
            let rendered: Vec<String> = args
                .iter()
                .map(|arg| {
                    let rendered_arg = render_arith_operand(arg, render);
                    let needs_owned_string =
                        is_bare_string_literal(arg) && v.payload.as_deref() == Some("Word");
                    if needs_owned_string {
                        format!("{rendered_arg}.to_string()")
                    } else {
                        rendered_arg
                    }
                })
                .collect();
            format!("{}::{name}({})", v.breed, rendered.join(", "))
        }
        None => render_call(name, args, render, scope),
    }
}

/// Renders `Name { field: expr, .. }` — see `ast::Expr::StructInit`. When
/// `name`'s fields are known (via `scope.signatures.litters`, collected
/// across the whole reachable `import` graph the same way `purr` and
/// `breed` signatures are), a `Word`-typed field gets the same bare-
/// string-literal `.to_string()` coercion a `purr` argument does, and a
/// `Num`-typed field gets the same unambiguous-`f64` treatment an
/// arithmetic operand does. A generic (`T`-typed) field, or a `litter`
/// this compile has no signature for, instead renders its value through
/// [`render_generic_field_value`] — Rust infers the concrete type
/// parameter from the value itself, the same way it infers any other
/// generic constructor call, with no explicit instantiation needed.
///
/// `name == "stash"` is the one reserved exception, handled first and
/// entirely separately (see [`render_stash_literal`]): a `stash{ .. }`
/// literal reuses this exact same `Name { field: expr, .. }` grammar (so
/// parsing/`fmt`/the duplicate-field lint all come for free), but lowers
/// to a `HashMap`, not a struct — `stash` is never a real registered
/// `litter`, so `scope.signatures.litters.get("stash")` would always be
/// `None` anyway even without this early return; the early return just
/// makes that intentional instead of accidental.
fn render_struct_init(
    name: &str,
    fields: &[(String, Expr)],
    render: &dyn Fn(&Expr) -> String,
    scope: &Scope,
) -> String {
    if name == "stash" {
        return render_stash_literal(fields, render);
    }
    let field_types = scope.signatures.litters.get(name);
    let rendered: Vec<String> = fields
        .iter()
        .map(|(field_name, field_expr)| {
            let ty = field_types
                .and_then(|fields| fields.iter().find(|(n, _)| n == field_name))
                .map(|(_, ty)| ty.as_str());
            let value_code = match ty {
                Some("Word") if is_bare_string_literal(field_expr) => {
                    format!("{}.to_string()", render_arith_operand(field_expr, render))
                }
                Some("Num") => render_arith_operand(field_expr, render),
                _ => render_generic_field_value(field_expr, render),
            };
            format!("{field_name}: {value_code}")
        })
        .collect();
    format!("{name} {{ {} }}", rendered.join(", "))
}

/// Renders `stash{ key: expr, .. }` (see `ast::Expr::StructInit`'s
/// `render_struct_init` caller) to a `std::collections::HashMap` literal —
/// a "stash" is always `String`-keyed, the key coming from the field name
/// itself (already a plain identifier — `parse_struct_init_field` requires
/// one, so no `escape_str` is needed the way an arbitrary string value
/// would). Values get the same literal-owning treatment
/// `render_generic_field_value` already gives any other struct-literal
/// field with no declared type to guide it (a bare number -> unambiguous
/// `f64`, a bare string -> owned `String`), which is exactly what a
/// `Num{}`/`Word{}`/`Flag{}`-typed map needs too.
fn render_stash_literal(fields: &[(String, Expr)], render: &dyn Fn(&Expr) -> String) -> String {
    let rendered: Vec<String> = fields
        .iter()
        .map(|(key, value)| {
            format!(
                "(\"{key}\".to_string(), {})",
                render_generic_field_value(value, render)
            )
        })
        .collect();
    format!("std::collections::HashMap::from([{}])", rendered.join(", "))
}

/// Renders a struct-literal field value with no declared (or generic `T`)
/// type to guide it: a literal renders in its natural owned Rust form
/// (same reasoning as [`render_signal_init`]), anything else renders as
/// whatever `render` would otherwise produce.
fn render_generic_field_value(expr: &Expr, render: &dyn Fn(&Expr) -> String) -> String {
    match expr {
        Expr::Number(n) => fmt_num_unambiguous(*n),
        Expr::Str(s) => format!("\"{}\".to_string()", escape_str(s)),
        other => render(other),
    }
}

/// Renders `receiver.method(arg, arg, ..)` — see `ast::Expr::MethodCall`.
/// Kittine has no receiver *or* argument type information for an arbitrary
/// method (unlike [`render_call`], where a same-file `purr`'s `Num`
/// parameters are always known to be `f64`), so a bare numeric literal
/// argument is rendered plain, not forced to `f64` — a real Rust method
/// just as often expects `usize`/`i32`/etc. (`Vec::get(0)`, not
/// `Vec::get(0f64)`), and Kittine has no way to tell which. Rust's own
/// type checker is the source of truth on whether the call is valid.
///
/// Three method names are reserved exceptions, checked first, exactly the
/// way `name == "stash"` is a reserved exception in
/// [`render_struct_init`]: `fixed`/`padded`/`grouped` are number-formatting
/// utilities `format!`'s macro syntax can express but no real Rust *method*
/// on `f64`/any other type can (Rust has no `.fixed()`/`.padded()`/
/// `.grouped()` inherent method — this isn't interop with something that
/// already exists, it's Kittine synthesizing the `format!` call a `.kitty`
/// author would otherwise have to drop into raw Rust for). See
/// [`render_fixed`], [`render_padded`], [`render_grouped`].
fn render_method_call(
    receiver: &Expr,
    method: &str,
    args: &[Expr],
    render: &dyn Fn(&Expr) -> String,
) -> String {
    match (method, args) {
        ("fixed", [precision]) => return render_fixed(receiver, precision, render),
        ("padded", [width]) => return render_padded(receiver, width, render),
        ("grouped", []) => return render_grouped(receiver, render),
        _ => {}
    }
    let rendered_args: Vec<String> = args.iter().map(|a| render(a)).collect();
    format!("{}.{method}({})", render(receiver), rendered_args.join(", "))
}

/// Renders `receiver.fixed(precision)` — a fixed-decimal-places number
/// format, e.g. `price.fixed(2)` on `3.14159` -> `"3.14"`. Lowers to
/// `format!("{:.*}", precision as usize, receiver)`: the `.*` precision
/// specifier consumes the *first* positional argument as the precision and
/// the second as the value, which is what makes a non-literal (a signal
/// read, a `hold` binding) precision argument possible at all — a plain
/// `{:.2}` only accepts a literal. `as usize` tolerates both a bare integer
/// literal (`2`, which renders unsuffixed and infers fine under a cast) and
/// a `Num` (`f64`) expression on the precision side, without Kittine having
/// to know which it is.
fn render_fixed(receiver: &Expr, precision: &Expr, render: &dyn Fn(&Expr) -> String) -> String {
    format!(
        "format!(\"{{:.*}}\", ({}) as usize, ({}))",
        render(precision),
        render(receiver)
    )
}

/// Renders `receiver.padded(width)` — zero-left-pads a value's `Display`
/// text to `width` characters, e.g. `n.padded(2)` on `7` -> `"07"` (clock
/// digits, IDs, anything that needs a stable column width). Lowers to
/// `format!("{:0>1$}", receiver, width as usize)`: `0>` fills with `'0'`
/// right-aligned, and `1$` reads the width from the *second* positional
/// argument, the same dynamic-width mechanism `fixed` uses for precision —
/// a literal `{:02}`-style spec only accepts a compile-time-constant width.
fn render_padded(receiver: &Expr, width: &Expr, render: &dyn Fn(&Expr) -> String) -> String {
    format!(
        "format!(\"{{:0>1$}}\", ({}), ({}) as usize)",
        render(receiver),
        render(width)
    )
}

/// Renders `receiver.grouped()` — thousands-separator formatting, e.g.
/// `1234567.grouped()` -> `"1,234,567"`. Rust's `format!` has no grouping
/// specifier at all (unlike `.*`-precision or `$`-width, there's no macro
/// syntax to reach for), so this lowers to a small self-contained block
/// expression instead of a single `format!` call — still just one Rust
/// *expression*, valid anywhere `render_method_call`'s caller needs one
/// (JSX interpolation, a signal mutation, a call argument), only spanning
/// more than one line of generated source. Splits the `Display` text on an
/// optional leading `-` and an optional `.` (decimal point), groups the
/// integer part into comma-separated 3-digit chunks from the right via
/// `rchunks(3).rev()`, and leaves the sign/decimal part untouched.
fn render_grouped(receiver: &Expr, render: &dyn Fn(&Expr) -> String) -> String {
    format!(
        "{{ let __kittine_v = format!(\"{{}}\", ({receiver})); \
let (__kittine_sign, __kittine_digits): (&str, &str) = match __kittine_v.strip_prefix('-') {{ \
Some(rest) => (\"-\", rest), None => (\"\", __kittine_v.as_str()) }}; \
let (__kittine_int, __kittine_frac): (&str, String) = match __kittine_digits.split_once('.') {{ \
Some((i, f)) => (i, format!(\".{{f}}\")), None => (__kittine_digits, String::new()) }}; \
let __kittine_grouped: String = __kittine_int.as_bytes().rchunks(3).rev()\
.map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(\",\"); \
format!(\"{{__kittine_sign}}{{__kittine_grouped}}{{__kittine_frac}}\") }}",
        receiver = render(receiver)
    )
}

/// Renders `callee(arg, arg, ..)` where `callee` isn't a bare name — see
/// `ast::Expr::CallResult`. Same trust model as [`render_method_call`]:
/// no type information, arguments render plain (no forced `f64`).
fn render_call_result(callee: &Expr, args: &[Expr], render: &dyn Fn(&Expr) -> String) -> String {
    let rendered_args: Vec<String> = args.iter().map(|a| render(a)).collect();
    format!("{}({})", render(callee), rendered_args.join(", "))
}

/// Renders `(a, b, c)` — see `ast::Expr::Tuple`.
fn render_tuple(items: &[Expr], render: &dyn Fn(&Expr) -> String) -> String {
    let rendered: Vec<String> = items.iter().map(|e| render(e)).collect();
    format!("({})", rendered.join(", "))
}

/// Renders `Segment::Segment(::Segment)*` — see `ast::Expr::Path`.
fn render_path(segments: &[String]) -> String {
    segments.join("::")
}

/// Renders `[a, b, c]` as `vec![a, b, c]`.
fn render_array(items: &[Expr], render: &dyn Fn(&Expr) -> String) -> String {
    let rendered: Vec<String> = items.iter().map(|e| render(e)).collect();
    format!("vec![{}]", rendered.join(", "))
}

/// `true` if `expr` (after stripping a `#t` tag) is an array or
/// struct/`stash` literal — used to decide whether `craft<...>` should
/// format with `{:?}` (arrays, generated `litter`/`breed` types, and
/// `HashMap` all have no `Display` impl, only `Debug`) instead of `{}`.
/// Only covers the *literal* shape, same scope limit for a struct/`stash`
/// as for an array: a bare identifier read from an array/struct/`stash`-
/// typed signal or prop still isn't covered (a real, pre-existing,
/// documented gap — see LANGUAGE.md § Known limitations).
fn is_array_like(expr: &Expr) -> bool {
    match expr {
        Expr::Array(_) | Expr::StructInit { .. } => true,
        Expr::Typed { value, .. } => is_array_like(value),
        _ => false,
    }
}

/// `true` if `expr` is a bare string literal — used to decide whether a `+`
/// should lower to Rust's numeric `+` or to string formatting (see
/// [`render_binary`]).
fn is_string_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Str(_))
}

/// Renders a binary expression, special-casing `+` where either side is a
/// string literal: Kittine has no type system to tell a numeric `+` from a
/// string one, so `+` involving a literal string lowers to `format!("{}{}",
/// ..)` instead of Rust's `+` operator (which doesn't accept `&str + &str`,
/// let alone `&str + f64`). This makes `'Taps: ' + <{count}>` produce
/// `"Taps: 5"` by `Display`-formatting both sides, while a plain `x + 1`
/// with no string literal in sight keeps compiling to ordinary numeric
/// addition, unchanged.
fn render_binary(left: &Expr, op: BinOp, right: &Expr, render: &dyn Fn(&Expr) -> String) -> String {
    if op == BinOp::Add && (is_string_literal(left) || is_string_literal(right)) {
        format!("format!(\"{{}}{{}}\", {}, {})", render(left), render(right))
    } else {
        format!(
            "({} {} {})",
            render_arith_operand(left, render),
            op_str(op),
            render_arith_operand(right, render)
        )
    }
}

/// Renders an operand of an arithmetic binary op, forcing a bare whole
/// number literal into an unambiguous float form. Rust infers a plain
/// integer literal as `f64` in a compound-assignment position (`*n += 1`
/// type-checks, which is why [`mutation_body`]'s compound-op special case
/// can use bare literals) but *not* as a general operator-trait operand
/// (`n * 2` fails to compile when `n: f64` with "cannot multiply `f64` by
/// `{integer}`") — so a plain arithmetic expression needs the literal
/// spelled out as unambiguously `f64` instead.
fn render_arith_operand(expr: &Expr, render: &dyn Fn(&Expr) -> String) -> String {
    match expr {
        Expr::Number(n) => fmt_num_unambiguous(*n),
        other => render(other),
    }
}

/// Like [`fmt_num`], but a whole number always carries an explicit `f64`
/// suffix so it type-checks as an operand next to an already-`f64`-typed
/// value (see [`render_arith_operand`]).
fn fmt_num_unambiguous(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}f64", n as i64)
    } else {
        format!("{n}")
    }
}

/// The mutation as a bare expression (no trailing `;`), for use as a JSX
/// event-handler closure body: `move |_| set_x.update(|n| ..)`.
fn mutation_expr(name: &str, value: &Expr, scope: &Scope) -> String {
    let body = mutation_body(name, value, scope);
    format!("set_{name}.update(|n| {body})")
}

// ---- expressions -----------------------------------------------------------

/// Reads a name that isn't a signal mutation target: a declared signal
/// becomes `name.get()`. A non-`Copy`-typed parameter (`Word`, or any
/// array type) is cloned, since Leptos view closures may capture the same
/// prop more than once; a view-position `spin`'s loop variable and a
/// `hold`-bound local are *always* cloned, regardless of type — a
/// `{move || item}` reactive closure needs to be callable more than once
/// (`Fn`, not just `FnOnce`), which requires not moving a non-`Copy` value
/// out of it, and `.clone()`ing a `Copy` type (e.g. `Num`) costs nothing
/// extra, so there's no reason to special-case per element/value type
/// here. Anything else (`Num`/`Flag` params, plain locals) is read bare.
fn render_var_read(name: &str, scope: &Scope) -> String {
    if scope.closure_params.contains(name) {
        name.to_string()
    } else if scope.declared.contains(name) {
        format!("{name}.get()")
    } else if scope.spin_items.contains(name)
        || scope.hold_items.contains(name)
        || scope
            .params
            .get(name)
            .is_some_and(|ty| is_non_copy_param_type(ty))
    {
        format!("{name}.clone()")
    } else {
        name.to_string()
    }
}

fn expr_to_rust(expr: &Expr, scope: &Scope) -> String {
    match expr {
        Expr::Ident(name) => render_ident(name, scope),
        Expr::VarRead(name) => render_var_read(name, scope),
        Expr::Number(n) => fmt_num(*n),
        // Deliberately a bare `&str` here, NOT owned via `.to_string()`:
        // unlike a signal initializer (see `render_signal_init`), a
        // string literal in general expression position (a function-call
        // argument, a `craft<...>` argument) might need to be `&str`
        // (`leptos_router::StaticSegment` requires `T: AsPath`, which
        // `String` does not implement — only `&str` does) or an owned
        // `String` (a `purr` parameter typed `Word`), depending entirely
        // on the callee — information Kittine doesn't have. Forcing one
        // choice broke the other case when tried; see `docs/LANGUAGE.md`
        // § Known limitations — passing a string *literal* directly to a
        // `Word`-typed `purr` parameter doesn't work yet, only passing a
        // `Word` signal/prop does (those already resolve to owned
        // `String` via `render_signal_init`/prop cloning).
        Expr::Str(s) => format!("\"{}\"", escape_str(s)),
        Expr::Binary { left, op, right } => {
            render_binary(left, *op, right, &|e| expr_to_rust(e, scope))
        }
        Expr::InlineAssign { name, value } => mutation_expr(name, value, scope),
        Expr::Bool(b) => b.to_string(),
        Expr::Array(items) => render_array(items, &|e| expr_to_rust(e, scope)),
        Expr::Typed { value, .. } => expr_to_rust(value, scope),
        Expr::Call { name, args } => {
            render_call_or_variant(name, args, &|e| expr_to_rust(e, scope), scope)
        }
        Expr::MethodCall { receiver, method, args } => {
            render_method_call(receiver, method, args, &|e| expr_to_rust(e, scope))
        }
        Expr::CallResult { callee, args } => {
            render_call_result(callee, args, &|e| expr_to_rust(e, scope))
        }
        Expr::Tuple(items) => render_tuple(items, &|e| expr_to_rust(e, scope)),
        Expr::Path(segments) => render_path(segments),
        Expr::FieldAccess { receiver, field } => {
            format!("{}.{field}", expr_to_rust(receiver, scope))
        }
        Expr::StructInit { name, fields } => {
            render_struct_init(name, fields, &|e| expr_to_rust(e, scope), scope)
        }
        Expr::Ref(inner) => format!("&{}", expr_to_rust(inner, scope)),
        Expr::Pounce {
            subject,
            arms,
            catch_all,
        } => render_pounce_expr(subject, arms, catch_all, scope, &expr_to_rust),
        Expr::Closure { params, body } => render_closure(params, body, scope, &expr_to_rust),
    }
}

/// Renders `pounce> subject (Variant(binding)? >> expr)+ (else> expr)?` —
/// see `ast::Expr::Pounce` — as a Rust `match` expression, reusing the
/// same variant -> owning-`breed` lookup and payload-binding pattern
/// `gen_stmt`'s `Stmt::Pounce` arm already does, just producing a value per
/// arm (`pattern => expr`) instead of running a statement. `render` renders
/// both the subject and every arm/catch-all body against a possibly
/// different `Scope` per arm (the payload binding, if any, is only in
/// scope for its own arm) — passed in rather than hardcoded so this same
/// function backs both `substitute_self` (self-referencing var-assign
/// context) and the ordinary `expr_to_rust` dispatch.
fn render_pounce_expr(
    subject: &Expr,
    arms: &[PounceExprArm],
    catch_all: &Option<Box<Expr>>,
    scope: &Scope,
    render: &dyn Fn(&Expr, &Scope) -> String,
) -> String {
    render_pounce_expr_impl(subject, arms, catch_all, scope, render, false)
}

/// Like [`render_pounce_expr`], but for a `Word`-returning `purr`'s
/// `return (..)` value specifically: gives each arm the same bare-string-
/// literal -> owned-`String` coercion [`render_return_value`] already gives
/// a *non*-`pounce>` return value (see [`is_bare_string_literal`]). Needed
/// because a real Rust `match` requires every arm to produce the *same*
/// type — one arm computing an owned `String` (e.g. `format!(..)`) while a
/// sibling arm is a bare, uncoerced `&'static str` literal (`else> 'idle'`)
/// is a real `E0308` once generated, confirmed by actually compiling that
/// exact pattern against rustc, not assumed.
fn render_pounce_expr_word_owned(
    subject: &Expr,
    arms: &[PounceExprArm],
    catch_all: &Option<Box<Expr>>,
    scope: &Scope,
    render: &dyn Fn(&Expr, &Scope) -> String,
) -> String {
    render_pounce_expr_impl(subject, arms, catch_all, scope, render, true)
}

fn render_pounce_expr_impl(
    subject: &Expr,
    arms: &[PounceExprArm],
    catch_all: &Option<Box<Expr>>,
    scope: &Scope,
    render: &dyn Fn(&Expr, &Scope) -> String,
    word_owned: bool,
) -> String {
    let render_arm_body = |body: &Expr, arm_scope: &Scope| {
        if word_owned && is_bare_string_literal(body) {
            format!("{}.to_string()", render(body, arm_scope))
        } else {
            render(body, arm_scope)
        }
    };
    let subject_code = render(subject, scope);
    let mut parts: Vec<String> = arms
        .iter()
        .map(|arm| {
            let breed = scope
                .signatures
                .variants
                .get(&arm.variant)
                .map(|v| v.breed.as_str())
                .unwrap_or(&arm.variant);
            let pattern = match &arm.binding {
                Some(binding) => format!("{breed}::{}({binding})", arm.variant),
                None => format!("{breed}::{}", arm.variant),
            };
            let mut arm_scope = scope.clone();
            if let Some(binding) = &arm.binding {
                arm_scope.hold_items.insert(binding.clone());
            }
            format!("{pattern} => {}", render_arm_body(&arm.body, &arm_scope))
        })
        .collect();
    if let Some(catch_all) = catch_all {
        parts.push(format!("_ => {}", render_arm_body(catch_all, scope)));
    }
    format!("match {subject_code} {{ {} }}", parts.join(", "))
}

/// Renders `|param, ..| expr` verbatim as a Rust closure — see
/// `ast::Expr::Closure`. Each `param` is bound (untyped — Rust infers it
/// from how the closure is used, e.g. `Vec::filter`'s `&T`) for rendering
/// `body`, the same "arm-local binding" treatment `render_pounce_expr`
/// gives a payload binding: added to a cloned `Scope`'s `hold_items` so a
/// closure param that happens to shadow an outer non-`Copy` name still
/// reads bare inside the closure instead of picking up an unrelated
/// `.clone()`.
fn render_closure(
    params: &[String],
    body: &Expr,
    scope: &Scope,
    render: &dyn Fn(&Expr, &Scope) -> String,
) -> String {
    let closure_scope = scope.with_closure_params(params);
    let params_str = params.join(", ");
    format!("|{params_str}| {}", render(body, &closure_scope))
}

/// Renders a top-level `if>` condition without the redundant outer
/// parentheses `expr_to_rust` would otherwise add around every `Binary`.
fn render_condition(expr: &Expr, scope: &Scope) -> String {
    render_top_level(expr, scope)
}

/// Renders an expression as a top-level tail/return value: a nested
/// operand of another expression needs parens for precedence, but the
/// outermost expression in a block's tail position (a `purr` return, an
/// `if>` condition) doesn't — and Rust warns about the redundant parens if
/// they're there anyway. Falls through to `expr_to_rust` for the
/// string-concatenation `+` case, whose `format!(..)` call needs no
/// unwrapping in the first place.
fn render_top_level(expr: &Expr, scope: &Scope) -> String {
    match expr {
        Expr::Binary { left, op, right }
            if !(*op == BinOp::Add && (is_string_literal(left) || is_string_literal(right))) =>
        {
            format!(
                "{} {} {}",
                render_arith_operand(left, &|e| expr_to_rust(e, scope)),
                op_str(*op),
                render_arith_operand(right, &|e| expr_to_rust(e, scope))
            )
        }
        other => expr_to_rust(other, scope),
    }
}

/// Renders a `purr`/`bare`-method's `return (expr)` value, additionally
/// coercing a bare `Word`-typed string literal to an owned `String` (see
/// [`is_bare_string_literal`]) — [`render_top_level`] alone doesn't know
/// the function's own declared return type, so `purr constant() #w {
/// return ('hi') }` would otherwise render `"hi"` verbatim: an
/// uncoerced `&'static str` that doesn't type-check against the `String`
/// the signature promises (`E0308: expected String, found &str` —
/// confirmed by actually compiling that exact pattern against rustc, the
/// same string-concatenation `+` already got this treatment for, just
/// extended to a bare literal with no `+` in sight).
fn render_return_value(expr: &Expr, return_type: &str, scope: &Scope) -> String {
    if return_type == "Word" && is_bare_string_literal(expr) {
        format!("{}.to_string()", render_top_level(expr, scope))
    } else if return_type == "Word"
        && let Expr::Pounce {
            subject,
            arms,
            catch_all,
        } = expr
    {
        render_pounce_expr_word_owned(subject, arms, catch_all, scope, &expr_to_rust)
    } else {
        render_top_level(expr, scope)
    }
}

fn op_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Eq => "==",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::Ne => "!=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

// ---- JSX --------------------------------------------------------------------

/// A tag beginning with an uppercase letter is a reference to another
/// Kittine component (local or imported), exactly like Leptos's own
/// `view!` macro distinguishes components from HTML elements.
fn is_component_tag(tag: &str) -> bool {
    tag.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// `true` for exactly `{ children() }` — a call to the special
/// zero-argument `children` parameter, which needs bare (non-reactive)
/// rendering. See the `JsxNode::ExprInterp` arm of `jsx_to_rust`.
fn is_children_call(expr: &Expr) -> bool {
    matches!(expr, Expr::Call { name, args } if name == "children" && args.is_empty())
}

/// `true` if `expr` reads the reserved `event` identifier anywhere in its
/// tree — used only for an `on<Event>` handler's own expression, to decide
/// whether that handler's closure needs to actually bind the event
/// (`|ev|`, with `event` rendering as `event_target_value(&ev)` — see
/// `Scope::event_bound`) instead of discarding it (`|_|`, the default for
/// every handler that doesn't need the event's value, e.g. `onClick={<{count}>
/// >> count + 1}`).
fn expr_references_event(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(n) => n == "event",
        Expr::VarRead(_) | Expr::Number(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Path(_) => false,
        Expr::Binary { left, right, .. } => {
            expr_references_event(left) || expr_references_event(right)
        }
        Expr::InlineAssign { value, .. } => expr_references_event(value),
        Expr::Array(items) | Expr::Tuple(items) => items.iter().any(expr_references_event),
        Expr::Typed { value, .. } => expr_references_event(value),
        Expr::Call { args, .. } => args.iter().any(expr_references_event),
        Expr::MethodCall { receiver, args, .. } => {
            expr_references_event(receiver) || args.iter().any(expr_references_event)
        }
        Expr::CallResult { callee, args } => {
            expr_references_event(callee) || args.iter().any(expr_references_event)
        }
        Expr::FieldAccess { receiver, .. } => expr_references_event(receiver),
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, v)| expr_references_event(v)),
        Expr::Ref(inner) => expr_references_event(inner),
        Expr::Pounce {
            subject,
            arms,
            catch_all,
        } => {
            expr_references_event(subject)
                || arms.iter().any(|a| expr_references_event(&a.body))
                || catch_all.as_deref().is_some_and(expr_references_event)
        }
        // A closure's own params always shadow the reserved `event` name
        // within its body — but Kittine has no nested-`on<Event>`-inside-a-
        // closure use case today, so treating the body as never referencing
        // the *outer* handler's `event` (the only thing this check is for)
        // is the correct, simple answer, not a shortcut.
        Expr::Closure { .. } => false,
    }
}

fn jsx_attr_leptos_name(name: &str) -> (String, bool) {
    let bytes = name.as_bytes();
    if name.len() > 2 && &name[0..2] == "on" && bytes[2].is_ascii_uppercase() {
        let event = name[2..].to_lowercase();
        (format!("on:{event}"), true)
    } else {
        (name.to_string(), false)
    }
}

/// Collects every distinct name referenced anywhere in `expr` that's a
/// non-`Copy` scope-tracked value — a `Word`/array prop, a view-position
/// `spin`'s loop variable, or a `hold` binding (see
/// [`wrap_reactive_closure`] for why this matters). Order is
/// first-encountered, deduplicated.
fn non_copy_tracked_names(expr: &Expr, scope: &Scope, out: &mut Vec<String>) {
    let note = |name: &str, out: &mut Vec<String>| {
        let is_tracked = scope.spin_items.contains(name)
            || scope.hold_items.contains(name)
            || scope
                .params
                .get(name)
                .is_some_and(|ty| is_non_copy_param_type(ty));
        if is_tracked && !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    };
    match expr {
        Expr::Ident(name) | Expr::VarRead(name) => note(name, out),
        Expr::Number(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Path(_) => {}
        Expr::Binary { left, right, .. } => {
            non_copy_tracked_names(left, scope, out);
            non_copy_tracked_names(right, scope, out);
        }
        Expr::InlineAssign { value, .. } => non_copy_tracked_names(value, scope, out),
        Expr::Array(items) | Expr::Tuple(items) => {
            for item in items {
                non_copy_tracked_names(item, scope, out);
            }
        }
        Expr::Typed { value, .. } => non_copy_tracked_names(value, scope, out),
        Expr::Call { name, args } => {
            // The callee itself matters, not just the arguments — a
            // `hold`-bound closure called by bare name (`navigate(..)`,
            // see `render_call`'s own `.clone()` on the callee) is exactly
            // as much a "read of a non-Copy tracked name" as using it any
            // other way.
            note(name, out);
            for arg in args {
                non_copy_tracked_names(arg, scope, out);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            non_copy_tracked_names(receiver, scope, out);
            for arg in args {
                non_copy_tracked_names(arg, scope, out);
            }
        }
        Expr::CallResult { callee, args } => {
            non_copy_tracked_names(callee, scope, out);
            for arg in args {
                non_copy_tracked_names(arg, scope, out);
            }
        }
        Expr::FieldAccess { receiver, .. } => non_copy_tracked_names(receiver, scope, out),
        Expr::StructInit { fields, .. } => {
            for (_, value) in fields {
                non_copy_tracked_names(value, scope, out);
            }
        }
        Expr::Ref(inner) => non_copy_tracked_names(inner, scope, out),
        Expr::Pounce {
            subject,
            arms,
            catch_all,
        } => {
            non_copy_tracked_names(subject, scope, out);
            for arm in arms {
                non_copy_tracked_names(&arm.body, scope, out);
            }
            if let Some(catch_all) = catch_all {
                non_copy_tracked_names(catch_all, scope, out);
            }
        }
        Expr::Closure { body, .. } => non_copy_tracked_names(body, scope, out),
    }
}

/// Wraps `body_code` (already-rendered Rust) in a `move CLOSURE_PARAMS
/// body_code` closure — pre-cloning, into their own local bindings just
/// before the closure, any non-`Copy` scope-tracked names `source_expr`
/// references.
///
/// This matters because a `move` closure captures every variable it uses
/// *by value*, not by reference — including ones only ever read via
/// `.clone()` inside the closure body. If the same original prop/spin-item
/// /`hold` value is read from a *second*, sibling closure elsewhere in the
/// same component (a name shown in two places, or a `hold`-bound closure
/// called from two event handlers), the second `move` closure tries to
/// move the same already-moved original and fails to compile (`E0382: use
/// of moved value`) — confirmed by actually compiling exactly that pattern
/// against real Leptos, not assumed. Pre-cloning into a fresh local first
/// gives *each* closure its own independently-owned copy to move, which is
/// also literally what rustc's own E0382 diagnostic suggests for this
/// shape of error.
fn wrap_reactive_closure(
    closure_params: &str,
    source_expr: &Expr,
    body_code: &str,
    scope: &Scope,
) -> String {
    let mut names = Vec::new();
    non_copy_tracked_names(source_expr, scope, &mut names);
    if names.is_empty() {
        format!("move {closure_params} {body_code}")
    } else {
        let pre_clones: String = names
            .iter()
            .map(|n| format!("let {n} = {n}.clone(); "))
            .collect();
        format!("{{ {pre_clones}move {closure_params} {body_code} }}")
    }
}

fn jsx_attr_value(value: &JsxAttrValue, is_event: bool, is_component: bool, scope: &Scope) -> String {
    match value {
        // A component prop typed `Word` is a concrete `String`, so a
        // string-literal prop value needs `.to_string()` — a plain HTML
        // attribute stays a bare `&str`, which `view!` accepts as-is.
        JsxAttrValue::Str(s) if is_component => {
            format!("\"{}\".to_string()", escape_str(s))
        }
        JsxAttrValue::Str(s) => format!("\"{}\"", escape_str(s)),
        JsxAttrValue::Expr(Expr::InlineAssign { name, value }) => {
            let needs_event = is_event && expr_references_event(value);
            let event_scope = scope.with_event_bound();
            let render_scope = if needs_event { &event_scope } else { scope };
            let body = mutation_expr(name, value, render_scope);
            let closure_params = if needs_event { "|ev|" } else { "|_|" };
            wrap_reactive_closure(closure_params, value, &body, scope)
        }
        JsxAttrValue::Expr(expr) if is_event => {
            let needs_event = expr_references_event(expr);
            let event_scope = scope.with_event_bound();
            let render_scope = if needs_event { &event_scope } else { scope };
            let body = expr_to_rust(expr, render_scope);
            let closure_params = if needs_event { "|ev|" } else { "|_|" };
            wrap_reactive_closure(closure_params, expr, &body, scope)
        }
        // Component props are plain typed values, not reactive DOM
        // attributes — pass them through bare rather than wrapping in a
        // `move || ..` tracking closure.
        JsxAttrValue::Expr(expr) if is_component => expr_to_rust(expr, scope),
        JsxAttrValue::Expr(expr) => {
            let body = expr_to_rust(expr, scope);
            wrap_reactive_closure("||", expr, &body, scope)
        }
    }
}

fn jsx_to_rust(node: &JsxNode, scope: &Scope, indent: usize) -> String {
    let mut out = String::new();
    match node {
        JsxNode::Text(s) => {
            push_line(&mut out, indent, &format!("\"{}\"", escape_str(s)));
        }
        JsxNode::VarInterp(name) => {
            let var_read_expr = Expr::VarRead(name.clone());
            let body = render_var_read(name, scope);
            let closure = wrap_reactive_closure("||", &var_read_expr, &body, scope);
            push_line(&mut out, indent, &format!("{{{closure}}}"));
        }
        JsxNode::ExprInterp(expr) if is_children_call(expr) => {
            // `Children` is `FnOnce() -> Fragment` — call it bare, exactly
            // once, matching Leptos's own idiom. Wrapping it in `move || ..`
            // like a normal reactive interpolation would either move it out
            // from under a closure Leptos might invoke more than once, or
            // just be pointless indirection for content that doesn't change.
            push_line(&mut out, indent, &format!("{{{}}}", expr_to_rust(expr, scope)));
        }
        JsxNode::ExprInterp(expr) => {
            let body = expr_to_rust(expr, scope);
            let closure = wrap_reactive_closure("||", expr, &body, scope);
            push_line(&mut out, indent, &format!("{{{closure}}}"));
        }
        JsxNode::Spin { item, list, key, body } => {
            let list_code = expr_to_rust(list, scope);
            // `item` needs to render as `item.clone()` inside both the key
            // expression and the body's reactive closures (see
            // `render_var_read`) — Leptos's `key` closure receives `&T`
            // (see `<For>`'s `KF: Fn(&T) -> K`), and `.clone()` on a `&T`
            // still yields an owned `T`, so the same rendering is correct
            // in both positions.
            let body_scope = scope.with_spin_item(item);
            let key_code = match key {
                Some(key_expr) => render_top_level(key_expr, &body_scope),
                // `{:?}` (Debug), not `{}` (Display): every element type
                // `spin` can iterate now includes a `litter`/`breed` (see
                // `ast::Expr::Pounce`'s sibling gap closed alongside it,
                // array-of-litter types), and `gen_litter`/`gen_breed`
                // only ever derive `Debug`, not `Display` — deriving the
                // latter isn't something `#[derive(..)]` can do at all for
                // an arbitrary struct. `Debug` is universal (every scalar
                // Kittine can produce implements it too), so this default
                // now works for every list-element type instead of only
                // the three scalars, at the cost of a quoted `"like
                // this"`/`5.0`-shaped key string instead of a bare one —
                // harmless, since a `<For>` key only needs to be unique
                // and stable, never user-visible.
                None => format!("format!(\"{{{item}:?}}\")"),
            };
            push_line(
                &mut out,
                indent,
                &format!("<For each=move || {list_code} key=|{item}| {key_code} let:{item}>"),
            );
            for child in body {
                out.push_str(&jsx_to_rust(child, &body_scope, indent + 1));
            }
            push_line(&mut out, indent, "</For>");
        }
        JsxNode::Element {
            tag,
            attrs,
            children,
            self_closing,
        } => {
            let is_component = is_component_tag(tag);
            let attrs_str: String = attrs
                .iter()
                .map(|(name, value)| {
                    let (leptos_name, is_event) = if is_component {
                        (name.clone(), false)
                    } else {
                        jsx_attr_leptos_name(name)
                    };
                    format!(
                        " {}={}",
                        leptos_name,
                        jsx_attr_value(value, is_event, is_component, scope)
                    )
                })
                .collect();

            if *self_closing {
                push_line(&mut out, indent, &format!("<{tag}{attrs_str}/>"));
            } else if children.is_empty() {
                push_line(&mut out, indent, &format!("<{tag}{attrs_str}></{tag}>"));
            } else {
                push_line(&mut out, indent, &format!("<{tag}{attrs_str}>"));
                for child in children {
                    out.push_str(&jsx_to_rust(child, scope, indent + 1));
                }
                push_line(&mut out, indent, &format!("</{tag}>"));
            }
        }
    }
    out
}
