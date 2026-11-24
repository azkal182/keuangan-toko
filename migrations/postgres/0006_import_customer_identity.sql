ALTER TABLE import_rows
    ADD COLUMN source_customer jsonb NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE import_rows
    ADD CONSTRAINT import_rows_source_customer_object
    CHECK (jsonb_typeof(source_customer) = 'object');
