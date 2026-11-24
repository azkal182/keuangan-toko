ALTER TABLE receivable_entries
    ADD COLUMN description text,
    ADD COLUMN quantity text,
    ADD COLUMN unit_price_idr bigint;

ALTER TABLE receivable_entries
    ADD CONSTRAINT receivable_entries_unit_price_nonnegative
    CHECK (unit_price_idr IS NULL OR unit_price_idr >= 0);

-- Restore descriptive cells for imports committed before these columns existed.
DROP TRIGGER receivable_entries_immutable ON receivable_entries;
UPDATE receivable_entries e
SET description = nullif(btrim(r.parsed_values->>'description'), ''),
    quantity = nullif(btrim(r.parsed_values->>'quantity'), ''),
    unit_price_idr = CASE
        WHEN r.parsed_values->>'unit_price_idr' ~ '^[0-9]+$'
         AND (r.parsed_values->>'unit_price_idr')::numeric <= 9223372036854775807
        THEN (r.parsed_values->>'unit_price_idr')::bigint
    END
FROM import_row_commits c
JOIN import_rows r ON r.id = c.import_row_id
WHERE e.id = c.ledger_entry_id
  AND e.entry_type IN ('legacy_debit', 'legacy_payment');
CREATE TRIGGER receivable_entries_immutable
    BEFORE UPDATE OR DELETE ON receivable_entries
    FOR EACH ROW EXECUTE FUNCTION protect_receivable_entry();
