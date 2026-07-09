//! Pestaña "Invitaciones": generación de invitaciones a un tenant
//! (backend `admin`, `SoloDev`). Sin historial — el backend no expone
//! `GET /admin/invitaciones`, así que el token generado solo se
//! muestra una vez, en el momento de crearlo.

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use uuid::Uuid;

use crate::api::admin::{self, InvitacionCreada};
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn PestanaInvitaciones() -> impl IntoView {
    let auth = use_auth();
    let invitacion_creada = RwSignal::new(None::<InvitacionCreada>);

    let workspaces = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        admin::listar_workspaces(&token)
            .await
            .map_err(|e| e.to_string())
    });

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Invitaciones"</h2>
            </div>

            <Show when=move || invitacion_creada.get().is_some()>
                <div class="banner" style="margin-bottom:16px;">
                    <p style="margin:0 0 6px;">
                        "Invitación generada — copia el token ahora, no se volverá a mostrar:"
                    </p>
                    <p class="mono" style="margin:0; font-weight:700; word-break:break-all;">
                        {move || invitacion_creada.get().map(|i| i.token).unwrap_or_default()}
                    </p>
                    <p class="text-faint" style="margin:6px 0 0; font-size:12px;">
                        "Expira: "
                        {move || {
                            invitacion_creada
                                .get()
                                .map(|i| i.expira.format("%d/%m/%Y %H:%M").to_string())
                                .unwrap_or_default()
                        }}
                    </p>
                </div>
            </Show>

            {move || match workspaces.get() {
                None => view! { <p class="text-soft">"Cargando tenants..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"Todavía no hay ningún tenant. Crea uno en la pestaña Tenants."</p> }
                        .into_any()
                }
                Some(Ok(lista)) => view! {
                    <FormularioInvitacion
                        workspaces=lista
                        on_creada=move |inv| invitacion_creada.set(Some(inv))
                    />
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn FormularioInvitacion<F>(workspaces: Vec<admin::Workspace>, on_creada: F) -> impl IntoView
where
    F: Fn(InvitacionCreada) + 'static + Copy,
{
    let auth = use_auth();
    let workspace_id = RwSignal::new(workspaces.first().map(|w| w.id).unwrap_or(Uuid::nil()));
    let email = RwSignal::new(String::new());
    let rol = RwSignal::new("member".to_string());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        if email.get_untracked().trim().is_empty() {
            error.set(Some("El email no puede estar vacío".to_string()));
            return;
        }

        guardando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let email_actual = email.get_untracked();
            let rol_actual = rol.get_untracked();
            let datos = admin::CrearInvitacionDatos {
                workspace_id: workspace_id.get_untracked(),
                email: &email_actual,
                role: &rol_actual,
            };
            let resultado = admin::crear_invitacion(&datos, &token).await;
            guardando.set(false);
            match resultado {
                Ok(invitacion) => {
                    email.set(String::new());
                    on_creada(invitacion);
                }
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=guardar>
            <div class="form-grid">
                <div class="field">
                    <label>"Tenant"</label>
                    <select
                        prop:value=move || workspace_id.get().to_string()
                        on:change=move |ev| {
                            if let Ok(id) = event_target_value(&ev).parse() {
                                workspace_id.set(id);
                            }
                        }
                    >
                        {workspaces
                            .iter()
                            .map(|w| view! { <option value=w.id.to_string()>{w.name.clone()}</option> })
                            .collect_view()}
                    </select>
                </div>
                <div class="field">
                    <label>"Email a invitar"</label>
                    <input
                        r#type="email"
                        prop:value=move || email.get()
                        on:input=move |ev| email.set(event_target_value(&ev))
                        required
                    />
                </div>
                <div class="field">
                    <label>"Rol en el workspace"</label>
                    <select prop:value=move || rol.get() on:change=move |ev| rol.set(event_target_value(&ev))>
                        <option value="member">"Miembro"</option>
                        <option value="admin">"Admin"</option>
                    </select>
                </div>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="banner banner-error" style="margin-bottom:14px;">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            <div class="form-actions">
                <button type="submit" class="btn btn-primary" disabled=move || guardando.get()>
                    {move || if guardando.get() { "Generando..." } else { "Generar invitación" }}
                </button>
            </div>
        </form>
    }
}
