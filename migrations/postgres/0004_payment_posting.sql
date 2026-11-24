CREATE INDEX payment_fifo_order
    ON receivable_entries (customer_id, effective_date, created_at, id)
    WHERE debit_idr > 0;

CREATE FUNCTION validate_payment_allocation() RETURNS trigger LANGUAGE plpgsql AS $$
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
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'payment allocations are immutable; reverse and replace' USING ERRCODE = '23514';
    END IF;
    IF NEW.status <> 'active' THEN
        RAISE EXCEPTION 'new payment allocation must be active' USING ERRCODE = '23514';
    END IF;

    SELECT p.customer_id, p.status, p.amount_idr
      INTO payment_customer, payment_status, payment_amount
      FROM payments p WHERE p.id = NEW.payment_id FOR UPDATE;
    SELECT e.customer_id, e.debit_idr, e.entry_type, s.status
      INTO debit_customer, debit_amount, debit_type, sale_status
      FROM receivable_entries e
      LEFT JOIN credit_sales s ON s.id = e.source_id AND e.entry_type = 'credit_sale'
     WHERE e.id = NEW.debit_entry_id
     FOR UPDATE OF e;

    IF payment_status <> 'posted' OR debit_amount IS NULL OR debit_amount <= 0
       OR debit_type NOT IN ('legacy_debit', 'credit_sale')
       OR (debit_type = 'credit_sale' AND sale_status <> 'posted')
       OR payment_customer IS DISTINCT FROM debit_customer THEN
        RAISE EXCEPTION 'payment and debit must be posted and belong to the same customer' USING ERRCODE = '23514';
    END IF;

    SELECT coalesce(sum(a.amount_idr), 0) INTO allocated_to_debit
      FROM payment_allocations a JOIN payments p ON p.id = a.payment_id
     WHERE a.debit_entry_id = NEW.debit_entry_id AND a.status = 'active' AND p.status = 'posted';
    SELECT coalesce(sum(a.amount_idr), 0) INTO allocated_from_payment
      FROM payment_allocations a JOIN payments p ON p.id = a.payment_id
     WHERE a.payment_id = NEW.payment_id AND a.status = 'active' AND p.status = 'posted';

    IF allocated_to_debit + NEW.amount_idr > debit_amount
       OR allocated_from_payment + NEW.amount_idr > payment_amount THEN
        RAISE EXCEPTION 'allocation exceeds payment or outstanding debit' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER payment_allocations_validate
    BEFORE INSERT OR UPDATE OR DELETE ON payment_allocations
    FOR EACH ROW EXECUTE FUNCTION validate_payment_allocation();
