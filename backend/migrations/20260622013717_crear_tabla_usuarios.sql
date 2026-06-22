-- =============================================================
-- Módulo: auth_workspace
-- =============================================================

CREATE TABLE users (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name          TEXT        NOT NULL,
    email         TEXT        NOT NULL,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT users_email_unique UNIQUE (email)
);

CREATE TABLE workspaces (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT        NOT NULL,
    owner_id   UUID        NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE workspace_members (
    workspace_id UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id      UUID        NOT NULL REFERENCES users(id)      ON DELETE CASCADE,
    role         TEXT        NOT NULL CHECK (role IN ('admin', 'member')),
    joined_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (workspace_id, user_id)
);

CREATE TABLE workspace_invitations (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id   UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    invited_email  TEXT        NOT NULL,
    role           TEXT        NOT NULL CHECK (role IN ('admin', 'member')),
    token          TEXT        NOT NULL,
    expires_at     TIMESTAMPTZ NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT workspace_invitations_token_unique UNIQUE (token)
);

CREATE TABLE refresh_tokens (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT refresh_tokens_hash_unique UNIQUE (token_hash)
);

-- =============================================================
-- Módulo: accounting
-- =============================================================

CREATE TABLE categories (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE, -- NULL = categoría global
    name         TEXT NOT NULL,
    type         TEXT NOT NULL CHECK (type IN ('income', 'expense')),

    CONSTRAINT categories_workspace_name_unique UNIQUE (workspace_id, name)
);

-- goals va antes que transactions porque transactions tiene FK opcional a goals
CREATE TABLE goals (
    id             UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id   UUID         NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name           TEXT         NOT NULL,
    target_amount  NUMERIC(15,2) NOT NULL CHECK (target_amount > 0),
    current_amount NUMERIC(15,2) NOT NULL DEFAULT 0 CHECK (current_amount >= 0),
    deadline       DATE         NOT NULL,
    is_completed   BOOLEAN      NOT NULL DEFAULT false,
    created_at     TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE transactions (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID         NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    type         TEXT         NOT NULL CHECK (type IN ('income', 'expense')),
    amount       NUMERIC(15,2) NOT NULL CHECK (amount > 0),
    date         DATE         NOT NULL,
    category_id  UUID         REFERENCES categories(id),
    description  TEXT,
    goal_id      UUID         REFERENCES goals(id),
    created_by   UUID         NOT NULL REFERENCES users(id),
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE subscriptions (
    id                UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id      UUID         NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name              TEXT         NOT NULL,
    amount            NUMERIC(15,2) NOT NULL CHECK (amount > 0),
    category_id       UUID         REFERENCES categories(id),
    periodicity       TEXT         NOT NULL CHECK (periodicity IN ('monthly', 'bimonthly', 'quarterly', 'annual')),
    next_billing_date DATE         NOT NULL,
    is_active         BOOLEAN      NOT NULL DEFAULT true,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE budgets (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID         NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    category_id  UUID         NOT NULL REFERENCES categories(id),
    month        DATE         NOT NULL, -- siempre el primer día del mes: 2026-06-01
    limit_amount NUMERIC(15,2) NOT NULL CHECK (limit_amount > 0),

    CONSTRAINT budgets_workspace_category_month_unique UNIQUE (workspace_id, category_id, month)
);

-- =============================================================
-- Módulo: investments
-- =============================================================

CREATE TABLE investments (
    id             UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id   UUID         NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name           TEXT         NOT NULL,
    principal      NUMERIC(15,2) NOT NULL CHECK (principal > 0),
    gat_annual_rate NUMERIC(7,4) NOT NULL CHECK (gat_annual_rate > 0),
    interest_type  TEXT         NOT NULL CHECK (interest_type IN ('simple', 'compound')),
    start_date     DATE         NOT NULL,
    term_days      INTEGER      NOT NULL CHECK (term_days > 0),
    end_date       DATE         NOT NULL,
    is_active      BOOLEAN      NOT NULL DEFAULT true,
    created_at     TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- =============================================================
-- Módulo: reminders
-- =============================================================

CREATE TABLE notifications (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    type         TEXT        NOT NULL CHECK (type IN ('subscription_due', 'budget_80', 'budget_100', 'goal_deadline')),
    title        TEXT        NOT NULL,
    body         TEXT        NOT NULL,
    reference_id UUID,       -- ID del recurso que originó la alerta (subscription, budget, goal)
    is_read      BOOLEAN     NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- =============================================================
-- Índices para consultas frecuentes
-- =============================================================

CREATE INDEX idx_transactions_workspace_date  ON transactions(workspace_id, date DESC);
CREATE INDEX idx_subscriptions_billing_date   ON subscriptions(workspace_id, next_billing_date);
CREATE INDEX idx_notifications_unread         ON notifications(workspace_id, is_read) WHERE is_read = false;
CREATE INDEX idx_budgets_workspace_month      ON budgets(workspace_id, month);
