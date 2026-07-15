-- La FK sin ON DELETE bloqueaba borrar una suscripción/previsto que ya
-- generó transacciones (violación de FK), justo el caso de uso normal
-- de "borrar una suscripción vieja". La transacción es un registro
-- financiero real: al borrar su origen, debe conservarse y solo perder
-- el puntero (no se borra en cascada).
ALTER TABLE transactions DROP CONSTRAINT transactions_subscription_id_fkey;
ALTER TABLE transactions ADD CONSTRAINT transactions_subscription_id_fkey
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE SET NULL;

ALTER TABLE transactions DROP CONSTRAINT transactions_planned_transaction_id_fkey;
ALTER TABLE transactions ADD CONSTRAINT transactions_planned_transaction_id_fkey
    FOREIGN KEY (planned_transaction_id) REFERENCES planned_transactions(id) ON DELETE SET NULL;
