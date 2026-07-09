//! Auditoría rápida: últimos movimientos del workspace (backend
//! `movimientos`), vista reducida — el listado completo vive en la
//! página "Movimientos".

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::movimientos::{self, etiqueta_accion};
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn AuditoriaRapida(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();

    let ultimos = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        movimientos::listar_movimientos(workspace_id, Some(5), None, &token)
            .await
            .map_err(|e| e.to_string())
    });

    view! {
        <section class="panel" style="margin-top:16px;">
            <div class="panel-head">
                <h3>"Auditoría rápida"</h3>
                <a href="/movimientos" class="btn-ghost" style="padding:6px 12px; font-size:12px;">
                    "Ver todo"
                </a>
            </div>
            {move || match ultimos.get() {
                None => view! { <p class="text-soft">"Cargando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"Todavía no hay actividad registrada."</p> }.into_any()
                }
                Some(Ok(lista)) => view! {
                    <div>
                        {lista
                            .into_iter()
                            .map(|m| {
                                let hora = m.created_at.format("%d/%m %H:%M").to_string();
                                view! {
                                    <div style="display:flex; justify-content:space-between; gap:10px; padding:6px 0; border-bottom:1px solid var(--line); font-size:13px;">
                                        <span>{etiqueta_accion(&m.action).to_string()} " — " <span class="text-soft">{m.actor_name.clone()}</span></span>
                                        <span class="text-faint" style="flex:none;">{hora}</span>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any(),
            }}
        </section>
    }
}
