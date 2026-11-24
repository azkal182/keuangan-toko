CREATE TABLE import_batches (
    id uuid PRIMARY KEY,
    file_sha256 char(64) NOT NULL UNIQUE,
    original_filename text NOT NULL,
    raw_workbook bytea NOT NULL,
    status text NOT NULL CHECK (status IN ('uploaded','reviewing','approved','committed','closed_with_exceptions','failed')),
    uploaded_by uuid NOT NULL REFERENCES users(id),
    approved_by uuid REFERENCES users(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    approved_at timestamptz,
    committed_at timestamptz,
    closed_at timestamptz,
    variance_idr bigint,
    variance_reason text,
    version integer NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE TABLE import_rows (
    id uuid PRIMARY KEY,
    batch_id uuid NOT NULL REFERENCES import_batches(id),
    sheet_name text NOT NULL,
    row_number integer NOT NULL CHECK (row_number > 0),
    raw_values jsonb NOT NULL,
    parsed_values jsonb,
    issues jsonb NOT NULL DEFAULT '[]',
    review_status text NOT NULL CHECK (review_status IN ('candidate','accepted','quarantined','excluded','committed')),
    mapped_customer_id uuid REFERENCES customers(id),
    reviewed_by uuid REFERENCES users(id),
    review_reason text,
    review_note text,
    source_fingerprint char(64) NOT NULL,
    source_order bigint NOT NULL,
    committed_entry_id uuid REFERENCES receivable_entries(id),
    version integer NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (batch_id, sheet_name, row_number)
);
CREATE INDEX import_rows_batch_review ON import_rows(batch_id, review_status, source_order);
CREATE INDEX import_rows_fingerprint ON import_rows(source_fingerprint);

CREATE TABLE import_row_commits (
    import_row_id uuid PRIMARY KEY REFERENCES import_rows(id),
    ledger_entry_id uuid NOT NULL UNIQUE REFERENCES receivable_entries(id)
);

CREATE TABLE legacy_credit_allocations (
    id uuid PRIMARY KEY,
    payment_entry_id uuid NOT NULL REFERENCES receivable_entries(id),
    debit_entry_id uuid NOT NULL REFERENCES receivable_entries(id),
    amount_idr bigint NOT NULL CHECK (amount_idr > 0),
    status text NOT NULL CHECK (status IN ('active','reversed')),
    created_by uuid NOT NULL REFERENCES users(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    reversed_by uuid REFERENCES users(id),
    reversal_reason text,
    CHECK ((status='active' AND reversed_by IS NULL AND reversal_reason IS NULL)
        OR (status='reversed' AND reversed_by IS NOT NULL AND reversal_reason ~ '[^[:space:]]'))
);
CREATE INDEX legacy_credit_allocations_debit ON legacy_credit_allocations(debit_entry_id,status);
CREATE INDEX legacy_credit_allocations_payment ON legacy_credit_allocations(payment_entry_id,status);

CREATE FUNCTION validate_legacy_credit_allocation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    payment_customer uuid;
    payment_amount bigint;
    payment_type text;
    debit_customer uuid;
    debit_amount bigint;
    debit_type text;
    allocated_to_debit numeric;
    allocated_from_payment numeric;
BEGIN
    IF TG_OP='UPDATE' AND OLD.status='active' AND NEW.status='reversed'
       AND (to_jsonb(NEW)-'status'-'reversed_by'-'reversal_reason')=(to_jsonb(OLD)-'status'-'reversed_by'-'reversal_reason')
       AND NEW.reversed_by IS NOT NULL AND NEW.reversal_reason ~ '[^[:space:]]' THEN RETURN NEW; END IF;
    IF TG_OP<>'INSERT' OR NEW.status<>'active' THEN RAISE EXCEPTION 'legacy allocations are append-only' USING ERRCODE='23514'; END IF;
    SELECT customer_id,credit_idr,entry_type INTO payment_customer,payment_amount,payment_type FROM receivable_entries WHERE id=NEW.payment_entry_id FOR UPDATE;
    SELECT customer_id,debit_idr,entry_type INTO debit_customer,debit_amount,debit_type FROM receivable_entries WHERE id=NEW.debit_entry_id FOR UPDATE;
    IF payment_type<>'legacy_payment' OR payment_amount<=0 OR debit_type<>'legacy_debit' OR debit_amount<=0 OR payment_customer IS DISTINCT FROM debit_customer THEN
        RAISE EXCEPTION 'legacy payment and debit must belong to same customer' USING ERRCODE='23514';
    END IF;
    SELECT coalesce(sum(amount_idr),0)+(SELECT coalesce(sum(a.amount_idr),0) FROM payment_allocations a JOIN payments p ON p.id=a.payment_id WHERE a.debit_entry_id=NEW.debit_entry_id AND a.status='active' AND p.status='posted') INTO allocated_to_debit FROM legacy_credit_allocations WHERE debit_entry_id=NEW.debit_entry_id AND status='active';
    SELECT coalesce(sum(amount_idr),0) INTO allocated_from_payment FROM legacy_credit_allocations WHERE payment_entry_id=NEW.payment_entry_id AND status='active';
    IF allocated_to_debit+NEW.amount_idr>debit_amount OR allocated_from_payment+NEW.amount_idr>payment_amount THEN RAISE EXCEPTION 'legacy allocation exceeds source' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER legacy_credit_allocations_validate BEFORE INSERT OR UPDATE OR DELETE ON legacy_credit_allocations
    FOR EACH ROW EXECUTE FUNCTION validate_legacy_credit_allocation();

ALTER TABLE credit_sales ADD COLUMN reversed_at timestamptz;
ALTER TABLE credit_sales ADD COLUMN reversed_by uuid REFERENCES users(id);
ALTER TABLE credit_sales ADD COLUMN reversal_reason text;
ALTER TABLE credit_sales ADD CONSTRAINT credit_sale_reversal_fields CHECK (
    (status = 'reversed' AND reversed_at IS NOT NULL AND reversed_by IS NOT NULL AND reversal_reason IS NOT NULL)
    OR (status <> 'reversed' AND reversed_at IS NULL AND reversed_by IS NULL AND reversal_reason IS NULL)
);
ALTER TABLE payments ADD COLUMN reversed_by uuid REFERENCES users(id);
ALTER TABLE payments ADD COLUMN reversal_reason text;
ALTER TABLE payments ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE payments ADD CONSTRAINT payment_reversal_fields CHECK (
    (status = 'posted' AND reversed_at IS NULL AND reversed_by IS NULL AND reversal_reason IS NULL)
    OR (status = 'reversed' AND reversed_at IS NOT NULL AND reversed_by IS NOT NULL AND reversal_reason ~ '[^[:space:]]')
);

CREATE FUNCTION protect_posted_payment() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.status = 'posted' AND NEW.status = 'reversed'
       AND NEW.reversed_at IS NOT NULL AND NEW.reversed_by IS NOT NULL
       AND NEW.reversal_reason ~ '[^[:space:]]'
       AND (to_jsonb(NEW) - 'status' - 'reversed_at' - 'reversed_by' - 'reversal_reason')
           = (to_jsonb(OLD) - 'status' - 'reversed_at' - 'reversed_by' - 'reversal_reason') THEN RETURN NEW; END IF;
    IF TG_OP = 'UPDATE' AND OLD.status = 'posted' AND NEW.status = 'posted'
       AND NEW.version = OLD.version + 1
       AND (to_jsonb(NEW) - 'version') = (to_jsonb(OLD) - 'version') THEN RETURN NEW; END IF;
    IF TG_OP = 'UPDATE' AND OLD.status = 'posted' AND NEW.status = 'posted'
       AND (to_jsonb(NEW) - 'customer_id') = (to_jsonb(OLD) - 'customer_id') THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'posted payment cannot be removed or mutated' USING ERRCODE='23514';
END;
$$;
CREATE TRIGGER payments_immutable BEFORE UPDATE OR DELETE ON payments
    FOR EACH ROW EXECUTE FUNCTION protect_posted_payment();

CREATE OR REPLACE FUNCTION protect_credit_sale() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.status = 'draft' THEN RETURN NEW; END IF;
    IF TG_OP = 'UPDATE' AND OLD.status = 'posted' AND NEW.status = 'reversed'
       AND NEW.reversed_at IS NOT NULL AND NEW.reversed_by IS NOT NULL
       AND NEW.reversal_reason ~ '[^[:space:]]'
       AND (to_jsonb(NEW) - 'status' - 'version' - 'reversed_at' - 'reversed_by' - 'reversal_reason')
           = (to_jsonb(OLD) - 'status' - 'version' - 'reversed_at' - 'reversed_by' - 'reversal_reason')
       AND NEW.version = OLD.version + 1 THEN RETURN NEW; END IF;
    IF TG_OP = 'UPDATE' AND OLD.status IN ('posted','reversed') AND
       (to_jsonb(NEW) - 'customer_id') = (to_jsonb(OLD) - 'customer_id') THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'credit sale cannot be removed or mutated after posting' USING ERRCODE = '23514';
END;
$$;

ALTER TABLE payment_allocations ADD COLUMN reversal_reason text;
ALTER TABLE payment_allocations ADD CONSTRAINT payment_allocation_reversal_fields CHECK (
    (status = 'active' AND reversed_by IS NULL AND reversal_reason IS NULL)
    OR (status = 'reversed' AND reversed_by IS NOT NULL AND reversal_reason ~ '[^[:space:]]')
);
CREATE OR REPLACE FUNCTION validate_payment_allocation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    payment_customer uuid;
    payment_status text;
    payment_amount bigint;
    debit_customer uuid;
    debit_amount bigint;
    debit_type text;
    sale_status text;
    allocated_to_debit numeric;
    allocated_from_payment numeric;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF OLD.status = 'active' AND NEW.status = 'reversed'
           AND (to_jsonb(NEW) - 'status' - 'reversed_by' - 'reversal_reason') = (to_jsonb(OLD) - 'status' - 'reversed_by' - 'reversal_reason')
           AND NEW.reversed_by IS NOT NULL AND NEW.reversal_reason ~ '[^[:space:]]' THEN RETURN NEW; END IF;
        RAISE EXCEPTION 'payment allocations are immutable; reverse and replace' USING ERRCODE = '23514';
    END IF;
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'payment allocations are immutable; reverse and replace' USING ERRCODE = '23514';
    END IF;
    IF NEW.status <> 'active' THEN
        RAISE EXCEPTION 'new payment allocation must be active' USING ERRCODE = '23514';
    END IF;
    SELECT p.customer_id, p.status, p.amount_idr INTO payment_customer, payment_status, payment_amount FROM payments p WHERE p.id = NEW.payment_id FOR UPDATE;
    SELECT e.customer_id, e.debit_idr, e.entry_type, s.status INTO debit_customer, debit_amount, debit_type, sale_status
      FROM receivable_entries e LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
     WHERE e.id=NEW.debit_entry_id FOR UPDATE OF e;
    IF payment_status <> 'posted' OR debit_amount IS NULL OR debit_amount <= 0 OR debit_type NOT IN ('legacy_debit','credit_sale')
       OR (debit_type='credit_sale' AND sale_status <> 'posted') OR payment_customer IS DISTINCT FROM debit_customer THEN
        RAISE EXCEPTION 'payment and debit must be posted and belong to the same customer' USING ERRCODE='23514';
    END IF;
    SELECT coalesce(sum(a.amount_idr),0)+(SELECT coalesce(sum(x.amount_idr),0) FROM legacy_credit_allocations x WHERE x.debit_entry_id=NEW.debit_entry_id AND x.status='active') INTO allocated_to_debit FROM payment_allocations a JOIN payments p ON p.id=a.payment_id
      WHERE a.debit_entry_id=NEW.debit_entry_id AND a.status='active' AND p.status='posted';
    SELECT coalesce(sum(a.amount_idr),0) INTO allocated_from_payment FROM payment_allocations a JOIN payments p ON p.id=a.payment_id
      WHERE a.payment_id=NEW.payment_id AND a.status='active' AND p.status='posted';
    IF allocated_to_debit + NEW.amount_idr > debit_amount OR allocated_from_payment + NEW.amount_idr > payment_amount THEN
        RAISE EXCEPTION 'allocation exceeds payment or outstanding debit' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TABLE transaction_reversals (
    id uuid PRIMARY KEY,
    original_entry_id uuid NOT NULL UNIQUE REFERENCES receivable_entries(id),
    reversal_entry_id uuid NOT NULL UNIQUE REFERENCES receivable_entries(id),
    replacement_entry_id uuid UNIQUE REFERENCES receivable_entries(id),
    source_type text NOT NULL CHECK (source_type IN ('credit_sale','payment','legacy_debit','legacy_payment')),
    source_id uuid NOT NULL,
    reversed_by uuid NOT NULL REFERENCES users(id),
    reason text NOT NULL CHECK (reason ~ '[^[:space:]]'),
    request_id uuid NOT NULL,
    reversed_at timestamptz NOT NULL DEFAULT now()
);
