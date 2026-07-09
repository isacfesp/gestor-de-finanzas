-- Límite de crédito para cuentas tipo 'credit'. Nullable porque no
-- aplica a los demás tipos (cash/debit/savings/investment) — la app
-- exige el valor cuando el tipo es 'credit', no la base.
ALTER TABLE accounts ADD COLUMN credit_limit NUMERIC(15,2);
