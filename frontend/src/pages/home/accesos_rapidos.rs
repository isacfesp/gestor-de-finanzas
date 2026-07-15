//! Accesos rápidos: últimas transacciones y próximos eventos de agenda
//! — reusa las llamadas ya existentes de `accounting`/`agenda`, solo
//! recorta a un puñado de filas para la vista de resumen.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::{accounting, agenda};
use crate::auth::{token_vigente, use_auth};

fn nombre_categoria(categorias: &[accounting::Categoria], id: Option<Uuid>) -> String {
    id.and_then(|id| categorias.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Sin categoría".to_string())
}

#[component]
pub fn AccesosRapidos(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();

    let categorias = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounting::listar_categorias(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

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
                {move || {
                    let categorias_ok = categorias.get().unwrap_or_default();
                    match proximos.get() {
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
                                    .map(|s| {
                                        let categoria = nombre_categoria(&categorias_ok, s.category_id);
                                        view! {
                                            <div style="padding:6px 0; border-bottom:1px solid var(--line); font-size:13px;">
                                                <div style="display:flex; justify-content:space-between;">
                                                    <span>{s.name.clone()}</span>
                                                    <span class="num" style="color:var(--negative);">
                                                        {format!("-{:.2}", s.amount)}
                                                    </span>
                                                </div>
                                                <div style="display:flex; justify-content:space-between; margin-top:3px;">
                                                    <span class="chip">"Suscripción"</span>
                                                    <span class="text-soft">{categoria} " · " {s.next_billing_date.to_string()}</span>
                                                </div>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                                {previstos
                                    .into_iter()
                                    .take(5)
                                    .map(|p| {
                                        let categoria = nombre_categoria(&categorias_ok, p.category_id);
                                        let (etiqueta_tipo, color, signo) = if p.tipo == "income" {
                                            ("Ingreso previsto", "var(--positive)", "+")
                                        } else {
                                            ("Egreso previsto", "var(--negative)", "-")
                                        };
                                        view! {
                                            <div style="padding:6px 0; border-bottom:1px solid var(--line); font-size:13px;">
                                                <div style="display:flex; justify-content:space-between;">
                                                    <span>{p.description.clone().unwrap_or_else(|| "Previsto".to_string())}</span>
                                                    <span class="num" style=format!("color:{color};")>
                                                        {format!("{signo}{:.2}", p.amount)}
                                                    </span>
                                                </div>
                                                <div style="display:flex; justify-content:space-between; margin-top:3px;">
                                                    <span class="chip">{etiqueta_tipo}</span>
                                                    <span class="text-soft">{categoria} " · " {p.due_date.to_string()}</span>
                                                </div>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                        .into_any(),
                    }
                }}
            </section>
        </div>
    }
}
