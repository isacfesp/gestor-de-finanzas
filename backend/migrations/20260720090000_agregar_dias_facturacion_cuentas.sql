-- Día de corte y día límite de pago de una tarjeta de crédito, como
-- día del mes recurrente (1-31) — se recalcula cada ciclo, no hay que
-- reingresarlo. Nullable igual que credit_limit: la obligatoriedad
-- para type='credit' se exige en la app (accounts::cuentas), no con un
-- CHECK cruzado de columnas.
ALTER TABLE accounts ADD COLUMN cutoff_day      SMALLINT CHECK (cutoff_day BETWEEN 1 AND 31);
ALTER TABLE accounts ADD COLUMN payment_due_day SMALLINT CHECK (payment_due_day BETWEEN 1 AND 31);
