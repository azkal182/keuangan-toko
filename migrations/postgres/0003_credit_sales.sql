CREATE TABLE credit_sales (
    id uuid PRIMARY KEY,
    customer_id uuid NOT NULL REFERENCES customers (id),
    effective_date date NOT NULL,
    reference_no text,
    backdate_reason text,
    status text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'posted', 'reversed')),
    total_idr bigint NOT NULL CHECK (total_idr > 0),
    version integer NOT NULL DEFAULT 1 CHECK (version > 0),
    created_by uuid NOT NULL REFERENCES users (id),
    posted_by uuid REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now(),
    posted_at timestamptz,
    CHECK ((status = 'draft' AND posted_by IS NULL AND posted_at IS NULL)
        OR (status IN ('posted', 'reversed') AND posted_by IS NOT NULL AND posted_at IS NOT NULL))
);

CREATE INDEX credit_sales_customer_id ON credit_sales (customer_id);

CREATE TABLE credit_sale_items (
    id uuid PRIMARY KEY,
    sale_id uuid NOT NULL REFERENCES credit_sales (id),
    ordinal integer NOT NULL CHECK (ordinal > 0),
    description text NOT NULL CHECK (description ~ '[^[:space:]]'),
    quantity numeric(12,3) NOT NULL CHECK (quantity > 0),
    unit_price_idr bigint NOT NULL CHECK (unit_price_idr >= 0),
    line_total_idr bigint NOT NULL CHECK (
        line_total_idr > 0 AND line_total_idr = round(quantity * unit_price_idr)
    ),
    UNIQUE (sale_id, ordinal)
);

CREATE TABLE idempotency_records (
    id uuid PRIMARY KEY,
    actor_id uuid NOT NULL REFERENCES users (id),
    route_key text NOT NULL,
    idempotency_key text NOT NULL,
    request_hash char(64) NOT NULL,
    response_status integer NOT NULL,
    response_body jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (actor_id, route_key, idempotency_key)
);

CREATE FUNCTION protect_credit_sale() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND (OLD.status = 'draft'
        OR (to_jsonb(NEW) - 'customer_id') = (to_jsonb(OLD) - 'customer_id')) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'credit sale cannot be removed or mutated after posting' USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER credit_sales_immutable
    BEFORE UPDATE OR DELETE ON credit_sales
    FOR EACH ROW EXECUTE FUNCTION protect_credit_sale();

CREATE FUNCTION protect_credit_sale_item() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    parent_id uuid;
    parent_status text;
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.sale_id <> OLD.sale_id THEN
        RAISE EXCEPTION 'credit sale item cannot be moved' USING ERRCODE = '23514';
    END IF;
    parent_id := CASE WHEN TG_OP = 'DELETE' THEN OLD.sale_id ELSE NEW.sale_id END;
    SELECT status INTO parent_status FROM credit_sales WHERE id = parent_id FOR UPDATE;
    IF parent_status <> 'draft' THEN
        RAISE EXCEPTION 'posted credit sale item is immutable' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER credit_sale_items_draft_only
    BEFORE INSERT OR UPDATE OR DELETE ON credit_sale_items
    FOR EACH ROW EXECUTE FUNCTION protect_credit_sale_item();
