CREATE TABLE payments (
    id uuid PRIMARY KEY,
    customer_id uuid NOT NULL REFERENCES customers (id),
    amount_idr bigint NOT NULL CHECK (amount_idr > 0),
    received_date date NOT NULL,
    method text NOT NULL CHECK (method IN ('cash', 'bank_transfer')),
    reference text,
    note text,
    status text NOT NULL CHECK (status IN ('posted', 'reversed')),
    created_by uuid NOT NULL REFERENCES users (id),
    posted_at timestamptz NOT NULL DEFAULT now(),
    reversed_at timestamptz
);

CREATE INDEX payments_customer_id ON payments (customer_id);

CREATE TABLE payment_allocations (
    id uuid PRIMARY KEY,
    payment_id uuid NOT NULL REFERENCES payments (id),
    debit_entry_id uuid NOT NULL REFERENCES receivable_entries (id),
    amount_idr bigint NOT NULL CHECK (amount_idr > 0),
    status text NOT NULL CHECK (status IN ('active', 'reversed')),
    created_at timestamptz NOT NULL DEFAULT now(),
    created_by uuid NOT NULL REFERENCES users (id),
    reversed_by uuid REFERENCES users (id),
    reason text
);

CREATE INDEX payment_allocations_payment_id ON payment_allocations (payment_id);
CREATE INDEX payment_allocations_debit_entry_id ON payment_allocations (debit_entry_id);

CREATE TABLE audit_events (
    id uuid PRIMARY KEY,
    actor_id uuid NOT NULL REFERENCES users (id),
    action text NOT NULL,
    entity_type text NOT NULL,
    entity_id uuid NOT NULL,
    reason text,
    request_id uuid NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    metadata jsonb NOT NULL DEFAULT '{}'
);

CREATE FUNCTION protect_audit_event() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit event is immutable' USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER audit_events_immutable
    BEFORE UPDATE OR DELETE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION protect_audit_event();
