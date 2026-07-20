//! Alerta de tarjeta de crédito visible en el Dashboard (no solo la
//! campana de notificaciones): tarjetas propias cuya fecha de corte o
//! de pago límite está próxima (backend `accounts::alertas_tarjeta`,
//! siempre refleja el estado actual, sin depender de si ya se generó o
//! leyó un aviso).

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::accounts;
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn AlertasTarjetas(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();

    let alertas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounts::listar_alertas_tarjeta(workspace_id, None, &token)
            .await
            .unwrap_or_default()
    });

    view! {
        <Show when=move || !alertas.get().unwrap_or_default().is_empty()>
            <div style="margin-top:16px; display:flex; flex-direction:column; gap:10px;">
                {move || {
                    alertas
                        .get()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|a| {
                            view! {
                                <p class="banner banner-warning">
                                    <strong>{a.account_name.clone()}</strong> " · corte: " {a.cutoff_date.to_string()}
                                    " · pago límite: " {a.payment_due_date.to_string()}
                                    " · " {format!("{} {:.2}", a.currency, a.balance)}
                                </p>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </Show>
    }
}
