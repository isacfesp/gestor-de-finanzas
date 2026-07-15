//! Pestaña "Tenants": alta de workspaces y gestión de sus miembros
//! (backend `admin`, todo `SoloDev`).

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use uuid::Uuid;

use super::util::confirmar;
use crate::api::admin::{self, MiembroWorkspace, UsuarioAdmin, Workspace};
use crate::auth::{token_vigente, use_auth};

#[derive(Clone)]
enum Vista {
    Lista,
    Detalle(Workspace),
}

#[component]
pub fn PestanaTenants() -> impl IntoView {
    let auth = use_auth();
    let vista = RwSignal::new(Vista::Lista);
    let mostrar_form = RwSignal::new(false);

    let tenants = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        admin::listar_workspaces(&token)
            .await
            .map_err(|e| e.to_string())
    });

    let volver = move || {
        vista.set(Vista::Lista);
        tenants.refetch();
    };

    view! {
        <section class="panel">
            {move || match vista.get() {
                Vista::Detalle(ws) => view! {
                    <DetalleTenant workspace=ws on_volver=volver/>
                }
                .into_any(),
                Vista::Lista => view! {
                    <div>
                        <div class="panel-head">
                            <h2>"Tenants"</h2>
                            <button
                                class="btn btn-primary"
                                style="padding:8px 15px; font-size:12.5px;"
                                on:click=move |_| mostrar_form.set(true)
                            >
                                "+ Nuevo tenant"
                            </button>
                        </div>

                        <Show when=move || mostrar_form.get()>
                            <FormularioTenant
                                on_guardado=move || { mostrar_form.set(false); tenants.refetch(); }
                                on_cancelar=move || mostrar_form.set(false)
                            />
                        </Show>

                        {move || match tenants.get() {
                            None => view! { <p class="text-soft">"Cargando tenants..."</p> }.into_any(),
                            Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                            Some(Ok(lista)) if lista.is_empty() => {
                                view! { <p class="text-soft">"Todavía no hay ningún tenant."</p> }.into_any()
                            }
                            Some(Ok(lista)) => view! {
                                <div class="table-scroll">
                                    <table class="data-table">
                                        <thead>
                                            <tr>
                                                <th>"Nombre"</th>
                                                <th>"Creado"</th>
                                                <th>"Miembros"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {lista
                                                .into_iter()
                                                .map(|ws| {
                                                    let para_detalle = ws.clone();
                                                    view! {
                                                        <tr>
                                                            <td>{ws.name.clone()}</td>
                                                            <td>{ws.created_at.format("%d/%m/%Y").to_string()}</td>
                                                            <td class="num">{ws.miembros}</td>
                                                            <td>
                                                                <button
                                                                    class="btn-ghost"
                                                                    style="padding:4px 8px; font-size:11px;"
                                                                    on:click=move |_| vista.set(Vista::Detalle(para_detalle.clone()))
                                                                >
                                                                    "Ver miembros"
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                            }
                            .into_any(),
                        }}
                    </div>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn FormularioTenant<F1, F2>(on_guardado: F1, on_cancelar: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let nombre = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        if nombre.get_untracked().trim().is_empty() {
            error.set(Some("El nombre no puede estar vacío".to_string()));
            return;
        }

        guardando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let nombre_actual = nombre.get_untracked();
            let datos = admin::CrearWorkspaceDatos {
                name: &nombre_actual,
            };
            let resultado = admin::crear_workspace(&datos, &token).await;
            guardando.set(false);
            match resultado {
                Ok(_) => on_guardado(),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=guardar>
            <div class="form-grid">
                <div class="field">
                    <label>"Nombre del tenant"</label>
                    <input
                        prop:value=move || nombre.get()
                        on:input=move |ev| nombre.set(event_target_value(&ev))
                        required
                    />
                </div>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="banner banner-error" style="margin-bottom:14px;">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            <div class="form-actions">
                <button type="button" class="btn-ghost" on:click=move |_| on_cancelar()>
                    "Cancelar"
                </button>
                <button type="submit" class="btn btn-primary" disabled=move || guardando.get()>
                    {move || if guardando.get() { "Guardando..." } else { "Crear tenant" }}
                </button>
            </div>
        </form>
    }
}

#[component]
fn DetalleTenant<FV>(workspace: Workspace, on_volver: FV) -> impl IntoView
where
    FV: Fn() + 'static + Copy,
{
    let auth = use_auth();
    let workspace_id = workspace.id;
    let mostrar_form = RwSignal::new(false);

    let miembros = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        admin::listar_miembros(workspace_id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    let eliminar = move |m: MiembroWorkspace| {
        if !confirmar(&format!("¿Quitar a {} del tenant?", m.name)) {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = admin::eliminar_miembro(workspace_id, m.user_id, &token).await;
                miembros.refetch();
            }
        });
    };

    view! {
        <div>
            <div class="panel-head">
                <button class="btn-ghost" on:click=move |_| on_volver()>"← Volver"</button>
                <h2>{workspace.name.clone()}</h2>
                <button
                    class="btn btn-primary"
                    style="padding:8px 15px; font-size:12.5px;"
                    on:click=move |_| mostrar_form.set(true)
                >
                    "+ Asignar miembro"
                </button>
            </div>

            <Show when=move || mostrar_form.get()>
                <FormularioMiembro
                    workspace_id=workspace_id
                    excluir_ids={
                        miembros
                            .get()
                            .and_then(|r| r.ok())
                            .unwrap_or_default()
                            .iter()
                            .map(|m| m.user_id)
                            .collect::<Vec<_>>()
                    }
                    on_guardado=move || { mostrar_form.set(false); miembros.refetch(); }
                    on_cancelar=move || mostrar_form.set(false)
                />
            </Show>

            {move || match miembros.get() {
                None => view! { <p class="text-soft">"Cargando miembros..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"Este tenant todavía no tiene miembros."</p> }.into_any()
                }
                Some(Ok(lista)) => view! {
                    <div class="table-scroll">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"Nombre"</th>
                                    <th>"Email"</th>
                                    <th>"Rol"</th>
                                    <th>"Ingresó"</th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {lista
                                    .into_iter()
                                    .map(|m| {
                                        let para_eliminar = m.clone();
                                        view! {
                                            <tr>
                                                <td>{m.name.clone()}</td>
                                                <td>{m.email.clone()}</td>
                                                <td>{m.role.clone()}</td>
                                                <td>{m.joined_at.format("%d/%m/%Y").to_string()}</td>
                                                <td>
                                                    <button
                                                        class="btn-ghost"
                                                        style="padding:4px 8px; font-size:11px; color:var(--negative);"
                                                        on:click=move |_| eliminar(para_eliminar.clone())
                                                    >
                                                        "Quitar"
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    </div>
                }
                .into_any(),
            }}
        </div>
    }
}

#[component]
fn FormularioMiembro<F1, F2>(
    workspace_id: Uuid,
    /// Usuarios que ya son miembros de este tenant — se excluyen de las
    /// sugerencias del `<datalist>` para no ofrecer reasignar a alguien
    /// que ya está adentro.
    excluir_ids: Vec<Uuid>,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let email = RwSignal::new(String::new());
    let rol = RwSignal::new("member".to_string());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    // Alimenta el `<datalist>` del campo de email: usuarios activos que
    // todavía no pertenecen a este tenant, para poder elegirlos por
    // nombre en vez de tener que teclear el email exacto de memoria.
    let usuarios = LocalResource::new(move || {
        let excluidos = excluir_ids.clone();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Vec::<UsuarioAdmin>::new();
            };
            admin::listar_usuarios(&token)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|u| u.is_active && !excluidos.contains(&u.id))
                .collect()
        }
    });

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
            let datos = admin::AsignarMiembroDatos {
                email: &email_actual,
                role: &rol_actual,
            };
            let resultado = admin::asignar_miembro(workspace_id, &datos, &token).await;
            guardando.set(false);
            match resultado {
                Ok(_) => on_guardado(),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=guardar>
            <div class="form-grid">
                <div class="field">
                    <label>"Email del usuario"</label>
                    <input
                        r#type="email"
                        list="usuarios-existentes"
                        prop:value=move || email.get()
                        on:input=move |ev| email.set(event_target_value(&ev))
                        required
                    />
                    <datalist id="usuarios-existentes">
                        {move || {
                            usuarios
                                .get()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|u| view! { <option value=u.email.clone() label=u.name.clone()></option> })
                                .collect_view()
                        }}
                    </datalist>
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
                <button type="button" class="btn-ghost" on:click=move |_| on_cancelar()>
                    "Cancelar"
                </button>
                <button type="submit" class="btn btn-primary" disabled=move || guardando.get()>
                    {move || if guardando.get() { "Guardando..." } else { "Asignar" }}
                </button>
            </div>
        </form>
    }
}
