---
id: '273'
title: Make Pi TypeScript extensions importable into imp
slug: make-pi-typescript-extensions-importable-into-imp
status: open
priority: 1
created_at: '2026-04-27T03:56:03.918483Z'
updated_at: '2026-04-27T03:58:06.374523Z'
notes: |-
  ---
  2026-04-27T03:58:06.374521+00:00
  User clarified: existing Pi extensions are available locally under `.pi/agent/extensions` and imp already has an `import` CLI command that imports from Pi; extension import can be added to that existing command rather than designing a brand-new import surface.
labels:
- cross-project
- imp
- pi
- extensions
- typescript
- guest-runtime
- import
verify: cd /Users/asher/tower && mana show 44 >/dev/null || test -d imp
kind: epic
---

Goal: coordinate the cross-project path for making imp compatible with TypeScript extensions users can import from Pi.

Context:
- imp currently has shipped Lua extension support and architecture docs for a host-owned guest-runtime substrate.
- TypeScript extensions are preferred future direction but must not be described as shipped in imp yet.
- User wants users to easily import their extensions from Pi into imp.
- This crosses Pi extension packaging and imp's extension substrate, so root mana is the source of truth.

Plan / decomposition:
1. Inspect and document Pi's current extension/package shape: manifest fields, entrypoint conventions, runtime assumptions, declared tools/hooks/commands, capability/permission model, install/import behavior, and any package registry/reference format.
2. Define a shared extension package manifest contract that both Pi and imp can understand. It must represent existing imp Lua extensions and future Pi/TypeScript extensions without making TypeScript execution mandatory in phase 1.
3. Design imp's import-first UX: `imp extension import <path-or-pi-ref>` or equivalent. First support should validate/copy/link packages, surface requested capabilities, and clearly report unsupported executable runtimes instead of pretending TS can run before the guest runtime exists.
4. Add/sequence imp implementation tasks for host-neutral extension loading, manifest validation, capability grants, and backward-compatible Lua discovery.
5. Only after the package/import contract and capability policy are stable, add a TypeScript guest-runtime adapter behind the Rust host boundary.

Non-goals:
- Do not replace Lua.
- Do not route worker execution through TypeScript.
- Do not choose or implement a JS/TS engine before the shared manifest and host capability model are defined.

Acceptance:
- Root mana contains the cross-project plan and child work needed to make Pi TypeScript extensions importable into imp.
- The plan distinguishes import/package compatibility from executable TypeScript runtime support.
- Follow-on units can be executed independently by agents inspecting Pi and imp.
