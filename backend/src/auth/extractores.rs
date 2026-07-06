// =====================================================================
// extractores.rs — Extractores de autenticación para Axum.
//
// Un extractor es un tipo que Axum sabe construir a partir de la
// petición HTTP. Si un handler pide `UsuarioAutenticado` en su firma,
// Axum ejecuta este código ANTES del handler: si el token no es
// válido, el handler ni siquiera se ejecuta y el cliente recibe 401.
// Así la autenticación queda declarada en la firma, imposible de olvidar.
// =====================================================================

use axum::{
    async_trait, extract::FromRequestParts, http::header::AUTHORIZATION, http::request::Parts,
};
use uuid::Uuid;

use crate::auth::jwt::validar_access_token;
use crate::errores::AppError;

/// Rol global del sistema (columna users.role).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolGlobal {
    /// Acceso total: ve todos los workspaces, crea tenants, audita.
    Dev,
    /// Solo opera dentro de los workspaces que le fueron asignados.
    Usuario,
}

impl RolGlobal {
    pub fn desde_texto(texto: &str) -> Result<Self, AppError> {
        match texto {
            "dev" => Ok(RolGlobal::Dev),
            "usuario" => Ok(RolGlobal::Usuario),
            otro => Err(AppError::Interno(format!("Rol desconocido: {otro}"))),
        }
    }

    // allow(dead_code): contraparte de desde_texto; la usarán los
    // módulos de datos al construir respuestas.
    #[allow(dead_code)]
    pub fn como_texto(&self) -> &'static str {
        match self {
            RolGlobal::Dev => "dev",
            RolGlobal::Usuario => "usuario",
        }
    }
}

/// Usuario que hizo la petición, ya autenticado vía JWT.
pub struct UsuarioAutenticado {
    pub id: Uuid,
    pub rol: RolGlobal,
}

impl UsuarioAutenticado {
    pub fn es_dev(&self) -> bool {
        self.rol == RolGlobal::Dev
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UsuarioAutenticado
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _estado: &S) -> Result<Self, Self::Rejection> {
        // Se espera el header estándar: Authorization: Bearer <token>
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::NoAutorizado("Falta el header Authorization".to_string()))?;

        let token = header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::NoAutorizado("Formato esperado: Authorization: Bearer <token>".to_string())
        })?;

        let claims = validar_access_token(token)?;

        Ok(UsuarioAutenticado {
            id: claims.sub,
            rol: RolGlobal::desde_texto(&claims.rol)?,
        })
    }
}

/// Extractor que además exige rol dev. Las rutas de administración lo
/// piden en su firma: un usuario normal recibe 403 automáticamente.
pub struct SoloDev(pub UsuarioAutenticado);

#[async_trait]
impl<S> FromRequestParts<S> for SoloDev
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, estado: &S) -> Result<Self, Self::Rejection> {
        // Reutiliza el extractor anterior y solo agrega la verificación de rol.
        let usuario = UsuarioAutenticado::from_request_parts(parts, estado).await?;
        if !usuario.es_dev() {
            return Err(AppError::Prohibido(
                "Esta operación requiere rol dev".to_string(),
            ));
        }
        Ok(SoloDev(usuario))
    }
}
