-- Une las transacciones (ingresos/gastos) con la cuenta de la que salen o
-- a la que entran, para que el saldo de cada cuenta refleje sus
-- movimientos reales (antes solo cambiaba por transferencias).
--
-- Se agrega nullable, se rellenan las filas existentes con la primera
-- cuenta del mismo workspace (dato de prueba, no hay transacciones reales
-- en producción todavía) y luego se exige NOT NULL.
ALTER TABLE transactions ADD COLUMN account_id UUID REFERENCES accounts(id);

UPDATE transactions t SET account_id = (
    SELECT id FROM accounts
    WHERE workspace_id = t.workspace_id
    ORDER BY created_at
    LIMIT 1
) WHERE account_id IS NULL;

ALTER TABLE transactions ALTER COLUMN account_id SET NOT NULL;
