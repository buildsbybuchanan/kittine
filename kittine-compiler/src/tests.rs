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
    assert!(out.contains(r#"let (username, set_username) = signal("Admin".to_string());"#));

    let out2 = compile(
        r#"
func App() {
    <{username}> >> "Admin"
    return ( <div></div> )
}
"#,
    );
    assert!(out2.contains(r#"let (username, set_username) = signal("Admin".to_string());"#));
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
    assert!(out.contains("let (scores, set_scores) = signal(vec![1f64, 2f64, 3f64]);"));
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
    assert!(out.contains(r#"let (label, set_label) = signal("hi".to_string());"#));
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
fn string_literal_argument_to_same_file_word_param_is_owned() {
    // A same-file `purr`'s signature is known at codegen time, so a bare
    // string literal passed where the parameter is `<<Word>>` gets
    // `.to_string()` — it would otherwise render as `&str`, which doesn't
    // type-check against a `Word` parameter's `String`.
    let out = compile(
        r#"
purr shout(<<Word>> word) <<Word>> {
    return (word)
}

func App() {
    craft<shout('hello')>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"shout("hello".to_string())"#));
}

#[test]
fn string_literal_argument_to_same_file_num_param_is_unaffected() {
    // Regression check: the `Word`-specific coercion must not affect a
    // string literal passed to a non-`Word` parameter position.
    let out = compile(
        r#"
purr describe(<<Num>> n, <<Word>> label) <<Word>> {
    return (label)
}

func App() {
    craft<describe(1, 'first')>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"describe(1f64, "first".to_string())"#));
}

#[test]
fn string_literal_argument_to_unknown_function_is_unaffected() {
    // A call to a function whose signature isn't known at codegen time
    // (e.g. one brought in via `import`, or a typo) renders the argument
    // bare, same as before this fix — no known signature means no basis
    // for the coercion.
    let out = compile(
        r#"
func App() {
    craft<someImportedFunc('hello')>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"someImportedFunc("hello")"#));
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
    let (out_path, _) = crate::build(&home_kitty, None).expect("recursive build should succeed");

    let home_rs = std::fs::read_to_string(&out_path).expect("read generated Home.rs");
    assert!(home_rs.contains("mod __kittine_mod_nav;"));
    assert!(home_rs.contains(r#"<Nav active="home".to_string()/>"#));

    let nav_rs = std::fs::read_to_string(dir.join("Nav.rs")).expect("Nav.rs should be generated");
    assert!(nav_rs.contains("pub fn Nav(active: String) -> impl IntoView"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unchanged_output_is_not_rewritten() {
    // Rebuilding a dependency whose source didn't change must not touch its
    // already-up-to-date .rs file's mtime — downstream tools (cargo,
    // wasm-bindgen, Vite's own file watcher) decide whether to redo work by
    // looking at file mtimes, and rewriting byte-identical content still
    // bumps them, making every reachable file look freshly modified on
    // every single build.
    let dir = std::env::temp_dir().join(format!(
        "kittine-incremental-test-{}-{}",
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
    let (_, was_written) = crate::build(&home_kitty, None).expect("first build should succeed");
    assert!(was_written, "a fresh build should report the file as written");

    let nav_rs_path = dir.join("Nav.rs");
    let first_mtime = std::fs::metadata(&nav_rs_path)
        .expect("Nav.rs should exist")
        .modified()
        .expect("mtime");

    // Sleep past typical filesystem mtime resolution, then rebuild with
    // Nav.kitty completely unchanged — only Home.kitty's own generated
    // output (a different prop value passed to Nav) should change.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(
        dir.join("Home.kitty"),
        r#"
import { Nav } from './Nav.kitty'

func Home() {
    return ( <div><Nav active='away'/></div> )
}
"#,
    )
    .expect("rewrite Home.kitty");
    let (_, was_written) = crate::build(&home_kitty, None).expect("second build should succeed");
    assert!(
        was_written,
        "Home.kitty's own content changed, so Home.rs should be reported as written"
    );

    let second_mtime = std::fs::metadata(&nav_rs_path)
        .expect("Nav.rs should still exist")
        .modified()
        .expect("mtime");
    assert_eq!(
        first_mtime, second_mtime,
        "Nav.rs was rewritten even though Nav.kitty's content didn't change"
    );

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
    assert!(out.contains("{move || n.clone()}"));
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
    assert!(out.contains("{move || n.clone()}"));
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
fn dynamic_route_segment_uses_a_tuple_path() {
    let out = compile(
        r#"
func User() {
    return ( <h1>"User"</h1> )
}

func App() {
    return (
        <Router>
            <Routes fallback={User}>
                <Route path={(StaticSegment('user'), ParamSegment('id'))} view={User}/>
            </Routes>
        </Router>
    )
}
"#,
    );
    assert!(out.contains(r#"path=(StaticSegment("user"), ParamSegment("id"))"#));
}

#[test]
fn method_call_chain_renders_verbatim() {
    // Kittine has no receiver-type information, so a method-call chain on
    // any expression renders as-is and lets Rust's own type checker
    // validate it -- this is how a dynamic route segment's value is read
    // (`use_params_map().get().get("id").unwrap_or_default()`), with no
    // dedicated Kittine syntax needed.
    let out = compile(
        r#"
func User() {
    return ( <p>{ use_params_map().get().get('id').unwrap_or_default() }</p> )
}
"#,
    );
    assert!(out.contains(r#"use_params_map().get().get("id").unwrap_or_default()"#));
}

#[test]
fn method_call_numeric_argument_is_not_forced_to_f64() {
    // Unlike a same-file `purr` call (where a `Num` parameter is known to
    // be `f64`), Kittine has no idea what type an arbitrary method's
    // parameter is -- a real Rust method just as often expects `usize`
    // (`Vec::get(0)`) as `f64`, so a numeric literal argument stays plain
    // rather than getting an `f64` suffix forced onto it.
    let out = compile(
        r#"
func App() {
    craft<items.get(0)>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("items.get(0)"));
    assert!(!out.contains("items.get(0f64)"));
}

#[test]
fn calling_the_result_of_an_expression_renders_verbatim() {
    // `use_navigate()('/', ..)` -- calling the closure `use_navigate()`
    // returns immediately, rather than a bare named function -- is a
    // distinct shape (`Expr::CallResult`) from an ordinary `Expr::Call`,
    // since the callee is itself an arbitrary expression.
    let out = compile(
        r#"
func App() {
    craft<use_navigate()('/home')>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"use_navigate()("/home")"#));
}

#[test]
fn generated_file_brings_leptos_router_hooks_into_scope() {
    // `leptos_router::hooks` (use_params_map, use_navigate, ..) isn't
    // re-exported at the crate root the way `components`/`matching` are,
    // so it needs its own explicit `use`.
    let out = compile(
        r#"
func App() {
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("use leptos_router::hooks::*;"));
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
fn logical_and_combines_two_conditions() {
    let out = compile(
        r#"
func App() {
    <{age}> >> 20
    <{status}> >> 'active'

    if><{age}> >= 18 && <{status}> >> 'active'
        craft<'eligible'>

    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"if (age.get() >= 18f64) && (status.get() == "active") {"#));
}

#[test]
fn logical_or_combines_two_conditions() {
    let out = compile(
        r#"
func App() {
    <{age}> >> 20

    if><{age}> < 13 || <{age}> >= 65
        craft<'discount'>

    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"if (age.get() < 13f64) || (age.get() >= 65f64) {"#));
}

#[test]
fn logical_and_binds_tighter_than_or() {
    // `a || b && c` should read as `a || (b && c)`.
    let out = compile(
        r#"
func App() {
    <{a}> >> 1
    <{b}> >> 2
    <{c}> >> 3

    if><{a}> >> 1 || <{b}> >> 2 && <{c}> >> 3
        craft<'x'>

    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(
        r#"if (a.get() == 1f64) || ((b.get() == 2f64) && (c.get() == 3f64)) {"#
    ));
}

#[test]
fn logical_operators_work_in_purr_functions() {
    let out = compile(
        r#"
purr inRange(<<Num>> n) <<Flag>> {
    return (n >= 0 && n <= 100)
}
"#,
    );
    assert!(out.contains("(n >= 0f64) && (n <= 100f64)"));
}

#[test]
fn craft_supports_logical_and_with_parenthesized_comparisons() {
    let out = compile(
        r#"
func App() {
    <{age}> >> 20
    craft<(age > 18) && (age < 65)>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(
        r#"leptos::logging::log!("{}", ((age.get() > 18f64) && (age.get() < 65f64)));"#
    ));
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

#[test]
fn array_typed_prop_and_return() {
    let out = compile(
        r#"
func NavList(<<Word[]>> items) {
    return ( <ul><li>{ items }</li></ul> )
}

purr passthrough(<<Num[]>> scores) <<Num[]>> {
    return (scores)
}
"#,
    );
    assert!(out.contains("pub fn NavList(items: Vec<String>) -> impl IntoView"));
    assert!(out.contains("pub fn passthrough(scores: Vec<f64>) -> Vec<f64>"));
    // `Vec` isn't `Copy`, so reads of an array-typed prop clone it, same
    // as a `Word` prop.
    assert!(out.contains("{move || items.clone()}"));
}

#[test]
fn string_signal_init_is_owned_string_not_borrowed_str() {
    // A bare string literal signal initializer must produce an owned
    // `String`, not `&'static str` — otherwise it silently fails to
    // compile the moment that signal's value is passed somewhere an
    // owned `String` is required (e.g. as a `Word`-typed prop to another
    // component).
    let out = compile(
        r#"
func App() {
    <{username}> >> 'Admin'
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"signal("Admin".to_string());"#));
}

#[test]
fn spin_in_view_clones_item_even_for_copy_types() {
    // Regression test: a view-position `spin`'s `{move || item}` closure
    // must be callable more than once (Fn, not FnOnce) for Leptos's
    // reactivity to work. Moving a non-`Copy` `item` (e.g. a `Word`) out
    // of the closure would only satisfy `FnOnce`, so `item` is always
    // `.clone()`d regardless of its element type — cheap/free for `Copy`
    // types like `Num`, required for everything else.
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
    assert!(out.contains("{move || n.clone()}"));
}

#[test]
fn array_type_tag_checks_element_literals() {
    let message = compile_err(
        r#"
func App() {
    <{scores}> >> <<Num[]>> ['a', 'b']
    return ( <div></div> )
}
"#,
    );
    assert!(message.contains("does not match"));
}

#[test]
fn private_purr_is_not_pub() {
    let out = compile(
        r#"
private purr helper(<<Num>> n) <<Num>> {
    return (n * 2)
}
"#,
    );
    assert!(out.contains("fn helper(n: f64) -> f64 {"));
    assert!(!out.contains("pub fn helper"));
}

#[test]
fn private_component_is_not_pub() {
    let out = compile(
        r#"
private func Internal() {
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("fn Internal() -> impl IntoView {"));
    assert!(!out.contains("pub fn Internal"));
}

#[test]
fn non_private_items_stay_pub_by_default() {
    let out = compile(
        r#"
func Home() {
    return ( <div></div> )
}

purr doubled(<<Num>> n) <<Num>> {
    return (n * 2)
}
"#,
    );
    assert!(out.contains("pub fn Home()"));
    assert!(out.contains("pub fn doubled(n: f64) -> f64"));
}
