---
id: '257'
title: Draft imp ontology.md for shared feature/runtime language
slug: draft-imp-ontologymd-for-shared-featureruntime-lan
status: open
priority: 2
created_at: '2026-04-15T01:22:42.880364Z'
updated_at: '2026-04-16T08:49:55.236565Z'
acceptance: A first-pass ontology.md exists in the imp project with a useful category structure and clear definitions for the core terms needed to discuss imp's features and functionality.
notes: |-
  Created from user request to establish shared language for imp features/functionality.

  ---
  2026-04-15T01:25:16.371200+00:00
  Drafted /Users/asher/imp/ontology.md as a first-pass shared vocabulary document. The draft defines core naming rules, feature families, and normalized terms for surfaces, modes, runs, sessions, context/memory layers, coordination with mana, policy/capabilities, tools/extensions, providers/models, personality/config, and output/observability. It also explicitly calls out overloaded terms like profile, run, worker, and view for future normalization.

  ---
  2026-04-15T01:25:40.862200+00:00
  Plan for next pass:
  1. Tighten ontology.md from broad first draft into a more canonical vocabulary standard.
  2. Add an explicit preferred-terms / avoid-or-qualify table so new docs can normalize language quickly.
  3. Split or label terms by status where useful: current runtime reality, target-state architecture, and planned/proposed language.
  4. After the ontology stabilizes, optionally follow with doc-alignment work for README/ARCHITECTURE/rebuild docs so high-frequency language matches the ontology.

  Current state:
  - First-pass ontology draft exists at imp/ontology.md.
  - The draft already covers surfaces, modes, runs, sessions, context/memory layers, mana coordination terms, policy/capability terms, tools/extensions, providers/models, personality/config, and output/observability.
  - The most overloaded terms to normalize next are run, profile, worker, and view.

  ---
  2026-04-16T04:52:53.456294+00:00
  User now wants a dedicated imp_ontology.md focused on naming the future-facing architecture pieces and candidate names, not only descriptive ontology cleanup. The document should enumerate the parts that need names, give a small description of each, and propose strong candidate names for iteration. This should reflect the broader future-facing discussion: imp as reusable runtime/API, generalized durable state engine beyond strict file-native mana, TypeScript-first extensions, and alignment with the /agents vision.

  ---
  2026-04-16T04:56:36.215694+00:00
  Created /Users/asher/imp/imp_ontology.md as a future-facing naming workshop distinct from ontology.md. The new file enumerates major architecture parts, gives short descriptions, and proposes candidate names/current leans for ecosystem layers, runtime concepts, durable state concepts, extensibility concepts, and shell/surface concepts. It explicitly reflects current discussion: imp as reusable runtime/API, generalized durable state engine beyond strict file-native mana, TypeScript-first extensions, and alignment with the /agents object/assertion/event/rule worldview.

  ---
  2026-04-16T05:00:30.091028+00:00
  User clarified the desired shape for imp_ontology.md: short, easy, alignment-oriented glossary rather than a long naming workshop. Reworking the file into a compact preferred-terms reference with small definitions and a few key prefer/avoid naming rules for future imp docs and architecture discussions.

  ---
  2026-04-16T05:09:20.328250+00:00
  Naming discussion to explore next: invert current public framing so `mana` names the underlying platform/substrate that apps are built on, while `imp` names the agent and possibly the primary human-facing shell built on that substrate. Important caveat to evaluate: if `mana` is renamed upward to the runtime/platform, the durable state layer beneath it still needs a clear code/function-oriented name (`mana state`, `state engine`, similar) so the architecture does not lose its distinction between execution and durable truth.

  ---
  2026-04-16T05:18:52.613554+00:00
  New naming pressure: user questions whether `shell` should remain a distinct term at all, and suggests `imp` may simply be the human-facing environment/shell synonym rather than requiring a separate umbrella term. Need evaluate whether separating agent identity from human-facing environment is still useful, or whether `imp` should directly name both the agent and the primary interactive environment built on mana.

  ---
  2026-04-16T05:21:51.739319+00:00
  imp_ontology.md was tightened again. `shell` was removed as a core ontology term. Current short framing is now: `mana` = platform, `imp` = agent + default human-facing environment, with neutral internal names `runtime`, `state`, `module system`, and `adapter layer`. User is now asking for the next practical plan and expects the refactor process to reveal holes, mismatches, and hardening opportunities.

  ---
  2026-04-16T05:25:45.702737+00:00
  Naming direction update from user: prefer `extension` over `module` as the main packaged extensibility term in imp_ontology and future architecture language. User is also actively evaluating replacements for the internal terms `capability`, `state`, and `runtime`, so future ontology revisions should treat those as unsettled and propose stronger alternatives.

  ---
  2026-04-16T05:30:03.956820+00:00
  Naming set chosen for the compact imp ontology: `mana`, `imp`, `runtime`, `graph`, `extension`, `action`, and `task`. This replaces earlier tentative preferences for `state`, `module`, and `capability` in the short alignment glossary.

  ---
  2026-04-16T08:49:55.236554+00:00
  Naming signals from architecture interview:
  - user is considering a very simple trio around `task`, `prompt`, and `run`, and explicitly asked `why not prompt?` for the middle scoped-handoff concept
  - terminology split now emerging: behavior customization should be called `skill`, while action-providing packaged extensibility should be called `extension`
  - prompt rendering is currently seen as imp-owned rather than mana-owned, though mana may still assemble the structured scoped input that imp renders from
labels:
- docs
- imp
- ontology
kind: job
decisions:
- 'Normalization decision for the ontology work: ontology.md should become the canonical vocabulary reference for imp, and term entries should indicate whether they describe current runtime reality, a target-state architectural concept, or planned/proposed language.'
---

Create an ontology document for imp that establishes a stable shared vocabulary for describing imp's features, runtime surfaces, workflow concepts, and boundaries with mana. Start by surveying current imp docs and code-facing terms, then draft a first-pass ontology.md with term definitions, category structure, and explicit notes on ambiguous or overloaded language that should be normalized later.
