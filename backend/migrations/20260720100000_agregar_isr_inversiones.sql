-- Tasa de ISR propia por inversión: hasta ahora era una constante fija
-- en código (0.50 % anual) igual para todas. El default preserva el
-- comportamiento de las inversiones ya existentes; las nuevas deben
-- mandar el valor explícito desde el formulario de alta.
ALTER TABLE investments
    ADD COLUMN isr_annual_rate NUMERIC(7,4) NOT NULL DEFAULT 0.50
    CHECK (isr_annual_rate >= 0);
