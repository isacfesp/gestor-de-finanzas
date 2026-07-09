//! `TarjetaDesglose`: pinta un `investments::DesgloseRendimiento`.
//! La usan tanto `simulador_tab` (resultado sin persistir) como
//! `inversiones_tab::DetalleInversion` (proyección de una inversión
//! real) — es la misma forma de datos en ambos casos, ver
//! `backend/src/investments/models.rs::DesgloseRendimiento`.

use leptos::prelude::*;

use crate::api::investments::{self, DesgloseRendimiento};

#[component]
pub fn TarjetaDesglose(desglose: DesgloseRendimiento) -> impl IntoView {
    view! {
        <div class="panel" style="background:var(--accent-soft); border-color:transparent;">
            <div style="display:grid; grid-template-columns:repeat(auto-fit,minmax(140px,1fr)); gap:14px;">
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"Capital"</p>
                    <p style="margin:2px 0 0; font-size:16px;">{format!("{:.2}", desglose.principal)}</p>
                </div>
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"Tasa GAT anual"</p>
                    <p style="margin:2px 0 0; font-size:16px;">{format!("{:.2}%", desglose.gat_annual_rate)}</p>
                </div>
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"Tipo de interés"</p>
                    <p style="margin:2px 0 0; font-size:16px;">
                        {investments::etiqueta_tipo_interes(&desglose.interest_type)}
                    </p>
                </div>
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"Plazo"</p>
                    <p style="margin:2px 0 0; font-size:16px;">{format!("{} días", desglose.term_days)}</p>
                </div>
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"Rendimiento bruto"</p>
                    <p style="margin:2px 0 0; font-size:16px; color:var(--text);">
                        {format!("{:.2}", desglose.gross_yield)}
                    </p>
                </div>
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"ISR retenido"</p>
                    <p style="margin:2px 0 0; font-size:16px; color:var(--negative);">
                        {format!("-{:.2}", desglose.isr_amount)}
                    </p>
                </div>
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"Rendimiento neto"</p>
                    <p style="margin:2px 0 0; font-size:16px; color:var(--positive);">
                        {format!("{:.2}", desglose.net_yield)}
                    </p>
                </div>
                <div>
                    <p class="text-faint" style="margin:0; font-size:11px;">"Monto al vencimiento"</p>
                    <p style="margin:2px 0 0; font-size:18px; font-weight:600;">
                        {format!("{:.2}", desglose.maturity_amount)}
                    </p>
                </div>
            </div>
        </div>
    }
}
