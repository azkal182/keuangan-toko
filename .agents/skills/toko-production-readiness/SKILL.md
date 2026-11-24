---
name: toko-production-readiness
description: Assess and prepare Keuangan Toko for secure web release, operations, backup/restore, observability, and rollback using actual repository and deployment evidence.
---

# Keuangan Toko Production Readiness

Use for a requested release-readiness review or when preparing an actual production deployment. Read `docs/QUALITY.md`, `docs/SECURITY.md`, `docs/OPERATIONS.md`, relevant `OPEN_QUESTIONS.md` items, and inspect real manifests, CI, infrastructure, and deployment configuration.

## Readiness review

Report each gate as evidenced, missing, or not applicable: authentication and server authorization; input/domain validation; atomic financial posting and idempotent retry; audit and redaction; secret management; CORS/transport; migrations and rollback/restore; backups and restore evidence; monitoring/alerts; operational ownership; browser/accessibility/performance targets; import file handling; release rollback. Separate design documents from implemented controls and tests.

Do not invent hosting, credentials, environment names, ports, CI commands, backup schedules, RPO/RTO, or deployment steps. Resolve the relevant OQ and inspect repo configuration first. Never expose secrets or customer data in reports.

## Mutating production systems

Prepare a concrete release plan, version, migration impact, backup/restore evidence, rollback path, and exact commands before requesting approval for a production mutation. Do not deploy, run a destructive migration, or alter live data without explicit authorization for that action. Do not treat successful compilation as production readiness.

## Closeout

Run only checks that exist in the repo and report command plus result. Update operational/security docs and `docs/PROJECT_STATUS.md` with evidence, unresolved risks, and the actual release state. A readiness review is not a deployment and must not be reported as one.
## Phase tracking

Production readiness is `PH-08`; desktop offline-first is `PH-09` and follows a stable web release. Check the active phase and evidence in `docs/PROJECT_STATUS.md` before reporting readiness. Keep release assessment, approval to deploy, and completed deployment as separate states.
