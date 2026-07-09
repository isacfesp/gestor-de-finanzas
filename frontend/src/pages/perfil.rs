//! Página "Perfil": datos propios de la sesión (nombre, email, rol,
//! workspace activo). Sin llamadas nuevas al backend — todo sale de
//! `AuthContext`/`WorkspaceContext`, ya cargados por `ProtectedShell`.
//! No es una de las 5 secciones financieras de `docs/frontend-ia.md`,
//! así que vive fuera de `pages/modulos/`.

use leptos::prelude::*;

use crate::auth::use_auth;
use crate::workspace::use_workspace;

#[component]
pub fn PerfilPage() -> impl IntoView {
    let auth = use_auth();
    let workspace = use_workspace();

    view! {
        <section class="panel" style="padding: 22px 20px; max-width:420px;">
            <p class="eyebrow">"Perfil"</p>
            {move || {
                auth.usuario()
                    .map(|u| {
                        view! {
                            <p class="figure" style="font-size: 24px; margin: 6px 0 4px;">{u.name}</p>
                            <p class="text-soft">{u.email}</p>
                            <p class="text-soft" style="margin-top:4px;">
                                "Rol: " {if u.role == "dev" { "Dev" } else { "Usuario" }}
                            </p>
                            <p class="text-soft" style="margin-top:4px;">
                                "Workspace activo: " {workspace.nombre().unwrap_or_else(|| "—".to_string())}
                            </p>
                        }
                            .into_any()
                    })
                    .unwrap_or_else(|| view! { <p class="text-soft">"Cargando..."</p> }.into_any())
            }}
        </section>
    }
}
