//! Componente raíz: provee los contextos globales (auth, tema) y
//! define el árbol de rutas completo de la aplicación.

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::components::ProtectedShell;
use crate::components::theme::provide_theme_context;
use crate::pages::{AdminPage, Home, Login, NotFound, PerfilPage, modulos};

#[component]
pub fn App() -> impl IntoView {
    crate::auth::provide_auth_context();
    provide_theme_context();
    crate::workspace::provide_workspace_context();

    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/login") view=Login/>

                // Todo lo que cuelga de "/" pasa primero por ProtectedShell,
                // que exige sesión y dibuja el encabezado + navegación antes
                // de mostrar la página pedida en el <Outlet/>. Las 5
                // secciones de navegación son las de docs/frontend-ia.md,
                // no un mapeo 1:1 con los módulos del backend.
                <ParentRoute path=path!("/") view=ProtectedShell>
                    <Route path=path!("") view=Home/>
                    <Route path=path!("cuentas") view=modulos::CuentasPage/>
                    <Route path=path!("agenda") view=modulos::AgendaPage/>
                    <Route path=path!("inversiones") view=modulos::InversionesPage/>
                    <Route path=path!("movimientos") view=modulos::MovimientosPage/>
                    <Route path=path!("admin") view=AdminPage/>
                    <Route path=path!("perfil") view=PerfilPage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}
