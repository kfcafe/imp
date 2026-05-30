# Workflow API surface

Status: planned

This workflow is intentionally blocked behind `benchmark-workflow-e2e`. The API should expose proven workflow core behavior rather than introduce a second workflow engine or paper over gaps in the native tool.

Initial scope:
- local-first API surface
- list workflows
- read workflow state
- run workflow next-action selection
- safe workflow updates through the existing mutation/event path
- read event history
- small client/dashboard-oriented dogfood

Non-goals for the first version:
- hosted multi-tenant workflow service
- replacing file-backed workflow storage
- remote authentication/collaboration semantics
