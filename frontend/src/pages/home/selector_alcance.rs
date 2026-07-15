//! Selector "Todo el workspace / Yo / <miembro>" — solo tiene sentido
//! para un dev global (`AuthContext::es_dev`). Para cualquier otro
//! usuario las métricas ya son personales por defecto en el backend
//! (ver `analytics::comun::resolver_filtro_usuario`), así que ni
//! siquiera hace falta mostrarlo.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::admin;
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn SelectorAlcance(workspace_id: Uuid, alcance: RwSignal<Option<Uuid>>) -> impl IntoView {
    let auth = use_auth();

    let miembros = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        admin::listar_miembros(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    view! {
        <div class="field" style="max-width:220px; margin-bottom:16px;">
            <label>"Ver métricas de"</label>
            <select
                on:change=move |ev| {
                    let valor = event_target_value(&ev);
                    alcance.set(Uuid::parse_str(&valor).ok());
                }
            >
                <option value="">"Todo el workspace"</option>
                {move || {
                    auth.usuario()
                        .map(|u| {
                            view! { <option value=u.id.to_string()>"Yo"</option> }
                        })
                }}
                {move || {
                    miembros
                        .get()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|m| {
                            let id = m.user_id.to_string();
                            view! { <option value=id.clone()>{m.name}</option> }
                        })
                        .collect_view()
                }}
            </select>
        </div>
    }
}
