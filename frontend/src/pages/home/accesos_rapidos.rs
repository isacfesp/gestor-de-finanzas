//! Accesos rápidos: últimas transacciones y próximos eventos de agenda
//! — reusa las llamadas ya existentes de `accounting`/`agenda`, solo
//! recorta a un puñado de filas para la vista de resumen.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::{accounting, agenda};
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn AccesosRapidos(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();

    let recientes = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        accounting::listar_transacciones(
            workspace_id,
            &accounting::FiltrosTransacciones::default(),
            &token,
        )
        .await
        .map(|lista| lista.into_iter().take(5).collect::<Vec<_>>())
        .map_err(|e| e.to_string())
    });

    let proximos = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        let cobros = agenda::proximos_cobros(workspace_id, 30, &token)
            .await
            .unwrap_or_default();
        let filtros = agenda::FiltrosPrevistos {
            pagado: Some(false),
            ..Default::default()
        };
        let previstos = agenda::listar_previstos(workspace_id, &filtros, &token)
            .await
            .unwrap_or_default();
        Ok::<_, String>((cobros, previstos))
    });

    view! {
        <div style="display:grid; grid-template-columns:repeat(auto-fit,minmax(280px,1fr)); gap:16px; margin-top:16px;">
            <section class="panel">
                <div class="panel-head"><h3>"Transacciones recientes"</h3></div>
                {move || match recientes.get() {
                    None => view! { <p class="text-soft">"Cargando..."</p> }.into_any(),
                    Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                    Some(Ok(lista)) if lista.is_empty() => {
                        view! { <p class="text-soft">"Sin movimientos todavía."</p> }.into_any()
                    }
                    Some(Ok(lista)) => view! {
                        <div>
                            {lista
                                .into_iter()
                                .map(|t| {
                                    let signo = if t.tipo == "income" { "+" } else { "-" };
                                    let color = if t.tipo == "income" { "var(--positive)" } else { "var(--negative)" };
                                    view! {
                                        <div style="display:flex; justify-content:space-between; padding:6px 0; border-bottom:1px solid var(--line); font-size:13px;">
                                            <span>{t.date.to_string()}</span>
                                            <span style=format!("color:{color};")>{format!("{signo}{:.2}", t.amount)}</span>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                }}
            </section>
            <section class="panel">
                <div class="panel-head"><h3>"Próximos eventos"</h3></div>
                {move || match proximos.get() {
                    None => view! { <p class="text-soft">"Cargando..."</p> }.into_any(),
                    Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                    Some(Ok((cobros, previstos))) if cobros.is_empty() && previstos.is_empty() => {
                        view! { <p class="text-soft">"Nada próximo por ahora."</p> }.into_any()
                    }
                    Some(Ok((cobros, previstos))) => view! {
                        <div>
                            {cobros
                                .into_iter()
                                .take(5)
                                .map(|s| view! {
                                    <div style="display:flex; justify-content:space-between; padding:6px 0; border-bottom:1px solid var(--line); font-size:13px;">
                                        <span>{s.name.clone()}</span>
                                        <span class="text-soft">{s.next_billing_date.to_string()}</span>
                                    </div>
                                })
                                .collect_view()}
                            {previstos
                                .into_iter()
                                .take(5)
                                .map(|p| view! {
                                    <div style="display:flex; justify-content:space-between; padding:6px 0; border-bottom:1px solid var(--line); font-size:13px;">
                                        <span>{p.description.clone().unwrap_or_else(|| "Previsto".to_string())}</span>
                                        <span class="text-soft">{p.due_date.to_string()}</span>
                                    </div>
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                }}
            </section>
        </div>
    }
}
