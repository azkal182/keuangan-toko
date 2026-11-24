---
name: toko-excel-migration
description: Profile, map, validate, stage, and reconcile Keuangan Toko's historical customer workbook without trusting formula-derived balances or exposing customer data.
---

# Keuangan Toko Excel Migration

Use for workbook inspection, importer design/implementation, staging review, or reconciliation. Read `AGENTS.md`, `docs/OPEN_QUESTIONS.md` (especially OQ-06, OQ-07, OQ-13), `FR-006`, `docs/DATA_DICTIONARY.md`, and `docs/SECURITY.md` first.

## Source handling

- Treat the workbook and all cell values as sensitive customer/financial data. Keep it local and out of commits, logs, generated examples, and documentation. Do not print raw customer names, addresses, references, or whole rows in tool output.
- Begin with structural profiling and aggregate counts. Explain which parser/tool was used and what it could verify. A sheet count, XML dimension, cached value, or historical report is not semantic validation.
- Do not execute workbook formulas/macros or trust cached formula outputs for saldo, status, remaining balance, or aging. Debit and credit source columns are candidates requiring checks and reconciliation.

## Import behavior

- Preserve raw source and provenance (sheet/row/cell mapping) in staging; normalize candidates separately.
- Flag invalid/missing dates, ambiguous customer identity, duplicate references/files, debit-credit mixed rows, formula errors, and quantity×price discrepancies for review. Never silently infer dates, merge invoice rows, discard records, or commit partial rows unless an approved decision explicitly permits it.
- Reconcile per customer and batch; show variances and unresolved exceptions before approval. Require the authorized human approval flow before production commit.
- Use deduplication/idempotency and transactional commit only according to the approved import contract. Keep audit lineage from source row to committed records.

## Completion

Update `OPEN_QUESTIONS.md` when new policy gaps surface, and update importer docs/status with aggregate evidence only. Report known limits and unverified business interpretation. Do not claim migration complete until approved reconciliation and commit evidence exist.
## Phase tracking

Workbook profiling and importer implementation belong to `PH-06` in `docs/IMPLEMENTATION_PLAN.md`. Work from the repo root `D:\azka\keuangan-toko`; check the current phase status and prerequisites before parsing or building the importer. Update the phase tracker with aggregate-only evidence and human reconciliation status.
