---
name: toko-frontend
description: Build or review Keuangan Toko React/TypeScript interfaces using the approved Tailwind CSS and shadcn/ui stack, with API-backed states and accessible financial workflows.
---

# Keuangan Toko Frontend

Use for React/TypeScript UI, component, styling, and frontend review work in this project.

## Approved stack and version policy

Tailwind CSS and shadcn/ui are approved by `docs/decisions/ADR-006-tailwind-shadcn.md`. At frontend bootstrap, use the latest official stable releases supported by the selected project setup, never prerelease/canary by default, and record resolved versions in the actual package manifest and lockfile. Consult current official documentation when setup details may have changed. Vite and package manager are not yet approved; inspect repo and `OPEN_QUESTIONS.md` before choosing them.

## UI behavior

- Read the relevant feature spec and decision dependencies before implementing a flow.
- Build around typed API data; mocks must be clearly marked and must not masquerade as persistence or final saldo.
- Provide visible loading, empty, success, validation-error, and network-error states appropriate to the flow.
- Keep money values out of binary floating-point calculations in the client; display server values and formatting rules agreed for the feature.
- Use shadcn/ui components as editable local UI building blocks and Tailwind utilities for layout/styling. Preserve keyboard operation, visible focus, semantic labels, contrast, and responsive behavior.
- Do not invent form rules, statuses, or workflow transitions while their OQ items remain open. Make unresolved choices explicit in a proposal or disabled prototype state.

Before closing work, check actual available typecheck/build/lint commands from package manifests and report only commands run. Update feature/API docs if UI uncovers a contract gap.
## Phase tracking

Work from `D:\azka\keuangan-toko` and follow the active phase in `docs/PROJECT_STATUS.md`. Frontend tasks normally belong to `PH-02` through `PH-08`; identify the matching `PH-*` and FR/AC before changing UI. Record verified UI states and blockers in the project tracker when the task changes phase evidence.
