---
name: toko-web-slice
description: Implement or plan one Keuangan Toko web feature as a coordinated React/TypeScript, Rust/Axum, and PostgreSQL vertical slice using the repository's approved decisions and acceptance criteria.
---

# Keuangan Toko Web Vertical Slice

Use for implementation or detailed planning of one web feature. Do not use to start desktop work before web stabilization.

## Before changing code

Read `AGENTS.md`, `docs/IMPLEMENTATION_PLAN.md`, the relevant FR and AC in `docs/FEATURE_SPECIFICATIONS.md`, its OQ dependencies, and applicable API/data/security docs. Inspect the actual repository structure and manifests; the proposed monorepo layout and commands are not facts until present.

State the feature ID, decision labels, dependency decisions, smallest user flow, request/response example, UI loading/empty/error/success states, and AC to verify. If a money/access/API decision is open, do not silently select it: prepare a reviewable UI mock or contract proposal and identify the exact blocker.

## Build the slice

- Develop React UI and Axum API together; use an explicitly labeled mock only while needed to validate the flow.
- Keep business rules in domain/application layers where the repo supports them; keep HTTP and PostgreSQL adapters at their boundaries. React uses a typed API client and never connects to the database.
- Validate at request boundary and domain boundary. Server owns authoritative saldo/allocation. Use atomic DB writes and idempotency for sensitive retries only under an approved contract.
- Enforce approved permissions on the server. Do not expose write APIs while auth/permission policy remains undecided.
- Update API, data dictionary, feature docs, and `OPEN_QUESTIONS.md` when changes reveal a gap. Do not change an approved public contract or historical format without an explicit decision and compatibility plan.

## Verification and status

Discover commands from manifests/workflows, then run relevant available checks and report their exact results. Do not invent lint, typecheck, test, build, or migration commands. For AC, verify UI through API and PostgreSQL when those components exist. Update `docs/PROJECT_STATUS.md` with evidence and mark a phase complete only when its acceptance criteria are verified.
## Phase tracking

Work from the repository root `D:\azka\keuangan-toko`. Before coding, map the task to an active `PH-*` from `docs/IMPLEMENTATION_PLAN.md`; include its phase ID with FR/AC/OQ. Update `docs/PROJECT_STATUS.md` with changed evidence and remaining gate. Do not advance a phase because a future slice was merely planned.
