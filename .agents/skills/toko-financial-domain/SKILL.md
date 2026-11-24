---
name: toko-financial-domain
description: Design or review Keuangan Toko ledger, money, credit sales, payment allocation, receivable balances, aging, corrections, and financial invariants without silently deciding open business rules.
---

# Keuangan Toko Financial Domain

Use when a task changes or evaluates transaction meaning, saldo piutang, payment allocation, aging, correction, or financial data representation. Read the product and feature specs plus all impacted OQ entries before proposing a rule.

## Invariants and decision discipline

- Debit adds receivable and credit reduces it; derive ledger balance from accepted debit/credit events, not copied Excel saldo/formula columns.
- Apply the final money representation, quantity scale, and rounding in OQ-05 / ADR-007. Do not use binary floating point for money.
- Apply the deterministic FIFO ordering, backdated transaction, and reallocation policy in OQ-03 / ADR-007.
- Keep overpayment as unallocated customer credit; future use is explicit and audited under OQ-04 / ADR-007.
- Use the final aging buckets and date exceptions from OQ-06/OQ-08/OQ-12 / ADR-007.
- Follow the final posting, reversal, role, retention, and audit policies in OQ-09/OQ-11/OQ-18 / ADR-007. Never erase posted financial history.

## Review method

For a change, identify affected BR/FR/AC/OQ; state examples with exact assumptions; inspect transaction atomicity, concurrency, idempotency, audit lineage, and effects on prior balances. Treat ADR-007 as the current decision baseline. If genuinely new policy is unresolved, document the impact in `docs/OPEN_QUESTIONS.md` and wait for user direction; do not silently revise an accepted decision. Update docs and `docs/PROJECT_STATUS.md` from verified evidence.
## Phase tracking

Map each financial change to `PH-03` (credit sale), `PH-04` (payment/allocation), `PH-05` (aging), `PH-06` (historical import), or `PH-07` (correction/audit). Read the active phase and its exit gate in `docs/PROJECT_STATUS.md` / `docs/IMPLEMENTATION_PLAN.md`; update status only from verified changes.
