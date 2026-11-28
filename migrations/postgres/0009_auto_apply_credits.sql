CREATE OR REPLACE FUNCTION validate_legacy_credit_allocation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    payment_customer uuid;
    payment_amount bigint;
    payment_type text;
    debit_customer uuid;
    debit_amount bigint;
    debit_type text;
    sale_status text;
    allocated_to_debit numeric;
    allocated_from_payment numeric;
BEGIN
    IF TG_OP='UPDATE' AND OLD.status='active' AND NEW.status='reversed'
       AND (to_jsonb(NEW)-'status'-'reversed_by'-'reversal_reason')=(to_jsonb(OLD)-'status'-'reversed_by'-'reversal_reason')
       AND NEW.reversed_by IS NOT NULL AND NEW.reversal_reason ~ '[^[:space:]]' THEN RETURN NEW; END IF;
    IF TG_OP<>'INSERT' OR NEW.status<>'active' THEN RAISE EXCEPTION 'legacy allocations are append-only' USING ERRCODE='23514'; END IF;
    SELECT customer_id,credit_idr,entry_type INTO payment_customer,payment_amount,payment_type FROM receivable_entries WHERE id=NEW.payment_entry_id FOR UPDATE;
    SELECT e.customer_id,e.debit_idr,e.entry_type,s.status INTO debit_customer,debit_amount,debit_type,sale_status
      FROM receivable_entries e LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
     WHERE e.id=NEW.debit_entry_id FOR UPDATE OF e;
    IF payment_type<>'legacy_payment' OR payment_amount<=0 OR debit_type NOT IN ('legacy_debit','credit_sale')
       OR (debit_type='credit_sale' AND sale_status<>'posted') OR debit_amount<=0 OR payment_customer IS DISTINCT FROM debit_customer THEN
        RAISE EXCEPTION 'legacy payment and debit must belong to same customer and be posted' USING ERRCODE='23514';
    END IF;
    SELECT coalesce(sum(amount_idr),0)+(SELECT coalesce(sum(a.amount_idr),0) FROM payment_allocations a JOIN payments p ON p.id=a.payment_id WHERE a.debit_entry_id=NEW.debit_entry_id AND a.status='active' AND p.status='posted') INTO allocated_to_debit FROM legacy_credit_allocations WHERE debit_entry_id=NEW.debit_entry_id AND status='active';
    SELECT coalesce(sum(amount_idr),0) INTO allocated_from_payment FROM legacy_credit_allocations WHERE payment_entry_id=NEW.payment_entry_id AND status='active';
    IF allocated_to_debit+NEW.amount_idr>debit_amount OR allocated_from_payment+NEW.amount_idr>payment_amount THEN RAISE EXCEPTION 'legacy allocation exceeds source' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END;
$$;

-- Rekonsiliasi otomatis seluruh deposit (kredit) terbuka terhadap debit terbuka secara FIFO
DO $$
DECLARE
    c_id uuid;
    d_id uuid;
    d_rem bigint;
    p_type text;
    p_id uuid;
    p_rem bigint;
    alloc_amt bigint;
    system_user uuid;
BEGIN
    SELECT id INTO system_user FROM users WHERE role='owner' ORDER BY created_at LIMIT 1;
    IF system_user IS NULL THEN
        SELECT id INTO system_user FROM users LIMIT 1;
    END IF;
    IF system_user IS NULL THEN
        RETURN;
    END IF;

    FOR c_id IN
        SELECT DISTINCT e.customer_id FROM receivable_entries e
    LOOP
        LOOP
            -- Cari debit terbuka tertua
            SELECT e.id, (e.debit_idr - (
                SELECT coalesce(sum(pa.amount_idr),0) FROM payment_allocations pa JOIN payments p ON p.id=pa.payment_id WHERE pa.debit_entry_id=e.id AND pa.status='active' AND p.status='posted'
            ) - (
                SELECT coalesce(sum(la.amount_idr),0) FROM legacy_credit_allocations la WHERE la.debit_entry_id=e.id AND la.status='active'
            ))::bigint INTO d_id, d_rem
            FROM receivable_entries e
            LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
            WHERE e.customer_id=c_id AND e.debit_idr>0
              AND e.entry_type IN ('legacy_debit','credit_sale')
              AND (e.entry_type<>'legacy_debit' OR NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id))
              AND (e.entry_type='legacy_debit' OR s.status='posted')
              AND (e.debit_idr - (
                SELECT coalesce(sum(pa.amount_idr),0) FROM payment_allocations pa JOIN payments p ON p.id=pa.payment_id WHERE pa.debit_entry_id=e.id AND pa.status='active' AND p.status='posted'
              ) - (
                SELECT coalesce(sum(la.amount_idr),0) FROM legacy_credit_allocations la WHERE la.debit_entry_id=e.id AND la.status='active'
              )) > 0
            ORDER BY e.effective_date NULLS LAST, e.source_order NULLS LAST, coalesce(s.posted_at,e.created_at), e.id
            LIMIT 1;

            EXIT WHEN d_id IS NULL;

            -- Cari kredit terbuka tertua
            SELECT source_type, src_id, rem INTO p_type, p_id, p_rem
            FROM (
                SELECT 'payment' AS source_type, p.id AS src_id,
                       p.received_date AS eff_date, NULL::bigint AS src_ord, p.posted_at AS post_at,
                       (p.amount_idr - coalesce((SELECT sum(pa.amount_idr) FROM payment_allocations pa WHERE pa.payment_id=p.id AND pa.status='active'),0))::bigint AS rem
                FROM payments p
                WHERE p.customer_id=c_id AND p.status='posted'
                  AND p.amount_idr > coalesce((SELECT sum(pa.amount_idr) FROM payment_allocations pa WHERE pa.payment_id=p.id AND pa.status='active'),0)
                UNION ALL
                SELECT 'legacy_payment' AS source_type, e.id AS src_id,
                       e.effective_date AS eff_date, e.source_order AS src_ord, e.created_at AS post_at,
                       (e.credit_idr - coalesce((SELECT sum(la.amount_idr) FROM legacy_credit_allocations la WHERE la.payment_entry_id=e.id AND la.status='active'),0))::bigint AS rem
                FROM receivable_entries e
                WHERE e.customer_id=c_id AND e.entry_type='legacy_payment' AND e.credit_idr>0
                  AND NOT EXISTS(SELECT 1 FROM payments p WHERE p.id=e.source_id)
                  AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id)
                  AND e.credit_idr > coalesce((SELECT sum(la.amount_idr) FROM legacy_credit_allocations la WHERE la.payment_entry_id=e.id AND la.status='active'),0)
            ) creds
            ORDER BY eff_date NULLS LAST, src_ord NULLS LAST, post_at, src_id
            LIMIT 1;

            EXIT WHEN p_id IS NULL;

            alloc_amt := LEAST(d_rem, p_rem);
            EXIT WHEN alloc_amt <= 0;

            IF p_type = 'payment' THEN
                INSERT INTO payment_allocations (id, payment_id, debit_entry_id, amount_idr, status, created_by, reason)
                VALUES (gen_random_uuid(), p_id, d_id, alloc_amt, 'active', system_user, 'Auto-apply dari deposit pelanggan');
            ELSE
                INSERT INTO legacy_credit_allocations (id, payment_entry_id, debit_entry_id, amount_idr, status, created_by)
                VALUES (gen_random_uuid(), p_id, d_id, alloc_amt, 'active', system_user);
            END IF;
        END LOOP;
    END LOOP;
END;
$$;
