---
name: toko-project-status
description: Inspect and report Keuangan Toko project progress, current phase, decisions, blockers, and next work from repository evidence; maintain the project status page when progress changes.
---

# Keuangan Toko Project Status

Use this skill when asked where the project stands, what is complete, what is missing, or to refresh its progress record.

## Source of truth

Read `AGENTS.md`, `README-DOCS.md`, `docs/IMPLEMENTATION_PLAN.md`, `docs/OPEN_QUESTIONS.md`, `docs/PROJECT_STATUS.md`, and relevant ADRs. Then inspect current `git status` and actual source manifests, migrations, CI, and application files. Treat `PROJECT_STATUS.md` as a prior snapshot, not evidence that the current tree still matches it.

Separate:

- approved product/platform decisions from proposals and open questions;
- documentation prepared from application code implemented;
- code present from behavior verified;
- local workbook structure from semantically validated/imported data;
- test/build commands discovered from results actually run.

## Status workflow

1. Map each phase in `docs/IMPLEMENTATION_PLAN.md` to its current evidence: files, FR/AC, manifests, migrations, and verification results.
2. Read dependencies in `OPEN_QUESTIONS.md`; never infer an answer from an option or recommendation.
3. Identify the active phase and blockers, distinguish immediate blockers from future phase dependencies, and propose the smallest next action.
4. When asked to refresh or when the current task materially changes progress, update `docs/PROJECT_STATUS.md` with dated, evidence-backed status. Do not change business decision labels as a side effect of status reporting.
5. Report uncertainty explicitly. Use `belum tersedia` when no implementation command or evidence exists; do not call a design or plan “implemented” as application functionality.

## Project-specific baseline

The baseline project is web-first: Rust/Axum API, React/TypeScript, PostgreSQL; Tailwind CSS and shadcn/ui are approved in ADR-006. GPUI/SQLite desktop follows web stabilization. Excel is historical evidence only; formula-derived balance cells are not authoritative. The current project snapshot is in `docs/PROJECT_STATUS.md`; always refresh it from the tree.
## Repo-local phase governance

The canonical project root is `D:\azka\keuangan-toko`. Project skills live in `.agents/skills/`; use the repository copy as authoritative. Track work with phase IDs `PH-00` through `PH-09` from `docs/IMPLEMENTATION_PLAN.md`. For every status report, state the active phase, status, evidence, blockers, and next gate; update `docs/PROJECT_STATUS.md` when evidence changes. The current table is a snapshot—inspect the working tree every time.
