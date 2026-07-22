//! Pantalla de inicio de sesión. Formulario controlado (signals, no
//! refs al DOM) que dispara `auth::iniciar_sesion` como una `Action` y
//! navega a "/" cuando responde con éxito.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::auth::{iniciar_sesion, use_auth};

/// Ícono de sobre (correo), para el prefijo del campo de email.
fn icono_correo() -> impl IntoView {
    view! {
        <svg class="field-icon" viewBox="0 0 24 24">
            <rect x="3.5" y="5.5" width="17" height="13" rx="2"></rect>
            <path d="M4 7l8 6 8-6"></path>
        </svg>
    }
}

/// Ícono de candado, para el prefijo del campo de contraseña.
fn icono_candado() -> impl IntoView {
    view! {
        <svg class="field-icon" viewBox="0 0 24 24">
            <rect x="5.5" y="10.5" width="13" height="9" rx="2"></rect>
            <path d="M8.5 10.5v-3a3.5 3.5 0 0 1 7 0v3"></path>
        </svg>
    }
}

/// Ícono de ojo (mostrar/ocultar contraseña); `abierto` decide si se
/// dibuja el ojo normal o el ojo tachado.
fn icono_ojo(abierto: bool) -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24">
            <path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"></path>
            <circle cx="12" cy="12" r="2.6"></circle>
            <Show when=move || !abierto>
                <path d="M4 4l16 16"></path>
            </Show>
        </svg>
    }
}

#[component]
pub fn Login() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let mostrar_clave = RwSignal::new(false);
    // Solo visual por ahora: el diseño de Geck trae este checkbox, pero
    // no hay "recordar sesión" persistente implementado en el backend.
    let recordar = RwSignal::new(true);

    // `?invitado=1` lo agrega el redirect de AceptarInvitacion al
    // canjear una invitación con éxito, y `?reset=1` el de
    // RecuperarPassword al fijar una contraseña nueva — ambos solo para
    // mostrar el aviso de abajo.
    let query = use_query_map();
    let viene_de_invitacion = move || query.with(|q| q.get("invitado").is_some());
    let viene_de_reset = move || query.with(|q| q.get("reset").is_some());

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
        <div class="auth-screen login-split">
            // Panel de formulario (mitad izquierda en escritorio).
            <div class="login-form-panel">
                <div class="login-form-panel-inner">
                    <div class="login-brand-row">
                        <span class="brand-mark brand-mark-lg">"g"</span>
                        <span class="brand-word">"geck"</span>
                        <span class="login-brand-tag">"GESTOR DE FINANZAS"</span>
                    </div>

                    <h1>"Bienvenido de vuelta"</h1>
                    <p class="auth-subtitle text-soft">"Ingresa a tu cuenta para seguir controlando tus finanzas."</p>

                    <Show when=viene_de_invitacion>
                        <p class="banner">"Cuenta creada, ya puedes iniciar sesión."</p>
                    </Show>
                    <Show when=viene_de_reset>
                        <p class="banner">"Contraseña actualizada, ya puedes iniciar sesión."</p>
                    </Show>

                    <form on:submit=move |ev| {
                        ev.prevent_default();
                        entrar.dispatch(());
                    }>
                        <div class="field">
                            <label for="email">"Correo electrónico"</label>
                            <div class="field-input-wrap">
                                {icono_correo()}
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
                        <div class="field">
                            <label for="password">"Contraseña"</label>
                            <div class="field-input-wrap has-toggle">
                                {icono_candado()}
                                <input
                                    id="password"
                                    type=move || if mostrar_clave.get() { "text" } else { "password" }
                                    placeholder="••••••••"
                                    autocomplete="current-password"
                                    required
                                    prop:value=move || password.get()
                                    on:input=move |ev| password.set(event_target_value(&ev))
                                />
                                <button
                                    type="button"
                                    class="password-toggle"
                                    aria-label="Mostrar u ocultar contraseña"
                                    on:click=move |_| mostrar_clave.update(|valor| *valor = !*valor)
                                >
                                    {move || icono_ojo(mostrar_clave.get())}
                                </button>
                            </div>
                        </div>

                        <div class="login-remember-row">
                            <button
                                type="button"
                                class="login-remember-btn"
                                on:click=move |_| recordar.update(|valor| *valor = !*valor)
                            >
                                <span class="login-remember-check" data-active=move || recordar.get().to_string()>
                                    <svg viewBox="0 0 24 24">
                                        <path d="M5 12l5 5L20 6"></path>
                                    </svg>
                                </span>
                                "Recordarme"
                            </button>
                            <a href="/recuperar-password" class="login-forgot-link">"¿Olvidaste tu contraseña?"</a>
                        </div>

                        <Show when=move || mensaje_error().is_some()>
                            <p class="banner banner-error">{mensaje_error}</p>
                        </Show>

                        <button type="submit" class="btn btn-primary btn-block" disabled=move || entrar.pending().get()>
                            {move || if entrar.pending().get() { "Entrando..." } else { "Iniciar sesión" }}
                        </button>
                    </form>

                    <p class="auth-footer">"¿No tienes cuenta? " <a href="#">"Crear cuenta"</a></p>
                </div>
            </div>

            // Panel de mascota (mitad derecha en escritorio, arriba en móvil).
            <div class="login-mascot-panel">
                <div class="login-mascot-glow login-mascot-glow-a"></div>
                <div class="login-mascot-glow login-mascot-glow-b"></div>

                <div class="login-mascot-top">
                    <span class="login-mascot-pill">"PWA · MXN"</span>
                    <span class="login-mascot-dot"></span>
                </div>

                <div class="login-mascot-body">
                    // La mascota vive en frontend/assets/gecko.svg (Trunk la copia
                    // a /assets/ tal cual). Si el archivo no está, el navegador
                    // muestra el ícono roto de imagen — no falla la compilación.
                    <img src="/assets/gecko.svg" alt="Geck" class="gecko-mascot" />
                    <p class="login-mascot-title">"Controla tus finanzas y controlarás tu vida"</p>
                    <p class="login-mascot-subtitle">"Gecko te acompaña con tus movimientos, metas y ahorros en un solo lugar."</p>
                </div>

                <div class="login-mascot-chips">
                    <span>"Movimientos"</span>
                    <span>"Metas"</span>
                    <span>"Inversiones"</span>
                    <span>"Agenda"</span>
                </div>
            </div>
        </div>
    }
}
