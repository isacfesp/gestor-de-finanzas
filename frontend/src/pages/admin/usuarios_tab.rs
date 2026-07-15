//! Pestaña "Usuarios": alta de usuarios y activar/desactivar cuentas
//! (backend `admin`, todo `SoloDev`).

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use uuid::Uuid;

use crate::api::admin::{self, UsuarioAdmin};
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn PestanaUsuarios() -> impl IntoView {
    let auth = use_auth();
    let mostrar_form = RwSignal::new(false);

    let usuarios = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        admin::listar_usuarios(&token)
            .await
            .map_err(|e| e.to_string())
    });

    let alternar_estado = move |u: UsuarioAdmin| {
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let resultado = if u.is_active {
                    admin::desactivar_usuario(u.id, &token).await
                } else {
                    admin::reactivar_usuario(u.id, &token).await
                };
                if resultado.is_ok() {
                    usuarios.refetch();
                }
            }
        });
    };

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Usuarios"</h2>
                <button
                    class="btn btn-primary"
                    style="padding:8px 15px; font-size:12.5px;"
                    on:click=move |_| mostrar_form.set(true)
                >
                    "+ Nuevo usuario"
                </button>
            </div>

            <Show when=move || mostrar_form.get()>
                <FormularioUsuario
                    on_guardado=move || { mostrar_form.set(false); usuarios.refetch(); }
                    on_cancelar=move || mostrar_form.set(false)
                />
            </Show>

            {move || match usuarios.get() {
                None => view! { <p class="text-soft">"Cargando usuarios..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"Todavía no hay usuarios."</p> }.into_any()
                }
                Some(Ok(lista)) => view! {
                    <div class="table-scroll">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"Nombre"</th>
                                    <th>"Email"</th>
                                    <th>"Rol"</th>
                                    <th>"Estado"</th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {lista
                                    .into_iter()
                                    .map(|u| {
                                        let para_alternar = u.clone();
                                        view! {
                                            <tr>
                                                <td>{u.name.clone()}</td>
                                                <td>{u.email.clone()}</td>
                                                <td>{etiqueta_rol(&u.role).to_string()}</td>
                                                <td>
                                                    <span style=if u.is_active {
                                                        "color:var(--positive); font-weight:600;"
                                                    } else {
                                                        "color:var(--negative); font-weight:600;"
                                                    }>
                                                        {if u.is_active { "Activo" } else { "Inactivo" }}
                                                    </span>
                                                </td>
                                                <td>
                                                    <button
                                                        class="btn-ghost"
                                                        style="padding:4px 8px; font-size:11px;"
                                                        on:click=move |_| alternar_estado(para_alternar.clone())
                                                    >
                                                        {if u.is_active { "Desactivar" } else { "Reactivar" }}
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
        </section>
    }
}

fn etiqueta_rol(rol: &str) -> &str {
    match rol {
        "dev" => "Dev",
        _ => "Usuario",
    }
}

#[component]
fn FormularioUsuario<F1, F2>(on_guardado: F1, on_cancelar: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let nombre = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    // "Agregar a un workspace" es opcional: "" significa "no asignar
    // todavía" (el usuario queda creado, se asigna después desde
    // Tenants como hoy). Evita el viaje de ida y vuelta entre pestañas
    // cuando sí se sabe a qué workspace va.
    let workspace_id = RwSignal::new(String::new());
    let rol = RwSignal::new("member".to_string());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let workspaces = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        admin::listar_workspaces(&token).await.unwrap_or_default()
    });
    // Workspaces creados desde el "+ Nuevo workspace..." de este mismo
    // formulario: no viven en `workspaces` (ese resource no se
    // refresca) sino aquí, para que aparezcan de inmediato en el select.
    let workspaces_nuevos = RwSignal::new(Vec::<admin::WorkspaceCreado>::new());
    let mostrar_nuevo_workspace = RwSignal::new(false);
    let nombre_nuevo_workspace = RwSignal::new(String::new());
    let creando_workspace = RwSignal::new(false);
    let error_workspace = RwSignal::new(None::<String>);
    const OPCION_NUEVO_WORKSPACE: &str = "__nuevo__";

    let crear_workspace_rapido = move |_| {
        let valor = nombre_nuevo_workspace.get_untracked();
        if valor.trim().is_empty() || creando_workspace.get_untracked() {
            return;
        }
        error_workspace.set(None);
        creando_workspace.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error_workspace.set(Some("Sesión vencida".to_string()));
                creando_workspace.set(false);
                return;
            };
            let datos = admin::CrearWorkspaceDatos { name: valor.trim() };
            let resultado = admin::crear_workspace(&datos, &token).await;
            creando_workspace.set(false);
            match resultado {
                Ok(creado) => {
                    workspace_id.set(creado.id.to_string());
                    workspaces_nuevos.update(|lista| lista.push(creado));
                    nombre_nuevo_workspace.set(String::new());
                    mostrar_nuevo_workspace.set(false);
                }
                Err(error_api) => error_workspace.set(Some(error_api.to_string())),
            }
        });
    };

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        if nombre.get_untracked().trim().is_empty()
            || email.get_untracked().trim().is_empty()
            || password.get_untracked().is_empty()
        {
            error.set(Some("Todos los campos son obligatorios".to_string()));
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
            let email_actual = email.get_untracked();
            let password_actual = password.get_untracked();
            let datos = admin::CrearUsuarioDatos {
                name: &nombre_actual,
                email: &email_actual,
                password: &password_actual,
            };
            let resultado = admin::crear_usuario(&datos, &token).await;
            guardando.set(false);

            let Ok(workspace_elegido) = Uuid::parse_str(&workspace_id.get_untracked()) else {
                // Sin workspace elegido: el usuario se crea y queda
                // sin asignar, como antes de este cambio.
                match resultado {
                    Ok(_) => on_guardado(),
                    Err(error_api) => error.set(Some(error_api.to_string())),
                }
                return;
            };

            match resultado {
                Ok(_) => {
                    let datos_miembro = admin::AsignarMiembroDatos {
                        email: &email_actual,
                        role: &rol.get_untracked(),
                    };
                    match admin::asignar_miembro(workspace_elegido, &datos_miembro, &token).await {
                        Ok(_) => on_guardado(),
                        // El usuario ya se creó exitosamente — aclarar
                        // que solo falló la asignación, para no dar la
                        // impresión de que hay que reintentar todo.
                        Err(error_api) => error.set(Some(format!(
                            "Usuario creado, pero no se pudo asignar al workspace: {error_api}"
                        ))),
                    }
                }
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=guardar>
            <div class="form-grid">
                <div class="field">
                    <label>"Nombre"</label>
                    <input
                        prop:value=move || nombre.get()
                        on:input=move |ev| nombre.set(event_target_value(&ev))
                        required
                    />
                </div>
                <div class="field">
                    <label>"Email"</label>
                    <input
                        r#type="email"
                        prop:value=move || email.get()
                        on:input=move |ev| email.set(event_target_value(&ev))
                        required
                    />
                </div>
                <div class="field">
                    <label>"Contraseña"</label>
                    <input
                        r#type="password"
                        prop:value=move || password.get()
                        on:input=move |ev| password.set(event_target_value(&ev))
                        required
                    />
                </div>
                <div class="field">
                    <label>"Agregar a un workspace (opcional)"</label>
                    <select
                        prop:value=move || workspace_id.get()
                        on:change=move |ev| {
                            let valor = event_target_value(&ev);
                            if valor == OPCION_NUEVO_WORKSPACE {
                                mostrar_nuevo_workspace.set(true);
                            } else {
                                workspace_id.set(valor);
                            }
                        }
                    >
                        <option value="">"No asignar todavía"</option>
                        {move || {
                            workspaces
                                .get()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|w| {
                                    let id = w.id.to_string();
                                    view! { <option value=id.clone()>{w.name.clone()}</option> }
                                })
                                .collect_view()
                        }}
                        {move || {
                            workspaces_nuevos
                                .get()
                                .into_iter()
                                .map(|w| {
                                    let id = w.id.to_string();
                                    view! { <option value=id.clone()>{w.name.clone()}</option> }
                                })
                                .collect_view()
                        }}
                        <option value=OPCION_NUEVO_WORKSPACE>"+ Nuevo workspace..."</option>
                    </select>
                </div>
                <Show when=move || mostrar_nuevo_workspace.get()>
                    <div class="field">
                        <label>"Nombre del nuevo workspace"</label>
                        <div style="display:flex; gap:6px;">
                            <input
                                style="flex:1;"
                                prop:value=move || nombre_nuevo_workspace.get()
                                on:input=move |ev| nombre_nuevo_workspace.set(event_target_value(&ev))
                            />
                            <button
                                type="button"
                                class="btn-ghost"
                                disabled=move || creando_workspace.get()
                                on:click=crear_workspace_rapido
                            >
                                {move || if creando_workspace.get() { "Creando..." } else { "Crear" }}
                            </button>
                        </div>
                        <Show when=move || error_workspace.get().is_some()>
                            <p class="banner banner-error" style="margin-top:6px;">
                                {move || error_workspace.get().unwrap_or_default()}
                            </p>
                        </Show>
                    </div>
                </Show>
                <Show when=move || !workspace_id.get().is_empty()>
                    <div class="field">
                        <label>"Rol en el workspace"</label>
                        <select prop:value=move || rol.get() on:change=move |ev| rol.set(event_target_value(&ev))>
                            <option value="member">"Miembro"</option>
                            <option value="admin">"Admin"</option>
                        </select>
                    </div>
                </Show>
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
                    {move || if guardando.get() { "Guardando..." } else { "Crear usuario" }}
                </button>
            </div>
        </form>
    }
}
