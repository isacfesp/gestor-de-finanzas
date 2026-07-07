//! Modo claro/oscuro. La preferencia se guarda en localStorage; si el
//! usuario nunca la tocó, se usa la preferencia del sistema operativo.

use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;

const CLAVE_TEMA: &str = "gestor.tema";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tema {
    Claro,
    Oscuro,
}

impl Tema {
    fn como_texto(self) -> &'static str {
        match self {
            Tema::Claro => "light",
            Tema::Oscuro => "dark",
        }
    }
}

/// Refleja el tema en `<html data-theme="...">`, que es lo que lee
/// `styles/main.css` para decidir la paleta.
fn aplicar_al_documento(tema: Tema) {
    let Some(documento) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(raiz) = documento.document_element() {
        let _ = raiz.set_attribute("data-theme", tema.como_texto());
    }
}

fn preferencia_del_sistema() -> Tema {
    let prefiere_oscuro = web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
        .flatten()
        .map(|lista| lista.matches())
        .unwrap_or(false);

    if prefiere_oscuro {
        Tema::Oscuro
    } else {
        Tema::Claro
    }
}

#[derive(Copy, Clone)]
pub struct TemaContext(RwSignal<Tema>);

impl TemaContext {
    pub fn actual(&self) -> Tema {
        self.0.get()
    }

    pub fn alternar(&self) {
        let nuevo = if self.0.get_untracked() == Tema::Oscuro {
            Tema::Claro
        } else {
            Tema::Oscuro
        };
        let _ = LocalStorage::set(CLAVE_TEMA, nuevo.como_texto());
        aplicar_al_documento(nuevo);
        self.0.set(nuevo);
    }
}

/// Decide el tema inicial (guardado > preferencia del sistema), lo
/// aplica al documento y lo deja disponible por contexto. Se llama
/// una sola vez, en la raíz de `App`.
pub fn provide_theme_context() {
    let guardado = LocalStorage::get::<String>(CLAVE_TEMA).ok();
    let inicial = match guardado.as_deref() {
        Some("dark") => Tema::Oscuro,
        Some("light") => Tema::Claro,
        _ => preferencia_del_sistema(),
    };
    aplicar_al_documento(inicial);
    provide_context(TemaContext(RwSignal::new(inicial)));
}

pub fn use_theme() -> TemaContext {
    use_context::<TemaContext>()
        .expect("TemaContext no está disponible: falta provide_theme_context() en App")
}
