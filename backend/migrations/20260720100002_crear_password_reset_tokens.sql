-- =============================================================
-- Módulo: auth — recuperación de contraseña por correo.
--
-- Mismo patrón que refresh_tokens/workspace_invitations: el valor
-- plano del token solo existe en el correo que recibe el usuario;
-- en la base se guarda únicamente su hash SHA-256.
-- =============================================================

CREATE TABLE password_reset_tokens (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT password_reset_tokens_hash_unique UNIQUE (token_hash)
);

CREATE INDEX idx_password_reset_tokens_user ON password_reset_tokens(user_id);
