CREATE TABLE users (
    id uuid PRIMARY KEY,
    username text NOT NULL UNIQUE CHECK (btrim(username) <> ''),
    password_hash text NOT NULL CHECK (password_hash LIKE '$argon2id$%'),
    role text NOT NULL CHECK (role IN ('owner', 'operator', 'viewer')),
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id),
    token_hash bytea NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '12 hours'),
    CHECK (last_seen_at >= created_at)
);

CREATE INDEX sessions_user_id ON sessions (user_id);

CREATE TABLE customers (
    id uuid PRIMARY KEY,
    code text,
    normalized_code text,
    name text NOT NULL CHECK (name ~ '[^[:space:]]'),
    address text,
    is_active boolean NOT NULL DEFAULT true,
    version integer NOT NULL DEFAULT 1 CHECK (version > 0),
    created_by uuid NOT NULL REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((code IS NULL AND normalized_code IS NULL)
        OR (code IS NOT NULL AND normalized_code IS NOT NULL
            AND code ~ '[^[:space:]]' AND normalized_code ~ '[^[:space:]]'))
);

CREATE UNIQUE INDEX customers_normalized_code ON customers (normalized_code)
    WHERE normalized_code IS NOT NULL;

CREATE TABLE receivable_entries (
    id uuid PRIMARY KEY,
    customer_id uuid NOT NULL REFERENCES customers (id),
    entry_type text NOT NULL,
    source_id uuid NOT NULL,
    effective_date date,
    debit_idr bigint NOT NULL DEFAULT 0,
    credit_idr bigint NOT NULL DEFAULT 0,
    source_order bigint,
    created_at timestamptz NOT NULL DEFAULT now(),
    created_by uuid NOT NULL REFERENCES users (id),
    CHECK ((debit_idr > 0 AND credit_idr = 0) OR (credit_idr > 0 AND debit_idr = 0)),
    UNIQUE (entry_type, source_id)
);

CREATE INDEX receivable_entries_customer_history
    ON receivable_entries (customer_id, effective_date, created_at, id);

CREATE FUNCTION protect_receivable_entry() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- An audited customer merge can relink the customer while preserving the event.
    IF TG_OP = 'UPDATE'
       AND (to_jsonb(NEW) - 'customer_id') = (to_jsonb(OLD) - 'customer_id') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'posted ledger entry is immutable' USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER receivable_entries_immutable
    BEFORE UPDATE OR DELETE ON receivable_entries
    FOR EACH ROW EXECUTE FUNCTION protect_receivable_entry();

CREATE FUNCTION prevent_customer_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'customers must be deactivated' USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER customers_no_delete
    BEFORE DELETE ON customers
    FOR EACH ROW EXECUTE FUNCTION prevent_customer_delete();
