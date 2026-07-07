use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="auth-screen">
            <section class="auth-card" style="text-align: center;">
                <p class="figure" style="font-size: 28px; margin-bottom: 8px;">"404"</p>
                <p class="text-soft" style="margin-bottom: 18px;">"Esta página no existe."</p>
                <A href="/" attr:class="btn btn-primary btn-block">"Volver al inicio"</A>
            </section>
        </div>
    }
}
