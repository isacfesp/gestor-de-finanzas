-- =============================================================
-- Módulo: accounts — Cuentas y billeteras
-- El dinero siempre sale o entra de un lugar concreto.
-- Esta tabla representa ese lugar (efectivo, banco, tarjeta, etc.)
-- =============================================================

CREATE TABLE accounts (
    id           UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID          NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name         TEXT          NOT NULL,
    type         TEXT          NOT NULL CHECK (type IN ('cash', 'debit', 'credit', 'savings', 'investment')),
    balance      NUMERIC(15,2) NOT NULL DEFAULT 0,
    currency     TEXT          NOT NULL DEFAULT 'MXN',
    is_active    BOOLEAN       NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ   NOT NULL DEFAULT now(),

    CONSTRAINT accounts_workspace_name_unique UNIQUE (workspace_id, name)
);

-- =============================================================
-- Módulo: transfers — Transferencias entre cuentas
-- Mover dinero de una cuenta a otra no es ingreso ni egreso.
-- Sin esta tabla, el balance total del workspace se inflaría.
-- =============================================================

CREATE TABLE transfers (
    id              UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID          NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    from_account_id UUID          NOT NULL REFERENCES accounts(id),
    to_account_id   UUID          NOT NULL REFERENCES accounts(id),
    amount          NUMERIC(15,2) NOT NULL CHECK (amount > 0),
    date            DATE          NOT NULL,
    description     TEXT,
    created_by      UUID          NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ   NOT NULL DEFAULT now(),

    -- Una cuenta no puede transferirse a sí misma
    CONSTRAINT transfers_different_accounts CHECK (from_account_id != to_account_id)
);

-- =============================================================
-- Módulo: planned_transactions — Pagos e ingresos previstos
-- Para flujo de caja proyectado: facturas futuras, cobros pendientes.
-- Distinto de subscriptions (recurrentes) — estos son eventos únicos.
-- =============================================================

CREATE TABLE planned_transactions (
    id           UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID          NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    type         TEXT          NOT NULL CHECK (type IN ('income', 'expense')),
    amount       NUMERIC(15,2) NOT NULL CHECK (amount > 0),
    due_date     DATE          NOT NULL,
    category_id  UUID          REFERENCES categories(id),
    account_id   UUID          REFERENCES accounts(id),
    description  TEXT,
    is_paid      BOOLEAN       NOT NULL DEFAULT false,
    created_by   UUID          NOT NULL REFERENCES users(id),
    created_at   TIMESTAMPTZ   NOT NULL DEFAULT now()
);

-- =============================================================
-- Módulo: tags — Etiquetas libres para transacciones
-- Las categorías son rígidas (income/expense). Las etiquetas
-- permiten agrupar transacciones de forma cruzada y flexible.
-- Ejemplo: "Viaje a Cancún" puede tener vuelos, hotel y comida
-- de distintas categorías pero todos con la misma etiqueta.
-- =============================================================

CREATE TABLE tags (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,

    CONSTRAINT tags_workspace_name_unique UNIQUE (workspace_id, name)
);

-- Tabla intermedia: relación muchos-a-muchos entre transacciones y etiquetas.
-- Una transacción puede tener varias etiquetas y una etiqueta puede
-- estar en varias transacciones.
CREATE TABLE transaction_tags (
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    tag_id         UUID NOT NULL REFERENCES tags(id)         ON DELETE CASCADE,

    PRIMARY KEY (transaction_id, tag_id)
);

-- =============================================================
-- Módulo: investment_yields — Historial de rendimientos
-- La tabla investments solo guarda el capital y la tasa.
-- Esta tabla registra cuándo y cuánto rindió realmente cada
-- inversión, permitiendo graficar el crecimiento en el tiempo.
-- =============================================================

CREATE TABLE investment_yields (
    id            UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    investment_id UUID          NOT NULL REFERENCES investments(id) ON DELETE CASCADE,
    yield_amount  NUMERIC(15,2) NOT NULL CHECK (yield_amount > 0),
    yield_date    DATE          NOT NULL,
    notes         TEXT,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

-- =============================================================
-- Índices para las nuevas tablas
-- =============================================================

CREATE INDEX idx_accounts_workspace      ON accounts(workspace_id);
CREATE INDEX idx_transfers_workspace     ON transfers(workspace_id, date DESC);
CREATE INDEX idx_planned_due_date        ON planned_transactions(workspace_id, due_date);
CREATE INDEX idx_planned_unpaid          ON planned_transactions(workspace_id, is_paid) WHERE is_paid = false;
CREATE INDEX idx_transaction_tags        ON transaction_tags(tag_id);
CREATE INDEX idx_investment_yields_date  ON investment_yields(investment_id, yield_date DESC);
