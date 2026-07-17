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
    assert!(out.contains("let (count, set_count) = signal(0);"));
    assert!(out.contains("use leptos::prelude::*;"));
}

#[test]
fn string_literal_uses_diaeresis_quotes() {
    let out = compile(
        r#"
func App() {
    <{username}> >> ¨Admin¨
    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"let (username, set_username) = signal("Admin");"#));
}

#[test]
fn craft_lowers_to_leptos_log() {
    let out = compile(
        r#"
func App() {
    craft<¨hello world¨>
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
    assert!(out.contains("let (count, set_count) = signal(0);"));
    assert!(out.contains("set_count.update(|n| *n += 1);"));
}

#[test]
fn if_orif_else_chain() {
    let out = compile(
        r#"
func App() {
    <{username}> >> ¨Admin¨

    if><{username}> >> ¨Admin¨
        craft<¨Welcome Admin¨>
    orif><{username}> >> ¨User¨
        craft<¨Welcome User¨>
    else>
        craft<¨no output¨>

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
    assert!(out.contains("on:click=move |_| set_count.update(|n| *n += 1)"));
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
    craft<¨Taps: ¨ + <{count}> >
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
    assert!(out.contains("set_count.update(|n| *n += 1);"));
}

#[test]
fn string_concat_in_jsx_expr_interpolation() {
    let out = compile(
        r#"
func App() {
    <{mood}> >> ¨Curious¨
    return (
        <div>
            <p>{ ¨Mood: ¨ + <{mood}> }</p>
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
    <{label}> >> ¨Taps: 0¨
    <{label}> >> ¨Taps: ¨ + <{label}>
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
            craft<¨both zero¨>
        else>
            craft<¨only a¨>
    else>
        craft<¨not a¨>

    return ( <div></div> )
}
"#,
    );
    assert!(out.contains(r#"leptos::logging::log!("both zero");"#));
    assert!(out.contains(r#"leptos::logging::log!("only a");"#));
    assert!(out.contains(r#"leptos::logging::log!("not a");"#));
}
