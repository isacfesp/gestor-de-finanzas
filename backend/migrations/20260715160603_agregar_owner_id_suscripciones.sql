-- Las suscripciones pasan a ser personales, igual que las cuentas
-- (ver 20260715143043_agregar_owner_id_cuentas.sql).
ALTER TABLE subscriptions ADD COLUMN owner_id UUID REFERENCES users(id);

UPDATE subscriptions s SET owner_id = (
    SELECT w.owner_id FROM workspaces w WHERE w.id = s.workspace_id
) WHERE owner_id IS NULL;

ALTER TABLE subscriptions ALTER COLUMN owner_id SET NOT NULL;

CREATE INDEX idx_subscriptions_owner ON subscriptions(workspace_id, owner_id);
