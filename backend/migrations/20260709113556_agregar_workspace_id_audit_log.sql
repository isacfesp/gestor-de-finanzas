-- workspace_id es NULLABLE: eventos globales (login, logout, alta de
-- usuario, bootstrap) no pertenecen a ningún workspace.
ALTER TABLE audit_log ADD COLUMN workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE;
