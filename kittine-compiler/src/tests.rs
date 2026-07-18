//! End-to-end tests exercising the full lex -> parse -> codegen pipeline
//! against the exact syntax shown in the Kittine language spec.

use crate::codegen;
use crate::lexer;
use crate::parser;

fn compile(src: &str) -> String {
    let tokens = lexer::tokenize(src).expect("lex should succeed");
    let program = parser::parse(tokens).expect("parse should succeed");
    let known_functions = codegen::collect_function_signatures(&program.items);
    codegen::generate(&program, &known_functions)
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
fn hold_lowers_to_a_plain_let_not_a_signal() {
    let out = compile(
        r#"
func App() {
    hold navigate >> use_navigate()

    return (
        <button onClick={navigate('/', NavigateOptions::default())}>
            "Go"
        </button>
    )
}
"#,
    );
    assert!(out.contains("let navigate = use_navigate();"));
    assert!(!out.contains("signal(use_navigate())"));
    // A bare call to a hold-bound name pre-clones it before the closure
    // (so a second, sibling closure calling `navigate` elsewhere doesn't
    // fight over moving the same original) and `.clone()`s it again to
    // actually call it, same as any other read.
    assert!(out.contains(
        r#"on:click={ let navigate = navigate.clone(); move |_| navigate.clone()("/", NavigateOptions::default()) }"#
    ));
}

#[test]
fn hold_bound_value_is_cloned_at_every_read() {
    // Same reasoning as a view-position spin's loop variable: a held
    // value may need to be captured by more than one reactive closure, so
    // reads are always `.clone()`d regardless of the held type.
    let out = compile(
        r#"
func App() {
    hold label >> 'fixed'
    return ( <div>{ label }<p>{ label }</p></div> )
}
"#,
    );
    assert!(out.contains("let label = \"fixed\""));
    // Each of the two sibling interpolations pre-clones `label` into its
    // own local before its closure (so neither fights the other over
    // moving the original), then `.clone()`s again inside.
    assert_eq!(
        out.matches("{ let label = label.clone(); move || label.clone() }")
            .count(),
        2
    );
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
fn word_signal_mutation_to_a_brand_new_literal_is_owned() {
    // `<{label}> >> 'reset'` as a *mutation* (not the signal's
    // first/declaring occurrence) used to render `*n = "reset"` -- a bare
    // `&'static str` assigned into `*n: &mut String`, which doesn't
    // type-check. Same class of fix as `render_signal_init`'s treatment
    // of a signal's first occurrence, just for a later mutation.
    let out = compile(
        r#"
func App() {
    <{label}> >> 'Taps: 0'
    <{label}> >> 'reset'
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"set_label.update(|n| *n = "reset".to_string());"#));
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
    // A Word-typed prop is pre-cloned into its own local before the
    // closure (so a second, sibling closure elsewhere doesn't fight over
    // moving the same original -- see `wrap_reactive_closure`), and read
    // via `.clone()` again inside (so the closure itself can be called
    // more than once without moving its own copy out).
    assert!(out.contains("{{ let active = active.clone(); move || active.clone() }}"));
}

#[test]
fn word_prop_used_in_two_places_does_not_conflict_over_moving() {
    // Regression test for a real bug found by actually compiling this
    // exact pattern against Leptos: a `move` closure captures every
    // variable it uses *by value*, including ones only ever read via
    // `.clone()` inside. Without pre-cloning each usage site into its own
    // local first, two sibling closures both reading the same original
    // `active` fail with E0382 ("use of moved value") -- confirmed with a
    // real `cargo check`, not assumed. Each of the two interpolations
    // below must pre-clone `active` independently.
    let out = compile(
        r#"
func Nav(<<Word>> active) {
    return ( <div><h1>{ active }</h1><p>{ active }</p></div> )
}
"#,
    );
    assert_eq!(
        out.matches("{ let active = active.clone(); move || active.clone() }")
            .count(),
        2
    );
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
    // A call to a function whose signature genuinely isn't known (a typo,
    // or a real Rust/Leptos function Kittine has no `purr` definition for
    // at all) renders the argument bare, same as before this fix — no
    // known signature means no basis for the coercion. (A call to a
    // function brought in via `import` *is* covered now — see
    // `cross_file_string_literal_argument_to_word_param_is_owned`, which
    // exercises the real `crate::build` CLI path since this file-level
    // `compile()` helper never sees another file's signatures.)
    let out = compile(
        r#"
func App() {
    craft<someUnknownFunc('hello')>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"someUnknownFunc("hello")"#));
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
fn cross_file_string_literal_argument_to_word_param_is_owned() {
    // A string literal passed to a `purr` brought in via `import` now
    // gets the same `Word`-parameter `.to_string()` treatment a same-file
    // call already had — `crate::build` collects every reachable file's
    // `purr` signatures (see `main.rs`'s `collect_all_signatures`) before
    // generating any single file's code, so this needs the real CLI build
    // path, not the single-file `compile()` helper used elsewhere in this
    // test file.
    let dir = std::env::temp_dir().join(format!(
        "kittine-cross-file-word-arg-test-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    std::fs::write(
        dir.join("Greeter.kitty"),
        r#"
purr greet(<<Word>> name) <<Word>> {
    return ('Hello, ' + name)
}
"#,
    )
    .expect("write Greeter.kitty");

    std::fs::write(
        dir.join("Home.kitty"),
        r#"
import { greet } from './Greeter.kitty'

func Home() {
    craft<greet('World')>
    return ( <div></div> )
}
"#,
    )
    .expect("write Home.kitty");

    let home_kitty = dir.join("Home.kitty");
    let (out_path, _) = crate::build(&home_kitty, None).expect("recursive build should succeed");

    let home_rs = std::fs::read_to_string(&out_path).expect("read generated Home.rs");
    assert!(home_rs.contains(r#"greet("World".to_string())"#));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn re_export_lets_a_third_file_import_through_an_intermediate() {
    // A.kitty defines Nav; B.kitty re-exports it (`export import`); C.kitty
    // imports Nav from B, never reaching back to A directly. Needs the
    // real CLI build path -- three separately compiled files, resolved and
    // linked together by kittine-compiler and then rustc.
    let dir = std::env::temp_dir().join(format!(
        "kittine-reexport-test-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    std::fs::write(
        dir.join("A.kitty"),
        r#"
func Nav(<<Word>> active) {
    return ( <span>{ active }</span> )
}
"#,
    )
    .expect("write A.kitty");

    std::fs::write(
        dir.join("B.kitty"),
        r#"
export import { Nav } from './A.kitty'
"#,
    )
    .expect("write B.kitty");

    std::fs::write(
        dir.join("C.kitty"),
        r#"
import { Nav } from './B.kitty'

func App() {
    return ( <div><Nav active='home'/></div> )
}
"#,
    )
    .expect("write C.kitty");

    let c_kitty = dir.join("C.kitty");
    let (out_path, _) = crate::build(&c_kitty, None).expect("recursive build should succeed");

    let c_rs = std::fs::read_to_string(&out_path).expect("read generated C.rs");
    assert!(c_rs.contains("mod __kittine_mod_b;"));
    assert!(c_rs.contains("use __kittine_mod_b::{Nav};"));
    assert!(c_rs.contains(r#"<Nav active="home".to_string()/>"#));

    let b_rs = std::fs::read_to_string(dir.join("B.rs")).expect("B.rs should be generated");
    assert!(b_rs.contains("mod __kittine_mod_a;"));
    assert!(b_rs.contains("pub use __kittine_mod_a::{Nav};"));

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
fn plain_import_is_not_pub_use() {
    // Regression check: an ordinary import must stay a private `use` --
    // only `export import` should ever emit `pub use`.
    let out = compile(
        r#"
import { Nav } from './Nav.kitty'

func Home() {
    return ( <div><Nav/></div> )
}
"#,
    );
    assert!(!out.contains("pub use __kittine_mod_nav"));
}

#[test]
fn export_import_emits_pub_use() {
    let out = compile(
        r#"
export import { Nav } from './Nav.kitty'

func Home() {
    return ( <div><Nav/></div> )
}
"#,
    );
    assert!(out.contains(r#"#[path = "./Nav.rs"]"#));
    assert!(out.contains("mod __kittine_mod_nav;"));
    assert!(out.contains("pub use __kittine_mod_nav::{Nav};"));
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
    // Pre-cloned into its own local before the closure, then `.clone()`d
    // again inside -- see `wrap_reactive_closure`.
    assert!(out.contains("{{ let n = n.clone(); move || n.clone() }}"));
}

#[test]
fn spin_in_view_supports_custom_key() {
    let out = compile(
        r#"
purr indexOf(<<Num>> n) <<Num>> {
    return (n * 2)
}

func List() {
    <{items}> >> [1, 2, 3]
    return (
        <ul>
            spin<{n}> in <{items}> key(indexOf(n)) }{
                <li>{ n }</li>
            }{
        </ul>
    )
}
"#,
    );
    assert!(out.contains(
        r#"<For each=move || items.get() key=|n| indexOf(n.clone()) let:n>"#
    ));
}

#[test]
fn spin_in_view_key_clause_does_not_shadow_key_identifier() {
    // `key` isn't a reserved word -- it must stay usable as an ordinary
    // identifier everywhere a `spin` view isn't specifically expecting a
    // `key(...)` clause right before the `}{` fence.
    let out = compile(
        r#"
func List() {
    <{key}> >> 5
    return (
        <ul>
            spin<{n}> in [1, 2, 3] }{
                <li>{ n }</li>
            }{
            <p>{ key }</p>
        </ul>
    )
}
"#,
    );
    assert!(out.contains(r#"key=|n| format!("{n}") let:n"#));
    assert!(out.contains("{move || key.get()}"));
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
    assert!(out.contains("{{ let n = n.clone(); move || n.clone() }}"));
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
fn path_qualified_expression_renders_verbatim() {
    let out = compile(
        r#"
func App() {
    craft<Default::default()>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("Default::default()"));
}

#[test]
fn path_qualified_expression_supports_more_than_two_segments() {
    let out = compile(
        r#"
func App() {
    craft<std::cmp::max(1, 2)>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains("std::cmp::max(1, 2)"));
}

#[test]
fn programmatic_navigation_call_shape_renders_correctly() {
    // This exact shape now renders correctly -- the piece that was a
    // documented gap (NavigateOptions needs a path-qualified `::default()`
    // call to construct) is fixed. It's NOT, on its own, how a real
    // onClick handler should call use_navigate() though: calling it lazily
    // inside the handler compiles fine but panics at runtime ("cannot call
    // use_navigate outside a <Router>") -- see
    // `programmatic_navigation_via_eager_signal_call_is_the_real_pattern`
    // for the pattern that's actually correct, discovered by testing this
    // one against a real running dev server, not just checking it compiled.
    let out = compile(
        r#"
func App() {
    craft<use_navigate()('/home', NavigateOptions::default())>
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"use_navigate()("/home", NavigateOptions::default())"#));
}

#[test]
fn programmatic_navigation_via_eager_hold_is_the_real_pattern() {
    // Leptos's context-dependent hooks (use_navigate among them) must run
    // while the component's own reactive owner is active -- true during
    // synchronous component setup, not by the time a later event fires.
    // `hold navigate >> use_navigate()` forces the eager call at the right
    // time; calling `navigate(..)` later (a bare call to a hold-bound
    // name) from *two* separate buttons is what example-app's User.kitty
    // actually uses (a real page needs to navigate to more than one
    // place), verified against a real running dev server with Playwright
    // (a click that changed the URL with no console panic) and against a
    // real `cargo check` (each closure must pre-clone `navigate`
    // independently, or the second one fails to compile -- see
    // `word_prop_used_in_two_places_does_not_conflict_over_moving` for the
    // same class of bug found and fixed elsewhere).
    let out = compile(
        r#"
func App() {
    hold navigate >> use_navigate()
    return (
        <div>
            <button onClick={navigate('/home', NavigateOptions::default())}>
                "Home"
            </button>
            <button onClick={navigate('/about', NavigateOptions::default())}>
                "About"
            </button>
        </div>
    )
}
"#,
    );
    assert!(out.contains("let navigate = use_navigate();"));
    assert_eq!(
        out.matches("{ let navigate = navigate.clone(); move |_| navigate.clone()(")
            .count(),
        2
    );
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
    // as a `Word` prop -- pre-cloned before the closure too, same as any
    // other non-`Copy` tracked name (see `wrap_reactive_closure`).
    assert!(out.contains("{{ let items = items.clone(); move || items.clone() }}"));
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
    assert!(out.contains("{{ let n = n.clone(); move || n.clone() }}"));
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
