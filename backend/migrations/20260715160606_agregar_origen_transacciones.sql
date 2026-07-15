-- Sin esto, la transacción real que genera marcar_cobrada/marcar_pagado
-- queda completamente desligada de la suscripción/previsto que la
-- originó: editar la categoría después (ej. corregir un olvido) no
-- tiene forma de propagarse, porque no hay ningún dato que relacione
-- las filas. Ambas columnas son nullable y mutuamente excluyentes con
-- una transacción manual (que no tiene origen).
ALTER TABLE transactions ADD COLUMN subscription_id UUID REFERENCES subscriptions(id);
ALTER TABLE transactions ADD COLUMN planned_transaction_id UUID REFERENCES planned_transactions(id);

CREATE INDEX idx_transactions_subscription ON transactions(subscription_id) WHERE subscription_id IS NOT NULL;
CREATE INDEX idx_transactions_planned ON transactions(planned_transaction_id) WHERE planned_transaction_id IS NOT NULL;
