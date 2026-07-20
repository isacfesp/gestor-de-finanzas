-- Cálculo diario automático de rendimiento e ISR de cada inversión
-- activa. Distinta de investment_yields: esa tabla es lo que la
-- SOFIPO realmente pagó (dato manual); esta es una estimación que
-- calcula el sistema día a día, con su propio desglose bruto/ISR/neto.
CREATE TABLE investment_accruals (
    id            UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    investment_id UUID          NOT NULL REFERENCES investments(id) ON DELETE CASCADE,
    accrual_date  DATE          NOT NULL,
    gross_yield   NUMERIC(15,2) NOT NULL,
    isr_amount    NUMERIC(15,2) NOT NULL,
    net_yield     NUMERIC(15,2) NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),

    -- Base de la idempotencia del job diario: un día ya calculado para
    -- una inversión nunca se vuelve a insertar (ON CONFLICT DO NOTHING).
    CONSTRAINT investment_accruals_unique UNIQUE (investment_id, accrual_date)
);

CREATE INDEX idx_investment_accruals_investment ON investment_accruals(investment_id, accrual_date DESC);
