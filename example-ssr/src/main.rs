//! The SSR server binary (feature `ssr`) — pairs with `src/lib.rs`'s
//! `hydrate` entry point (feature `hydrate`), built separately by
//! `cargo-leptos` as the wasm32 client bundle. See `docs/SSR.md` for how
//! the two fit together.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_meta::MetaTags;
    use example_ssr::App;

    /// The HTML shell every route is rendered into: `<HydrationScripts>`
    /// (from `leptos`'s own prelude, not `leptos_meta`) is what injects the
    /// `<script type="module">` tag that loads the client wasm bundle and
    /// calls `hydrate()` on it -- omit this and the page renders correctly
    /// server-side but never becomes interactive, since nothing ever loads
    /// the client bundle.
    fn shell(options: LeptosOptions) -> impl IntoView {
        view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8"/>
                    <meta name="viewport" content="width=device-width, initial-scale=1"/>
                    <HydrationScripts options/>
                    <MetaTags/>
                    <title>"Kittine + SSR example"</title>
                </head>
                <body>
                    <App/>
                </body>
            </html>
        }
    }

    let conf = leptos::config::get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        // Serves the wasm/JS bundle and any other static assets from
        // `site-root`, falling back to `shell` (a real 404 page) for
        // anything else unmatched -- without this, the hydration
        // bundle's own `/pkg/*.js`/`*.wasm` requests 404 and the page
        // never hydrates.
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{addr}");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
fn main() {}
