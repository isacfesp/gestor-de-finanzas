//! Pantalla de inicio de sesión. Formulario controlado (signals, no
//! refs al DOM) que dispara `auth::iniciar_sesion` como una `Action` y
//! navega a "/" cuando responde con éxito.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::auth::{iniciar_sesion, use_auth};

#[component]
pub fn Login() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());

    // new_unsync: los futures que llaman a `fetch` desde el navegador no
    // son Send (WASM es de un solo hilo), así que se usa la variante de
    // Action que no lo exige en vez de Action::new.
    let entrar = Action::new_unsync(move |_: &()| {
        let correo = email.get_untracked();
        let clave = password.get_untracked();
        async move { iniciar_sesion(auth, &correo, &clave).await }
    });

    // En cuanto la acción resuelve con éxito, salir de /login.
    Effect::new(move |_| {
        if let Some(Ok(())) = entrar.value().get() {
            navigate("/", NavigateOptions::default());
        }
    });

    let mensaje_error = move || {
        entrar
            .value()
            .get()
            .and_then(|resultado| resultado.err())
            .map(|error| error.to_string())
    };

    view! {
        <div class="auth-screen">
            <section class="auth-card">
                <h1>"Iniciar sesión"</h1>

                <form on:submit=move |ev| {
                    ev.prevent_default();
                    entrar.dispatch(());
                }>
                    <div class="field">
                        <label for="email">"Correo"</label>
                        <input
                            id="email"
                            type="email"
                            autocomplete="username"
                            required
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="field">
                        <label for="password">"Contraseña"</label>
                        <input
                            id="password"
                            type="password"
                            autocomplete="current-password"
                            required
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                        />
                    </div>

                    <Show when=move || mensaje_error().is_some()>
                        <p class="banner banner-error">{mensaje_error}</p>
                    </Show>

                    <button type="submit" class="btn btn-primary btn-block" disabled=move || entrar.pending().get()>
                        {move || if entrar.pending().get() { "Entrando..." } else { "Entrar" }}
                    </button>
                </form>
            </section>
        </div>
    }
}
