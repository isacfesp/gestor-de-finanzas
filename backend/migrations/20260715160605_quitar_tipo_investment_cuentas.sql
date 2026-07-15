-- El tipo 'investment' de accounts era un tipo muerto: el frontend ya
-- lo excluía de las 4 operaciones (Ingreso/Gasto/Ahorro/Transferencia,
-- ver Operacion::tipos_cuenta en transacciones_tab.rs), así que una
-- cuenta con este tipo no podía recibir ni enviar dinero por ningún
-- flujo. Las inversiones formales viven en su propio módulo
-- (`investments`), que no depende de `accounts` en absoluto. No hay
-- filas existentes con este tipo (verificado antes de esta migración).
ALTER TABLE accounts DROP CONSTRAINT accounts_type_check;
ALTER TABLE accounts ADD CONSTRAINT accounts_type_check
    CHECK (type = ANY (ARRAY['cash'::text, 'debit'::text, 'credit'::text, 'savings'::text]));
