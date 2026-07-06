-- =============================================================
-- Módulo: auth — roles globales, bloqueo por fuerza bruta,
-- invitaciones de un solo uso y bitácora de auditoría.
-- =============================================================

-- Rol global del usuario:
--   'dev'     → acceso total: ve todos los workspaces, crea tenants, audita.
--   'usuario' → solo opera dentro de los workspaces que dev le asigna.
ALTER TABLE users
    ADD COLUMN role TEXT NOT NULL DEFAULT 'usuario' CHECK (role IN ('dev', 'usuario')),
    ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT true,
    -- Contador de logins fallidos consecutivos; se reinicia al entrar bien.
    ADD COLUMN failed_login_attempts INT NOT NULL DEFAULT 0,
    -- Si tiene valor y es futuro, la cuenta está bloqueada temporalmente.
    ADD COLUMN locked_until TIMESTAMPTZ;

-- Invitaciones: quién la creó y cuándo se usó (NULL = sigue disponible).
-- La columna token existente guardará el HASH del token, nunca el token plano.
ALTER TABLE workspace_invitations
    ADD COLUMN created_by UUID REFERENCES users(id),
    ADD COLUMN used_at TIMESTAMPTZ;

-- Bitácora de auditoría: acciones sensibles del sistema.
-- user_id es NULL cuando la acción no proviene de un usuario identificado
-- (p. ej. un intento de login con un email que no existe).
CREATE TABLE audit_log (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        REFERENCES users(id) ON DELETE SET NULL,
    action     TEXT        NOT NULL,
    detail     JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_log_created_at ON audit_log(created_at DESC);
CREATE INDEX idx_refresh_tokens_user  ON refresh_tokens(user_id);
