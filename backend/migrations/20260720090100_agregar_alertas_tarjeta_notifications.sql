-- Nuevos tipos de notificación para avisar de la fecha de corte y la
-- fecha límite de pago de una tarjeta de crédito.
ALTER TABLE notifications DROP CONSTRAINT notifications_type_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_type_check
    CHECK (type = ANY (ARRAY['subscription_due', 'budget_80', 'budget_100', 'goal_deadline',
                              'credit_card_cutoff', 'credit_card_due']::text[]));
