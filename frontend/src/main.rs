mod api;
mod app;
mod auth;
mod components;
mod pages;
mod workspace;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    // Si el logger de consola no arranca, la app sigue funcionando
    // igual — solo se pierden los logs de depuración, nada crítico.
    let _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount::mount_to_body(App);

    quitar_loader_inicial();
}

/// Quita del DOM el loader estático de `index.html` (mascota + "Cargando…")
/// una vez que Leptos ya montó la app real. Vive fuera del árbol de
/// componentes porque debe verse mientras el WASM aún se descarga/compila.
fn quitar_loader_inicial() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    if let Some(loader) = document.get_element_by_id("wasm-loader") {
        loader.remove();
    }
}
