mod api;
mod app;
mod auth;
mod components;
mod pages;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    // Si el logger de consola no arranca, la app sigue funcionando
    // igual — solo se pierden los logs de depuración, nada crítico.
    let _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount::mount_to_body(App);
}
