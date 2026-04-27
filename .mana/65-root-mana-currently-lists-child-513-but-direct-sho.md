---
id: '65'
title: Root mana currently lists child 51.3 but direct show/update resolution for 51.3 fails
slug: root-mana-currently-lists-child-513-but-direct-sho
status: open
priority: 3
created_at: '2026-04-09T12:06:03.352370Z'
updated_at: '2026-04-24T05:33:46.770414Z'
labels:
- fact
verify: cd /Users/asher/tower && mana list --all --parent 51 | rg -q '51\.3' && ! mana show 51.3 >/dev/null 2>&1
checkpoint: cfc6cee411f353d311fb044002b2c84346ab1ac4
verify_hash: '9ea7ebd138b3bb3dc3b016a7e9e63131c46c17ab66df3538fb63209ff69abebd'
kind: epic
unit_type: fact
last_verified: '2026-04-09T23:16:10.195220Z'
stale_after: '2026-05-09T23:16:10.195220Z'
paths:
- '.mana'
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-09T23:15:34.660738Z'
  finished_at: '2026-04-24T05:33:46.770414Z'
---
