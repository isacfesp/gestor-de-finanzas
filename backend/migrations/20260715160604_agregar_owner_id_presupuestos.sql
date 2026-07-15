-- Los presupuestos pasan a ser personales: cada usuario tendrá su
-- propio límite por categoría/mes (la categoría en sí sigue
-- compartida). La restricción única deja de ser (workspace, categoría,
-- mes) — un solo límite por workspace — y pasa a incluir el dueño.
ALTER TABLE budgets ADD COLUMN owner_id UUID REFERENCES users(id);

UPDATE budgets b SET owner_id = (
    SELECT w.owner_id FROM workspaces w WHERE w.id = b.workspace_id
) WHERE owner_id IS NULL;

ALTER TABLE budgets ALTER COLUMN owner_id SET NOT NULL;

ALTER TABLE budgets DROP CONSTRAINT budgets_workspace_category_month_unique;
ALTER TABLE budgets ADD CONSTRAINT budgets_workspace_owner_category_month_unique
    UNIQUE (workspace_id, owner_id, category_id, month);
