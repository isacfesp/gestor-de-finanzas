//! Pantalla pública donde un invitado canjea el token que le compartió
//! el dev (link `/invitaciones/aceptar?token=...` armado en la pestaña
//! Invitaciones del panel admin) para crear su cuenta.

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::api;

#[component]
pub fn AceptarInvitacion() -> impl IntoView {
    let navigate = use_navigate();
    let query = use_query_map();
    // Se lee una sola vez: esta página no cambia de token sin recargar.
    let token = query.with_untracked(|q| q.get("token").unwrap_or_default());

    let nombre = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let confirmar = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);

    let claves_no_coinciden =
        move || !confirmar.get().is_empty() && password.get() != confirmar.get();

    let token_para_accion = token.clone();
    let aceptar = Action::new_unsync(move |_: &()| {
        let token = token_para_accion.clone();
        let nombre_actual = nombre.get_untracked();
        let password_actual = password.get_untracked();
        async move { api::auth::aceptar_invitacion(&token, &nombre_actual, &password_actual).await }
    });

    Effect::new(move |_| {
        if let Some(resultado) = aceptar.value().get() {
            match resultado {
                Ok(()) => navigate("/login?invitado=1", NavigateOptions::default()),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        }
    });

    view! {
        <div class="auth-screen auth-screen-center">
            <section class="auth-card glass">
                <h1>"Crea tu cuenta"</h1>

                <Show
                    when=move || !token.is_empty()
                    fallback=|| {
                        view! {
                            <p class="banner banner-error">
                                "Este link de invitación no es válido. Pide uno nuevo a quien te invitó."
                            </p>
                        }
                    }
                >
                    <form on:submit=move |ev: SubmitEvent| {
                        ev.prevent_default();
                        error.set(None);
                        if claves_no_coinciden() {
                            error.set(Some("Las contraseñas no coinciden".to_string()));
                            return;
                        }
                        aceptar.dispatch(());
                    }>
                        <div class="field">
                            <label for="nombre">"Nombre"</label>
                            <div class="field-input-wrap">
                                <input
                                    id="nombre"
                                    type="text"
                                    placeholder="Tu nombre"
                                    autocomplete="name"
                                    required
                                    prop:value=move || nombre.get()
                                    on:input=move |ev| nombre.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <div class="field">
                            <label for="password">"Contraseña"</label>
                            <div class="field-input-wrap">
                                <input
                                    id="password"
                                    type="password"
                                    placeholder="Mínimo 12 caracteres"
                                    autocomplete="new-password"
                                    minlength="12"
                                    required
                                    prop:value=move || password.get()
                                    on:input=move |ev| password.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <div class="field">
                            <label for="confirmar">"Confirmar contraseña"</label>
                            <div class="field-input-wrap">
                                <input
                                    id="confirmar"
                                    type="password"
                                    placeholder="Repite la contraseña"
                                    autocomplete="new-password"
                                    required
                                    prop:value=move || confirmar.get()
                                    on:input=move |ev| confirmar.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <Show when=claves_no_coinciden>
                            <p class="banner banner-error">"Las contraseñas no coinciden"</p>
                        </Show>

                        <Show when=move || error.get().is_some()>
                            <p class="banner banner-error">{move || error.get().unwrap_or_default()}</p>
                        </Show>

                        <button
                            type="submit"
                            class="btn btn-primary btn-block"
                            disabled=move || aceptar.pending().get()
                        >
                            {move || if aceptar.pending().get() { "Creando cuenta..." } else { "Crear cuenta" }}
                        </button>
                    </form>
                </Show>
            </section>
        </div>
    }
}
