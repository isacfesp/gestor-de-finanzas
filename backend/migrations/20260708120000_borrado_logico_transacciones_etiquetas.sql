-- Borrado lógico: "borrar" una transacción o una etiqueta ya no elimina
-- la fila físicamente, la marca inactiva. El saldo de la cuenta se
-- sigue ajustando igual que hoy al borrar una transacción; el listado
-- normal oculta las filas inactivas.
ALTER TABLE transactions ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE tags ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT true;
