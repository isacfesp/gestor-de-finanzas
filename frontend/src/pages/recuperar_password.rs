//! Pantalla pública de recuperación de contraseña: sin `?token=` en la
//! URL pide el correo para enviar el link; con token, deja fijar la
//! contraseña nueva (link armado por el backend como
//! `/recuperar-password?token=...`, ver `correo::plantilla_recuperacion`).

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::api;

#[component]
pub fn RecuperarPassword() -> impl IntoView {
    let query = use_query_map();
    // Se lee una sola vez: esta página no cambia de token sin recargar.
    let token = query.with_untracked(|q| q.get("token").unwrap_or_default());
    let token_para_show = token.clone();
    let token_para_fallback = token.clone();

    view! {
        <div class="auth-screen auth-screen-center">
            <section class="auth-card glass">
                <Show
                    when=move || token_para_show.is_empty()
                    fallback={
                        let token = token_para_fallback.clone();
                        move || view! { <FormularioNuevaPassword token=token.clone()/> }
                    }
                >
                    <FormularioSolicitud/>
                </Show>
            </section>
        </div>
    }
}

/// Sin token: pide el correo y dispara `/auth/solicitar-recuperacion`.
#[component]
fn FormularioSolicitud() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);

    let solicitar = Action::new_unsync(move |_: &()| {
        let correo_actual = email.get_untracked();
        async move { api::auth::solicitar_recuperacion(&correo_actual).await }
    });

    Effect::new(move |_| {
        if let Some(resultado) = solicitar.value().get()
            && let Err(error_api) = resultado
        {
            error.set(Some(error_api.to_string()));
        }
    });

    let enviado = move || matches!(solicitar.value().get(), Some(Ok(())));

    view! {
        <h1>"Recupera tu contraseña"</h1>

        <Show
            when=enviado
            fallback=move || {
                view! {
                    <p class="auth-subtitle text-soft">
                        "Escribe tu correo y te mandamos un link para elegir una contraseña nueva."
                    </p>
                    <form on:submit=move |ev: SubmitEvent| {
                        ev.prevent_default();
                        error.set(None);
                        solicitar.dispatch(());
                    }>
                        <div class="field">
                            <label for="email">"Correo electrónico"</label>
                            <div class="field-input-wrap">
                                <input
                                    id="email"
                                    type="email"
                                    placeholder="tu@correo.com"
                                    autocomplete="username"
                                    required
                                    prop:value=move || email.get()
                                    on:input=move |ev| email.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <Show when=move || error.get().is_some()>
                            <p class="banner banner-error">{move || error.get().unwrap_or_default()}</p>
                        </Show>

                        <button
                            type="submit"
                            class="btn btn-primary btn-block"
                            disabled=move || solicitar.pending().get()
                        >
                            {move || if solicitar.pending().get() { "Enviando..." } else { "Enviar link" }}
                        </button>
                    </form>
                }
            }
        >
            <p class="banner">
                "Si el correo está registrado, te llegará un link para recuperar tu contraseña."
            </p>
        </Show>
    }
}

/// Con token: pide la contraseña nueva y la confirma, y dispara
/// `/auth/recuperar-password`.
#[component]
fn FormularioNuevaPassword(token: String) -> impl IntoView {
    let navigate = use_navigate();

    let password = RwSignal::new(String::new());
    let confirmar = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);

    let claves_no_coinciden =
        move || !confirmar.get().is_empty() && password.get() != confirmar.get();

    let token_para_accion = token.clone();
    let confirmar_reset = Action::new_unsync(move |_: &()| {
        let token = token_para_accion.clone();
        let password_actual = password.get_untracked();
        async move { api::auth::recuperar_password(&token, &password_actual).await }
    });

    Effect::new(move |_| {
        if let Some(resultado) = confirmar_reset.value().get() {
            match resultado {
                Ok(()) => navigate("/login?reset=1", NavigateOptions::default()),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        }
    });

    view! {
        <h1>"Elige tu contraseña nueva"</h1>

        <form on:submit=move |ev: SubmitEvent| {
            ev.prevent_default();
            error.set(None);
            if claves_no_coinciden() {
                error.set(Some("Las contraseñas no coinciden".to_string()));
                return;
            }
            confirmar_reset.dispatch(());
        }>
            <div class="field">
                <label for="password">"Contraseña nueva"</label>
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
                disabled=move || confirmar_reset.pending().get()
            >
                {move || if confirmar_reset.pending().get() { "Guardando..." } else { "Guardar contraseña" }}
            </button>
        </form>
    }
}
