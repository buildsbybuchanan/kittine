use wasm_bindgen::JsCast;

#[path = "App.rs"]
mod app;
use app::App;

fn main() {
    console_error_panic_hook::set_once();

    let root = web_sys::window()
        .expect("no global `window`")
        .document()
        .expect("no `document` on `window`")
        .get_element_by_id("root")
        .expect("no element with id 'root' in index.html")
        .unchecked_into::<web_sys::HtmlElement>();

    leptos::mount::mount_to(root, App).forget();
}
