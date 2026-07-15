-- Cada cuenta pasa a tener un dueño individual: hoy cualquier miembro
-- del workspace puede operar cualquier cuenta, y el negocio quiere que
-- cada cuenta sea personal (el admin del workspace solo supervisa,
-- ver backend/src/accounts/cuentas.rs).
--
-- Se agrega nullable, se rellena con el owner del workspace (mismo
-- criterio que 20260708000001_agregar_account_id_transacciones.sql)
-- y luego se exige NOT NULL.
ALTER TABLE accounts ADD COLUMN owner_id UUID REFERENCES users(id);

UPDATE accounts a SET owner_id = (
    SELECT w.owner_id FROM workspaces w WHERE w.id = a.workspace_id
) WHERE owner_id IS NULL;

ALTER TABLE accounts ALTER COLUMN owner_id SET NOT NULL;

CREATE INDEX idx_accounts_owner ON accounts(workspace_id, owner_id);
