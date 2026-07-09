//! Página "Admin" (dev-only): panel de administración con las 8
//! acciones de `/admin` — crear/listar tenants, crear/listar/activar
//! usuarios, asignar/eliminar miembros, generar invitaciones y ver la
//! auditoría global. No es una de las 5 secciones financieras de
//! `docs/frontend-ia.md`, así que vive fuera de `pages/modulos/` y
//! trae su propio guardia de rol (`ProtectedShell` solo exige sesión,
//! no rol dev).

use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::auth::use_auth;

mod auditoria_tab;
mod invitaciones_tab;
mod tenants_tab;
mod usuarios_tab;
mod util;

use auditoria_tab::PestanaAuditoria;
use invitaciones_tab::PestanaInvitaciones;
use tenants_tab::PestanaTenants;
use usuarios_tab::PestanaUsuarios;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Pestana {
    Tenants,
    Usuarios,
    Invitaciones,
    Auditoria,
}

#[component]
pub fn AdminPage() -> impl IntoView {
    let auth = use_auth();
    let pestana = RwSignal::new(Pestana::Tenants);

    view! {
        <Show when=move || auth.es_dev() fallback=|| view! { <Redirect path="/"/> }>
            <BarraPestanas pestana=pestana/>
            <Show when=move || pestana.get() == Pestana::Tenants>
                <PestanaTenants/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Usuarios>
                <PestanaUsuarios/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Invitaciones>
                <PestanaInvitaciones/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Auditoria>
                <PestanaAuditoria/>
            </Show>
        </Show>
    }
}

#[component]
fn BarraPestanas(pestana: RwSignal<Pestana>) -> impl IntoView {
    let clase = move |p: Pestana| {
        if pestana.get() == p {
            "tab-btn is-active"
        } else {
            "tab-btn"
        }
    };

    view! {
        <div class="tabs">
            <button class=move || clase(Pestana::Tenants) on:click=move |_| pestana.set(Pestana::Tenants)>
                "Tenants"
            </button>
            <button class=move || clase(Pestana::Usuarios) on:click=move |_| pestana.set(Pestana::Usuarios)>
                "Usuarios"
            </button>
            <button class=move || clase(Pestana::Invitaciones) on:click=move |_| pestana.set(Pestana::Invitaciones)>
                "Invitaciones"
            </button>
            <button class=move || clase(Pestana::Auditoria) on:click=move |_| pestana.set(Pestana::Auditoria)>
                "Auditoría"
            </button>
        </div>
    }
}
