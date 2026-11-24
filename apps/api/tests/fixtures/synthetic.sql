-- Synthetic test data. Password digests are random and have no known password.
INSERT INTO users (id, username, password_hash, role, is_active) VALUES
    ('00000000-0000-4000-8000-000000000001', 'fixture-owner', '$argon2id$v=19$m=19456,t=2,p=1$OCoFk4zfLF02jePmcVzrkg$v0Df76g9BUdLH61LoI++Wfq46d+F/6oOOlsXJA+ZQNc', 'owner', true),
    ('00000000-0000-4000-8000-000000000002', 'fixture-operator', '$argon2id$v=19$m=19456,t=2,p=1$vIC2hC3lyIXW3w9P+XvpdA$9QzvJjtes4f4bG/usRlg0Z8TFFDfyuL1V3E3VBRQUaM', 'operator', true),
    ('00000000-0000-4000-8000-000000000003', 'fixture-viewer', '$argon2id$v=19$m=19456,t=2,p=1$G9hZLAW+mstqYlYsviVjxg$OuOFMThHAsMSV39bu055aaayHLet3MsMSbF2FzKTdjM', 'viewer', true),
    ('00000000-0000-4000-8000-000000000004', 'fixture-inactive', '$argon2id$v=19$m=19456,t=2,p=1$WaswFh34z4iml6NZbbqXww$17iFEANIMXFydbc9H4sAD6osC/GgZEJyOmItO6T5Rio', 'operator', false);

WITH fixture_tokens (suffix, user_id, token, age, expired, revoked) AS (
    VALUES
        ('001', '001', 'a', interval '5 minutes', false, false),
        ('002', '002', 'b', interval '5 minutes', false, false),
        ('003', '003', 'c', interval '5 minutes', false, false),
        ('004', '004', 'd', interval '5 minutes', false, false),
        ('005', '002', 'e', interval '5 minutes', true, false),
        ('006', '002', 'f', interval '31 minutes', false, false),
        ('007', '002', '0', interval '5 minutes', false, true)
)
INSERT INTO sessions (id, user_id, token_hash, created_at, last_seen_at, expires_at, revoked_at)
SELECT ('00000000-0000-4000-8000-000000001' || suffix)::uuid,
       ('00000000-0000-4000-8000-000000000' || user_id)::uuid,
       sha256(convert_to(repeat(token, 64), 'UTF8')),
       now() - interval '1 hour', now() - age,
       CASE WHEN expired THEN now() - interval '1 minute' ELSE now() + interval '1 hour' END,
       CASE WHEN revoked THEN now() END
FROM fixture_tokens;

INSERT INTO customers (id, code, normalized_code, name, created_by) VALUES
    ('00000000-0000-4000-8000-000000002001', 'UJI-A', 'uji-a', 'Contoh Pelanggan A', '00000000-0000-4000-8000-000000000001'),
    ('00000000-0000-4000-8000-000000002002', NULL, NULL, 'Contoh Pelanggan B', '00000000-0000-4000-8000-000000000001');

INSERT INTO receivable_entries (id, customer_id, entry_type, source_id, effective_date, debit_idr, credit_idr, created_by) VALUES
    ('00000000-0000-4000-8000-000000003001', '00000000-0000-4000-8000-000000002001', 'legacy_debit',
     '00000000-0000-4000-8000-000000004001', '2026-01-01', 100000, 0, '00000000-0000-4000-8000-000000000001'),
    ('00000000-0000-4000-8000-000000003002', '00000000-0000-4000-8000-000000002001', 'legacy_payment',
     '00000000-0000-4000-8000-000000004002', '2026-01-02', 0, 40000, '00000000-0000-4000-8000-000000000001'),
    ('00000000-0000-4000-8000-000000003003', '00000000-0000-4000-8000-000000002002', 'legacy_debit',
     '00000000-0000-4000-8000-000000004003', NULL, 17000, 0, '00000000-0000-4000-8000-000000000001');

INSERT INTO payments (id, customer_id, amount_idr, received_date, method, reference, note, status, created_by) VALUES
    ('00000000-0000-4000-8000-000000004002', '00000000-0000-4000-8000-000000002001', 40000,
     '2026-01-02', 'cash', 'UJI-BAYAR-A', 'Pembayaran fixture', 'posted', '00000000-0000-4000-8000-000000000001'),
    ('00000000-0000-4000-8000-000000004004', '00000000-0000-4000-8000-000000002002', 20000,
     '2026-01-03', 'bank_transfer', 'UJI-KREDIT-B', NULL, 'posted', '00000000-0000-4000-8000-000000000001');

INSERT INTO payment_allocations (id, payment_id, debit_entry_id, amount_idr, status, created_by) VALUES
    ('00000000-0000-4000-8000-000000005001', '00000000-0000-4000-8000-000000004002',
     '00000000-0000-4000-8000-000000003001', 40000, 'active', '00000000-0000-4000-8000-000000000001');

INSERT INTO receivable_entries (id, customer_id, entry_type, source_id, effective_date, debit_idr, credit_idr, created_by) VALUES
    ('00000000-0000-4000-8000-000000003004', '00000000-0000-4000-8000-000000002002', 'payment',
     '00000000-0000-4000-8000-000000004004', '2026-01-03', 0, 20000, '00000000-0000-4000-8000-000000000001');
