# imp host, imp mirror, and imp daemon

Status: draft
Date: 2026-05-22

Purpose: define the hosted/self-hosted architecture for syncing imp work across devices and teams while keeping agent computation on trusted machines by default.

## Naming

Use lowercase product phrases in docs and UI:

- **imp** — local CLI/TUI agent runtime.
- **imp work** — durable tasks, memory, runs, checks, evidence, context, and handoff.
- **imp run** — one bounded execution attempt, and the command that starts one.
- **imp host** — optional hosted or self-hosted sync/control service.
- **imp mirror** — optional git object/ref mirror managed by imp host.
- **imp daemon** — trusted local process that connects to imp host and executes runs on a user's machine.
- **imp agent** — future custom role/profile/tool-policy definition.

Use `imp-work` only when referring to the internal implementation/storage subsystem.

## Summary

imp should remain useful as a fully local coding agent. The hosted product should not start as a hosted autonomous engineer. The first hosted product should be **imp host**: a durable coordination service for imp work, run events, evidence, approvals, devices, and optional code mirrors.

The key boundary:

> imp host stores work, events, evidence, approvals, and optional git mirrors. imp daemon executes runs on trusted machines.

This gives solo developers and small teams cross-device continuity without asking them to hand arbitrary code execution to a SaaS worker.

## Goals

- Sync imp work across devices and small teams.
- Keep local imp useful without hosted services.
- Allow a phone/tablet/web controller to create tasks, watch runs, and approve actions.
- Allow trusted machines to execute work through imp daemon.
- Preserve exact code state for runs through repo refs and optional imp mirror.
- Attach evidence, checks, and artifacts to durable runs.
- Make hosted execution optional and later, not foundational.
- Support self-hosting as a natural trust path.

## Non-goals

- Do not build a GitHub replacement or forge UI.
- Do not build issues, PR review UI, branch protection, social features, or code browsing first.
- Do not require private source code mirroring for basic hosted work sync.
- Do not live-migrate an in-memory agent loop between devices.
- Do not make iPhone/iPad a full shell-capable executor.
- Do not make hosted SaaS execution the default trust model.

## Product model

### Local-only imp

Local imp works without imp host:

- TUI/CLI agent runtime
- local tools
- local provider auth/secrets
- local sessions
- local imp work
- local evidence and traces

### imp host

imp host is the optional server:

- projects
- devices
- imp work sync
- run records and runtime events
- approvals
- evidence and artifact metadata
- artifact/blob storage
- daemon registration and run queue
- optional imp mirror per project
- team/project permissions

### imp daemon

imp daemon is the trusted executor process:

- runs on a user's Mac/Linux/Windows machine or self-hosted server
- connects to imp host
- advertises capabilities
- claims run requests
- executes imp runs locally against local checkouts/worktrees
- uses local tools, secrets, and provider credentials unless configured otherwise
- streams runtime events back to imp host
- uploads evidence/artifacts/patches/refs

### imp mirror

imp mirror is an optional code-state subsystem inside imp host:

- bare git mirror / object cache
- upstream fetch/prune
- internal imp refs for runs, checkpoints, patches, and prototypes
- stable code refs for evidence/context/reproducibility
- bundle or patch import from daemons
- later smart HTTP/SSH git support if needed

imp mirror is not a forge. It should not grow human-first GitHub features unless the product direction changes deliberately.

## Device classes

### Full executor

Mac/Linux/Windows workstation or server.

Can:

- run imp daemon
- execute agent runs
- read/write local repos
- use shell and git
- run tests/builds
- access local secrets
- stream events and evidence to host

### Controller

Phone, iPad, browser, or lightweight desktop UI.

Can:

- view tasks/runs/evidence
- create and edit work
- approve or deny actions
- comment or record decisions
- trigger a run on an available daemon
- inspect artifacts and patches

Usually cannot:

- run arbitrary shell commands
- build/test code locally
- manipulate a full working tree

### Light executor

Future constrained device such as iPad.

May support:

- planning
- review
- docs edits
- model-only tasks
- limited file editing through synced documents

Should not be treated as a full executor.

### Hosted executor

Future optional worker managed by the service.

Should require stronger isolation, policy, and approval boundaries than imp daemon. Hosted execution is not required for the first hosted product.

## Architecture

```text
                 controller device
              web / phone / tablet
                       |
                       | tasks, approvals, event streams
                       v
+--------------------------------------------------+
|                    imp host                      |
|--------------------------------------------------|
| Postgres: projects, work, runs, events, leases   |
| Object storage: evidence, logs, patches, traces  |
| imp mirror: optional bare git repos / refs       |
| Realtime: SSE/WebSocket run and project streams  |
+--------------------------------------------------+
          ^                              |
          | sync/events/requests         | optional fetch/sync
          |                              v
+-----------------------+        upstream git host
|      imp daemon       |        GitHub/Gitea/GitLab/etc.
|-----------------------|
| local imp runtime     |
| local checkout        |
| local shell/tools     |
| local secrets         |
+-----------------------+
```

## Storage choice

### Hosted source of truth: Postgres

Use Postgres for imp host.

Reasons:

- multi-user/project permissions
- transactional run/task updates
- row-level locks or leases
- concurrent daemons
- queryable task queues
- runtime event streams
- approval state
- artifact metadata
- audit history
- operational maturity

### Local state: existing imp work, later SQLite/cache if needed

Local imp can continue using the current imp-work store. A future local sync cache may use SQLite or an append-only local event journal, but the hosted service should not be constrained by the local storage format.

### Git objects: bare repos, not Postgres

imp mirror should store git objects in bare repositories or a git-aware storage layer. Postgres stores metadata and refs, not raw git objects.

### Artifacts: object storage

Store large blobs outside Postgres:

- evidence packets
- traces
- command logs
- patches
- bundles
- screenshots
- context packs if large

## Hosted domain model

Initial entities:

```text
orgs
users
memberships
projects
repositories
devices
daemons
work_items
work_links
memory_items
decisions
context_packs
runs
runtime_events
approvals
checks
evidence
artifacts
leases
repo_refs
policy_profiles
```

### Project

```text
id
org_id
name
slug
created_by
created_at
updated_at
```

### Repository

```text
id
project_id
upstream_url
default_branch
mirror_enabled
mirror_path or storage_key
last_fetch_at
last_seen_commit
created_at
updated_at
```

### Work item

```text
id
org_id
project_id
kind: task | epic | prototype | memory | decision
status
title
description
acceptance
checks
paths
topics
assignee_id
created_by
updated_by
source_refs
version
created_at
updated_at
```

### Run

```text
id
org_id
project_id
work_item_id?
repository_id?
executor_kind: daemon | hosted | local_upload | human
executor_id?
status: queued | running | awaiting_approval | paused | completed | failed | blocked | canceled
autonomy
mode
agent_profile_id?
model
base_ref
result_ref?
started_at
ended_at
final_outcome
```

### Runtime event

Use the existing `RuntimeEvent` model as the basis:

```text
id
run_id
sequence
timestamp
event_type
payload_json
```

Sequence is monotonic per run. Clients reconnect with `after_sequence`.

### Daemon

```text
id
org_id
user_id
device_id
name
platform
version
capabilities
allowed_project_ids
allowed_repository_ids
max_autonomy
policy_profile_id
status: online | offline | busy
last_seen_at
created_at
updated_at
```

Capabilities may include:

```text
can_read_repo
can_write_repo
can_run_shell
can_run_tests
can_use_network
can_access_local_secrets
can_open_pr
can_accept_remote_runs
```

### Approval

```text
id
org_id
project_id
run_id
type
status: pending | approved | denied | expired
requested_by
resolved_by
summary
payload_json
created_at
resolved_at
expires_at
```

Approvals are first-class records, not transient websocket messages.

## Event-first sync model

imp host should be event-first with projections.

Events provide:

- audit history
- sync cursors
- replay/rebuild
- offline/local queue support
- runtime stream resume
- handoff lineage

Projections provide:

- current task list
- current run state
- current daemon presence
- pending approvals
- evidence lists

Event examples:

```text
work.created
work.updated
work.status_changed
memory.recorded
decision.recorded
run.requested
run.claimed
run.started
run.event_appended
approval.requested
approval.resolved
check.completed
evidence.attached
artifact.uploaded
repo.mirrored
repo.ref_updated
daemon.heartbeat
run.completed
```

Every event should include:

```text
id
org_id
project_id
aggregate_type
aggregate_id
sequence or project_cursor
actor_type: user | daemon | agent | system | integration
actor_id
device_id?
run_id?
timestamp
event_type
payload_json
idempotency_key
```

## Sync API

Initial API shape:

```text
POST /auth/device
GET  /projects
POST /projects

GET  /projects/:id/sync?after=<cursor>
POST /projects/:id/events

GET  /work-items
POST /work-items
PATCH /work-items/:id

POST /runs
GET  /runs/:id
GET  /runs/:id/events?after_sequence=<n>
POST /runs/:id/events
POST /runs/:id/cancel

GET  /approvals?status=pending
POST /approvals/:id/resolve

POST /artifacts
GET  /artifacts/:id

POST /daemons/register
POST /daemons/:id/heartbeat
GET  /daemons/:id/requests
POST /run-requests/:id/claim
POST /run-requests/:id/complete
```

Realtime can start with SSE:

```text
GET /projects/:id/events/stream
GET /runs/:id/events/stream
```

Use WebSockets later only if bidirectional realtime materially helps.

## imp daemon lifecycle

```text
1. user runs `imp daemon start`
2. daemon authenticates with imp host
3. daemon registers device/capabilities
4. daemon sends heartbeat
5. daemon polls/subscribes for run requests
6. daemon claims an eligible request
7. daemon prepares local repo/worktree
8. daemon runs imp-core with the run contract
9. daemon streams runtime events to host
10. daemon pauses on approval requests when required
11. daemon uploads evidence/artifacts/patches/refs
12. daemon records final outcome and releases lease
```

Proposed CLI:

```bash
imp host login
imp host status
imp host sync

imp daemon start
imp daemon status
imp daemon stop
```

Potential later commands:

```bash
imp daemon allow-project <project>
imp daemon allow-repo <repo>
imp daemon run-once
```

## Run request lifecycle

Controller or local imp creates a request:

```text
run_request:
  project_id
  work_item_id
  repository_id
  base_ref
  requested_capabilities
  autonomy
  policy_profile
  target_daemon_id?
```

Eligible daemon claims with a transaction:

```text
queued -> claimed -> running -> completed/blocked/failed/canceled
```

A run request should be idempotent. A daemon may crash; leases expire and the request can be retried or marked stale.

## Handoff model

Do not live-migrate active agent loops. Handoff happens at checkpoint boundaries.

A continuation run links to its parent:

```text
run A: local exploration
run B: daemon implementation, parent=A
run C: local review/takeover, parent=B
```

Handoff artifacts:

- context pack
- base ref
- result ref or patch
- runtime event summary
- evidence packet
- final/partial outcome
- approval state

The UI can present lineage as one workflow while storage keeps runs separate.

## imp mirror design

### Purpose

imp mirror preserves code state for agent work. It allows imp host and daemons to refer to exact git objects, agent result refs, checkpoints, patches, and prototype refs.

### Non-goals

- no forge UI
- no PR review UI
- no issue tracker
- no public social coding features
- no branch protection first
- no LFS first unless needed by real repos

### Mirror modes

#### Metadata-only mode

No code mirror. Store only:

- remote URL
- branch
- commit SHA
- patches/artifacts
- evidence

This is lowest trust friction and should be the default until users enable mirroring.

#### Mirror mode

imp host keeps a bare mirror:

```text
/repos/<org>/<project>/<repo>.git
```

Stores:

- upstream refs
- internal imp refs
- fetched objects

#### Runner-only mode

No server code storage. Daemon with local checkout handles all code operations and uploads only events/evidence/patch metadata.

### Internal refs

Use internal refs for agent outputs:

```text
refs/imp/runs/<run-id>
refs/imp/checkpoints/<checkpoint-id>
refs/imp/patches/<patch-id>
refs/imp/prototypes/<prototype-id>
refs/imp/context/<context-pack-id>
```

A run records:

```text
base_ref: main@abc123
result_ref: refs/imp/runs/R-123@def456
verification_ref: def456
```

### Bundle-based MVP

Avoid implementing a full git server initially.

Daemon can upload a bundle:

```bash
git bundle create run.bundle <base>..HEAD
```

Server imports into the bare mirror:

```bash
git fetch run.bundle HEAD:refs/imp/runs/<run-id>
```

This gives durable refs without smart HTTP/SSH git support.

### Later git protocol support

Add smart HTTP or SSH only when needed:

```text
git clone https://host.imp.dev/org/project.git
git fetch origin refs/imp/runs/<run-id>
git push origin HEAD:refs/imp/runs/<run-id>
```

## Relationship to upstream git hosts

imp host should integrate with existing git hosts instead of replacing them.

Upstream remains canonical unless the user explicitly chooses otherwise.

Initial integrations:

- remote URL detection
- commit/branch refs
- optional fetch mirror
- optional PR/open-branch integration later

Likely first external integration: GitHub. Later: Forgejo/Gitea, GitLab.

## iPhone and iPad

iPhone/iPad should be controller devices first.

Rust can compile to iOS, but a full imp executor on iOS is constrained by:

- no normal shell
- limited filesystem access
- background execution limits
- sandboxing/code signing
- difficult full-repo checkout/test workflows
- limited process execution

Design iOS/iPad support around:

- task list
- run stream
- approvals
- evidence view
- comments/decisions
- triggering a daemon on a trusted machine

A mobile-friendly web UI is likely enough before a native app.

## Security and privacy

### Default trust posture

Local execution is the default. imp host coordinates and stores state; imp daemon runs computation on trusted machines.

### Sensitive data classes

Potentially sensitive:

- prompts
- code excerpts
- diffs
- patches
- tool outputs
- evidence
- memory
- context packs
- private git objects

Less sensitive but still protected:

- task titles/status
- run status
- daemon presence
- check pass/fail
- artifact metadata

### Controls

Required early:

- project/org access control
- device/daemon registration and revocation
- token rotation
- audit events for sync, approvals, mirror fetches, artifact reads
- clear deletion/export behavior
- no secret values stored in host by default
- no local secrets sent unless a policy explicitly permits it
- approval records for sensitive actions

For imp mirror:

- mirror disabled by default or explicit opt-in
- private repo credentials stored securely
- encryption at rest
- access logs
- project deletion removes mirror and artifacts according to retention policy

For future hosted execution:

- isolated container or stronger sandbox per run
- no ambient production secrets
- egress policy/logging
- command timeout/output limits
- artifact redaction
- approval gates for side effects

## Policy model

imp host should store policy profiles, but imp daemon must enforce policy locally too.

Policy can constrain:

- allowed tools
- denied tools
- allowed write paths
- denied write paths
- allowed commands
- denied commands
- max autonomy
- approval requirements
- provider/model restrictions
- secret access
- mirror/code upload behavior

Host-side policy is for scheduling, approvals, and audit. Daemon-side policy is the final enforcement point for local execution.

## Why this is a product

This can be the paid service around open-source imp:

Free/local:

- local imp runtime
- local imp work
- local sessions/evidence
- local hooks/extensions

Paid/self-hosted:

- imp host sync
- cross-device work state
- daemon coordination
- evidence/artifact storage
- approvals from web/mobile
- optional imp mirror
- team projects
- retention/search
- GitHub/Gitea integration
- later hosted/self-hosted runners

This avoids charging for the CLI while offering durable value around continuity, coordination, and code/work state.

## Implementation phases

### Phase 0: design and schemas

- hosted domain model
- event schema
- runtime event mapping
- daemon contract
- mirror model
- auth/device model

### Phase 1: hosted imp work sync

- Postgres project/work/event store
- device auth
- `imp host login`
- `imp host sync`
- push/pull work events
- server-authoritative projections
- no code mirror required

Success criteria:

- two local clients can see/update the same project work
- repeated sync is idempotent
- local state can rebuild from server events

### Phase 2: run event upload and web/controller view

- local runs upload runtime events
- run detail page streams events
- evidence/artifact upload
- pending approval records
- web/controller can approve or deny

Success criteria:

- start run on laptop and watch from browser
- approval record appears and can be resolved
- evidence is visible after closeout

### Phase 3: imp daemon

- `imp daemon start`
- daemon registration/heartbeat
- run request queue
- daemon claims run
- daemon executes locally
- events/evidence stream to host

Success criteria:

- create run request from web/controller
- daemon claims and executes it
- controller sees progress and final outcome

### Phase 4: imp mirror metadata and public repo mirror

- repository table
- upstream remote URL and commit refs
- optional public repo bare mirror
- internal refs metadata
- fetch/prune

Success criteria:

- project can enable mirror for public repo
- host records base/result refs
- evidence links to exact commits

### Phase 5: bundle-based result refs

- daemon creates git bundle for run result
- uploads bundle artifact
- host imports bundle into imp mirror under `refs/imp/runs/<run-id>`
- controller can see result ref

Success criteria:

- local daemon result survives as host-managed ref
- another machine can fetch/apply the result through host-controlled artifact/ref

### Phase 6: private repo mirror and upstream integration

- GitHub App or deploy key flow
- private mirror fetch
- token revocation handling
- optional PR creation from result ref

Success criteria:

- private repo can opt into mirror
- daemon result can be promoted to upstream branch/PR after approval

### Phase 7: hosted/self-hosted execution expansion

- self-hosted daemon packages
- optional managed hosted runners
- stronger sandboxing
- billing/retention/team controls

## Open questions

- Should imp host be open-source/self-hostable from the start, or service-first with later self-hosting?
- Should mirror be opt-in per project or per repo? Recommendation: per repo.
- Should server store full prompts/tool outputs by default, or redact/encrypt sensitive payloads? Recommendation: clear defaults plus project policy.
- Should bundle upload be the first result-ref transport? Recommendation: yes.
- Should local imp continue to support file-backed imp work indefinitely? Recommendation: yes; hosted sync is optional.
- Should hosted work sync require auth to a central service, or allow arbitrary host URLs first? Recommendation: design protocol for arbitrary host URLs even if first deployment is official.

## Immediate next steps

1. Define Postgres schema for projects, work events, runs, runtime events, approvals, artifacts, daemons, repositories, and repo refs.
2. Define `imp host` CLI shape and config.
3. Define daemon registration and heartbeat protocol.
4. Define sync event envelope and idempotency rules.
5. Prototype local imp work sync to a local Postgres service.
6. Prototype daemon run request claim/execute/stream loop.
7. Prototype bundle upload into a bare mirror.
