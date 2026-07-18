//! End-to-end tests exercising the full lex -> parse -> codegen pipeline
//! against the exact syntax shown in the Kittine language spec.

use crate::codegen;
use crate::lexer;
use crate::parser;

fn compile(src: &str) -> String {
    let tokens = lexer::tokenize(src).expect("lex should succeed");
    let program = parser::parse(tokens).expect("parse should succeed");
    codegen::generate(&program)
}

/// Returns the parser error message for source that should fail to parse.
fn compile_err(src: &str) -> String {
    let tokens = lexer::tokenize(src).expect("lex should succeed");
    match parser::parse(tokens) {
        Ok(_) => panic!("expected a parse error, but parsing succeeded"),
        Err(e) => e.message,
    }
}

#[test]
fn var_init_lowers_to_signal() {
    let out = compile(
        r#"
func App() {
    <{count}> >> 0
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("let (count, set_count) = signal(0f64);"));
    assert!(out.contains("use leptos::prelude::*;"));
}

#[test]
fn string_literal_uses_single_or_double_quotes() {
    let out = compile(
        r#"
func App() {
    <{username}> >> 'Admin'
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"let (username, set_username) = signal("Admin");"#));

    let out2 = compile(
        r#"
func App() {
    <{username}> >> "Admin"
    return ( <div></div> )
}
"#,
    );
    assert!(out2.contains(r#"let (username, set_username) = signal("Admin");"#));
}

#[test]
fn craft_lowers_to_leptos_log() {
    let out = compile(
        r#"
func App() {
    craft<'hello world'>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"leptos::logging::log!("hello world");"#));
}

#[test]
fn mutation_after_declaration_uses_update() {
    let out = compile(
        r#"
func App() {
    <{count}> >> 0
    <{count}> >> count + 1
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("let (count, set_count) = signal(0f64);"));
    assert!(out.contains("set_count.update(|n| *n += 1f64);"));
}

#[test]
fn if_orif_else_chain() {
    let out = compile(
        r#"
func App() {
    <{username}> >> 'Admin'

    if><{username}> >> 'Admin'
        craft<'Welcome Admin'>
    orif><{username}> >> 'User'
        craft<'Welcome User'>
    else>
        craft<'no output'>

    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"if username.get() == "Admin" {"#));
    assert!(out.contains(r#"} else if username.get() == "User" {"#));
    assert!(out.contains("} else {"));
    assert!(out.contains(r#"leptos::logging::log!("Welcome Admin");"#));
    assert!(out.contains(r#"leptos::logging::log!("Welcome User");"#));
    assert!(out.contains(r#"leptos::logging::log!("no output");"#));
}

#[test]
fn jsx_button_counter_with_event_handler() {
    let out = compile(
        r#"
func App() {
    <{count}> >> 0
    return (
        <div>
            <button onClick={<{count}> >> count + 1}>
                "Clicks: "
                <{count}>
            </button>
        </div>
    )
}
"#,
    );
    assert!(out.contains("view! {"));
    assert!(out.contains("<div>"));
    assert!(out.contains("on:click=move |_| set_count.update(|n| *n += 1f64)"));
    assert!(out.contains(r#""Clicks: ""#));
    assert!(out.contains("{move || count.get()}"));
}

#[test]
fn self_closing_jsx_element() {
    let out = compile(
        r#"
func App() {
    return ( <input/> )
}
"#,
    );
    assert!(out.contains("<input/>"));
}

#[test]
fn string_literal_plus_lowers_to_format() {
    let out = compile(
        r#"
func App() {
    <{count}> >> 0
    craft<'Taps: ' + <{count}> >
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"format!("{}{}", "Taps: ", count.get())"#));
}

#[test]
fn numeric_plus_is_unaffected_by_string_concat() {
    let out = compile(
        r#"
func App() {
    <{count}> >> 0
    <{count}> >> count + 1
    return ( <div></div> )
}
"#,
    );
    assert!(!out.contains("format!"));
    assert!(out.contains("set_count.update(|n| *n += 1f64);"));
}

#[test]
fn string_concat_in_jsx_expr_interpolation() {
    let out = compile(
        r#"
func App() {
    <{mood}> >> 'Curious'
    return (
        <div>
            <p>{ 'Mood: ' + <{mood}> }</p>
        </div>
    )
}
"#,
    );
    assert!(out.contains(r#"{move || format!("{}{}", "Mood: ", mood.get())}"#));
}

#[test]
fn string_concat_in_mutation() {
    let out = compile(
        r#"
func App() {
    <{label}> >> 'Taps: 0'
    <{label}> >> 'Taps: ' + <{label}>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"set_label.update(|n| *n = format!("{}{}", "Taps: ", *n));"#));
}

#[test]
fn nested_if_inside_if_block() {
    let out = compile(
        r#"
func App() {
    <{a}> >> 0
    <{b}> >> 0

    if><{a}> >> 0
        if><{b}> >> 0
            craft<'both zero'>
        else>
            craft<'only a'>
    else>
        craft<'not a'>

    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"leptos::logging::log!("both zero");"#));
    assert!(out.contains(r#"leptos::logging::log!("only a");"#));
    assert!(out.contains(r#"leptos::logging::log!("not a");"#));
}

#[test]
fn bool_literal_lowers_to_rust_bool() {
    let out = compile(
        r#"
func App() {
    <{ready}> >> yes>
    <{done}> >> no>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("let (ready, set_ready) = signal(true);"));
    assert!(out.contains("let (done, set_done) = signal(false);"));
}

#[test]
fn array_literal_lowers_to_vec_macro() {
    let out = compile(
        r#"
func App() {
    <{scores}> >> [1, 2, 3]
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("let (scores, set_scores) = signal(vec![1, 2, 3]);"));
}

#[test]
fn craft_array_uses_debug_format() {
    let out = compile(
        r#"
func App() {
    craft<[1, 2, 3]>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"leptos::logging::log!("{:?}", vec![1, 2, 3]);"#));
}

#[test]
fn type_tag_erases_to_bare_value() {
    let out = compile(
        r#"
func App() {
    <{count}> >> <<Num>> 0
    <{label}> >> <<Word>> 'hi'
    <{ready}> >> <<Flag>> yes>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("let (count, set_count) = signal(0f64);"));
    assert!(out.contains(r#"let (label, set_label) = signal("hi");"#));
    assert!(out.contains("let (ready, set_ready) = signal(true);"));
}

#[test]
fn type_tag_accepts_a_dynamic_value() {
    let out = compile(
        r#"
func App() {
    <{count}> >> 0
    <{doubled}> >> <<Num>> count
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("let (doubled, set_doubled) = signal(count.get());"));
}

#[test]
fn mismatched_type_tag_is_a_parse_error() {
    let message = compile_err(
        r#"
func App() {
    <{count}> >> <<Num>> 'oops'
    return ( <div></div> )
}
"#,
    );
    assert!(message.contains("does not match"));
}

#[test]
fn unknown_type_tag_is_a_parse_error() {
    let message = compile_err(
        r#"
func App() {
    <{count}> >> <<Nope>> 0
    return ( <div></div> )
}
"#,
    );
    assert!(message.contains("unknown type tag"));
}

#[test]
fn spin_loop_lowers_to_for_loop() {
    let out = compile(
        r#"
func App() {
    spin<{score}> in [1, 2, 3] }{
        craft<score>
    }{
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("for score in (vec![1, 2, 3]).into_iter() {"));
    assert!(out.contains(r#"leptos::logging::log!("{}", score);"#));
}

#[test]
fn component_with_typed_prop() {
    let out = compile(
        r#"
func Nav(<<Word>> active) {
    return ( <span>{ active }</span> )
}
"#,
    );
    assert!(out.contains("pub fn Nav(active: String) -> impl IntoView"));
    // A Word-typed prop is cloned at read sites, since it isn't `Copy` and
    // Leptos view closures may capture it more than once.
    assert!(out.contains("{move || active.clone()}"));
}

#[test]
fn purr_function_with_params_and_return() {
    let out = compile(
        r#"
purr double(<<Num>> n) <<Num>> {
    return (n * 2)
}
"#,
    );
    assert!(out.contains("pub fn double(n: f64) -> f64 {"));
    // No `#[component]`, no view! — a purr function returns a plain value.
    assert!(!out.contains("#[component]"));
    assert!(!out.contains("view!"));
    // Top-level tail expression: no redundant wrapping parens, and the
    // literal `2` is spelled unambiguously as `f64` (bare `2` next to an
    // `f64` variable doesn't type-check as a general arithmetic operand).
    assert!(out.contains("n * 2f64"));
    assert!(!out.contains("(n * 2f64)"));
}

#[test]
fn component_composition_passes_props_as_plain_values() {
    let out = compile(
        r#"
func Nav(<<Word>> active) {
    return ( <span>{ active }</span> )
}

func Home() {
    return ( <div><Nav active='home'/></div> )
}
"#,
    );
    // A component tag (PascalCase) gets its string-literal prop converted
    // to an owned `String` and passed bare, not wrapped in a `move || ..`
    // reactive-attribute closure the way a real HTML element attribute is.
    assert!(out.contains(r#"<Nav active="home".to_string()/>"#));
}

#[test]
fn html_element_attrs_still_use_reactive_closures() {
    let out = compile(
        r#"
func App() {
    <{label}> >> 'hi'
    return ( <div title={label}></div> )
}
"#,
    );
    // Lowercase tags are plain HTML elements: expr-valued attributes still
    // get the `move || ..` wrapper Leptos expects for reactive attributes.
    assert!(out.contains("title=move || label.get()"));
}

#[test]
fn function_call_with_literal_argument_is_unambiguous_f64() {
    let out = compile(
        r#"
purr double(<<Num>> n) <<Num>> {
    return (n * 2)
}

func App() {
    craft<double(21)>
    return ( <div></div> )
}
"#,
    );
    // A bare literal argument doesn't type-check against an `f64`
    // parameter without an explicit suffix, same as a signal initializer
    // or an arithmetic operand.
    assert!(out.contains("double(21f64)"));
}

#[test]
fn function_call_renders_as_rust_call() {
    let out = compile(
        r#"
purr double(<<Num>> n) <<Num>> {
    return (n * 2)
}

func App() {
    <{count}> >> 0
    return ( <div>{ double(count) }</div> )
}
"#,
    );
    assert!(out.contains("{move || double(count.get())}"));
}

#[test]
fn cli_build_resolves_imports_recursively() {
    let dir = std::env::temp_dir().join(format!(
        "kittine-import-test-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    std::fs::write(
        dir.join("Nav.kitty"),
        r#"
func Nav(<<Word>> active) {
    return ( <span>{ active }</span> )
}
"#,
    )
    .expect("write Nav.kitty");

    std::fs::write(
        dir.join("Home.kitty"),
        r#"
import { Nav } from './Nav.kitty'

func Home() {
    return ( <div><Nav active='home'/></div> )
}
"#,
    )
    .expect("write Home.kitty");

    let home_kitty = dir.join("Home.kitty");
    let out_path = crate::build(&home_kitty, None).expect("recursive build should succeed");

    let home_rs = std::fs::read_to_string(&out_path).expect("read generated Home.rs");
    assert!(home_rs.contains("mod __kittine_mod_nav;"));
    assert!(home_rs.contains(r#"<Nav active="home".to_string()/>"#));

    let nav_rs = std::fs::read_to_string(dir.join("Nav.rs")).expect("Nav.rs should be generated");
    assert!(nav_rs.contains("pub fn Nav(active: String) -> impl IntoView"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn import_emits_module_and_use_declaration() {
    let out = compile(
        r#"
import { Nav } from './Nav.kitty'

func Home() {
    return ( <div><Nav/></div> )
}
"#,
    );
    assert!(out.contains(r#"#[path = "./Nav.rs"]"#));
    assert!(out.contains("mod __kittine_mod_nav;"));
    assert!(out.contains("use __kittine_mod_nav::{Nav};"));
}

#[test]
fn spin_loop_over_declared_variable() {
    let out = compile(
        r#"
func App() {
    <{scores}> >> [1, 2, 3]
    spin<{score}> in <{scores}> }{
        craft<score>
    }{
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("for score in (scores.get()).into_iter() {"));
}

#[test]
fn spin_in_view_lowers_to_leptos_for() {
    let out = compile(
        r#"
func List() {
    <{items}> >> [1, 2, 3]
    return (
        <ul>
            spin<{n}> in <{items}> }{
                <li>{ n }</li>
            }{
        </ul>
    )
}
"#,
    );
    assert!(out.contains(r#"<For each=move || items.get() key=|n| format!("{n}") let:n>"#));
    assert!(out.contains("</For>"));
    assert!(out.contains("{move || n}"));
}

#[test]
fn spin_in_view_over_array_literal() {
    let out = compile(
        r#"
func List() {
    return (
        <ul>
            spin<{n}> in [1, 2, 3] }{
                <li>{ n }</li>
            }{
        </ul>
    )
}
"#,
    );
    assert!(out.contains(r#"<For each=move || vec![1, 2, 3] key=|n| format!("{n}") let:n>"#));
}

#[test]
fn spin_in_view_supports_multiple_children() {
    let out = compile(
        r#"
func List() {
    return (
        <ul>
            spin<{n}> in [1, 2] }{
                <li>"Item: "</li>
                <li>{ n }</li>
            }{
        </ul>
    )
}
"#,
    );
    // Both sibling <li>s inside one iteration should be emitted, not just
    // the last one.
    assert!(out.contains(r#""Item: ""#));
    assert!(out.contains("{move || n}"));
}

#[test]
fn component_with_children_param() {
    let out = compile(
        r#"
func Card(<<Word>> title, children) {
    return (
        <div>
            <h2>{ title }</h2>
            { children() }
        </div>
    )
}
"#,
    );
    assert!(out.contains("pub fn Card(title: String, children: Children) -> impl IntoView"));
    // children() is called bare, not wrapped in `move || ..` like a normal
    // reactive interpolation.
    assert!(out.contains("{children()}"));
    assert!(!out.contains("{move || children()}"));
}

#[test]
fn composing_with_children_passes_nested_jsx_through() {
    let out = compile(
        r#"
func Card(children) {
    return ( <div>{ children() }</div> )
}

func Page() {
    return (
        <Card>
            <p>"hello"</p>
        </Card>
    )
}
"#,
    );
    // Leptos's view! macro wires nested JSX content into the `children`
    // prop automatically — Kittine doesn't need to emit an explicit
    // `children=` attribute, just pass the content through as-is.
    assert!(out.contains("<Card>"));
    assert!(out.contains(r#""hello""#));
    assert!(out.contains("</Card>"));
}

#[test]
fn generated_file_brings_leptos_router_into_scope() {
    // Kittine has no routing syntax of its own — <Router>/<Routes>/<Route>/<A>
    // are just ordinary component composition, and StaticSegment(..) is a
    // plain function call. Both already work through existing codegen; all
    // that's needed is leptos_router in scope everywhere, unconditionally.
    let out = compile(
        r#"
func App() {
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("use leptos_router::components::*;"));
    assert!(out.contains("use leptos_router::*;"));
    assert!(out.contains("unused_imports"));
}

#[test]
fn route_composition_uses_plain_calls_and_bare_component_refs() {
    let out = compile(
        r#"
func Home() {
    return ( <h1>"Home"</h1> )
}

func App() {
    return (
        <Router>
            <Routes fallback={Home}>
                <Route path={StaticSegment('')} view={Home}/>
            </Routes>
        </Router>
    )
}
"#,
    );
    // `view={Home}` and `fallback={Home}` are bare component references (no
    // `.get()`/`.clone()`/call parens) — Home isn't a signal or a prop, so
    // it renders as a plain identifier, exactly what `ChooseView`/`FnOnce()
    // -> Fallback` expect.
    assert!(out.contains("fallback=Home"));
    assert!(out.contains("view=Home"));
    // StaticSegment('') is an ordinary function call, rendered the same way
    // any other `name(args)` call would be.
    assert!(out.contains(r#"path=StaticSegment("")"#));
}

#[test]
fn comparison_operators_in_condition() {
    let out = compile(
        r#"
func App() {
    <{age}> >> 20

    if><{age}> >= 18
        craft<'adult'>
    orif><{age}> < 13
        craft<'child'>
    else>
        craft<'teen'>

    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("if age.get() >= 18f64 {"));
    assert!(out.contains("} else if age.get() < 13f64 {"));
}

#[test]
fn all_comparison_operators_lower_correctly() {
    for (kitty_op, rust_op) in [
        ("<", "<"),
        ("<=", "<="),
        (">=", ">="),
        ("!=", "!="),
    ] {
        let out = compile(&format!(
            r#"
func App() {{
    <{{age}}> >> 20
    if><{{age}}> {kitty_op} 18
        craft<'yes'>
    return ( <div></div> )
}}
"#
        ));
        assert!(
            out.contains(&format!("if age.get() {rust_op} 18f64 {{")),
            "operator {kitty_op} did not lower to {rust_op}: {out}"
        );
    }
}

#[test]
fn comparison_works_in_purr_functions() {
    let out = compile(
        r#"
purr isAdult(<<Num>> age) <<Flag>> {
    return (age >= 18)
}
"#,
    );
    assert!(out.contains("pub fn isAdult(age: f64) -> bool {"));
    assert!(out.contains("age >= 18f64"));
    assert!(!out.contains("(age >= 18f64)")); // no redundant top-level parens
}

#[test]
fn craft_with_bare_gt_comparison_requires_parens() {
    // A bare `>` at the top level of `craft<...>` is ambiguous with
    // craft's own closing `>` — parens disambiguate.
    let out = compile(
        r#"
func App() {
    <{age}> >> 20
    craft<(age > 18)>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("leptos::logging::log!(\"{}\", (age.get() > 18f64));"));
}

#[test]
fn craft_without_comparison_is_unaffected_by_gt_parsing() {
    // Regression check: adding `>` as a general comparison operator must
    // not break plain `craft<expr>` calls that don't use one at all.
    let out = compile(
        r#"
func App() {
    craft<'just a string'>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"leptos::logging::log!("just a string");"#));
}
