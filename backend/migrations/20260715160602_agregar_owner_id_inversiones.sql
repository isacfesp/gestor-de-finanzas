-- Las inversiones pasan a ser personales, igual que las cuentas
-- (ver 20260715143043_agregar_owner_id_cuentas.sql): no cuelgan de
-- `accounts`, así que necesitan su propio dueño.
ALTER TABLE investments ADD COLUMN owner_id UUID REFERENCES users(id);

UPDATE investments i SET owner_id = (
    SELECT w.owner_id FROM workspaces w WHERE w.id = i.workspace_id
) WHERE owner_id IS NULL;

ALTER TABLE investments ALTER COLUMN owner_id SET NOT NULL;

CREATE INDEX idx_investments_owner ON investments(workspace_id, owner_id);
