# Prompt-template and shell-tool storage wiring audit

This audit resolves mana unit `264.9`, a follow-up from the broader storage topology audit. It determines whether prompt-template files and TOML-defined shell-tool roots are active production storage surfaces or should be treated as experimental/unwired before the storage contract preserves them.

## Inspected files and searches

Primary files:

- `crates/imp-core/src/resources.rs`
- `crates/imp-core/src/tools/shell.rs`
- `crates/imp-core/src/storage.rs`

Search command:

```sh
rg -n "discover_prompts|PromptTemplate|load_shell_tools|ShellToolDef|ShellTool::new|global_prompts_dir|project_prompts_dir" \
  crates/imp-core/src crates/imp-cli/src crates/imp-tui/src crates/imp-lua/src -g'*.rs'
```

## Prompt-template surface

### Defined behavior

`resources.rs` defines:

- `PromptTemplate { name, path, content }`
- `PromptTemplate::expand(&HashMap<String, String>)`
- `discover_prompts(cwd, user_config_dir)`

Current discovery roots:

- user-global: `<user_config_dir>/prompts`
- project-local: `storage::project_prompts_dir(cwd)` → `<cwd>/.imp/prompts`

Current discovery behavior:

- only immediate files in those two directories are considered;
- only `.md` files are loaded;
- name is file stem;
- content is full file content;
- no recursive project ancestry walk for prompts;
- no collision/precedence override logic beyond returning a vector in user-then-project order.

`storage.rs` also defines canonical helpers:

- `global_prompts_dir()` → `~/.imp/prompts`
- `project_prompts_dir(project_dir)` → `<project>/.imp/prompts`

### Production call-site finding

No production call site was found for `discover_prompts` or `PromptTemplate` outside `resources.rs` and its unit tests.

Observed references:

- helper/type definitions in `resources.rs`;
- prompt discovery and expansion tests in `resources.rs`;
- storage path helper definitions/tests in `storage.rs`.

No references were found in CLI, TUI, agent runtime, context assembly, tool registration, command parsing, or extension loading paths.

### Classification

Prompt templates are **defined but unwired/experimental** in the current shipped path.

They should not be deleted casually because tests and storage helpers preserve an intended product surface, but the normalized storage topology should not describe them as fully active runtime behavior until a real consumer exists.

### Recommendation

Keep canonical roots reserved:

- `~/.imp/prompts`
- `<project>/.imp/prompts`

But document status as **experimental/unwired** and avoid promising runtime discovery semantics beyond the helper itself.

Before promoting to supported:

1. decide consumer: shell command, slash/colon command, prompt picker, or extension API;
2. define precedence/collision behavior between global and project prompts;
3. decide whether project prompts should walk ancestry like skills/instructions or only use current project root;
4. add production call-site tests.

## shell-tool TOML surface

### Defined behavior

`tools/shell.rs` defines TOML-backed shell tools:

- `ShellToolDef`
- `ShellParamDef`
- `ShellExecDef`
- `ShellTool`
- `load_shell_tools(dir, registry)`

Current loader behavior:

- walks the provided directory recursively;
- loads only `.toml` files;
- parses each TOML file into `ShellToolDef`;
- registers valid definitions into a `ToolRegistry`;
- silently skips invalid TOML definitions;
- returns an error if walking/reading the directory fails;
- missing directory is a no-op.

`storage.rs` defines canonical helper candidates:

- `global_tools_dir()` → `~/.imp/tools`
- `project_tools_dir(project_dir)` → `<project>/.imp/tools`

### Production call-site finding

No production call site was found for `load_shell_tools` outside `tools/shell.rs` tests.

Observed references:

- type/helper definitions in `tools/shell.rs`;
- shell-tool unit tests including `load_shell_tools_registers_valid_defs_and_skips_invalid_ones`;
- storage helper definitions/tests for global/project tools dirs.

No references were found in builder/tool registry construction, CLI startup, TUI startup, Lua extension loading, agent context assembly, or project resource discovery.

### Classification

TOML shell tools are **implemented as a loader and executable tool type, but unwired/experimental** in the current shipped path.

Because these tools execute arbitrary commands, they should not be silently promoted to active production discovery as part of storage normalization. Enabling them requires policy and UX decisions, not just path cleanup.

### Recommendation

Keep roots reserved only as experimental candidate roots:

- `~/.imp/tools`
- `<project>/.imp/tools`

Do not describe TOML shell-tool loading as automatically active until a separate implementation explicitly wires discovery into tool registry construction with policy controls.

Before promoting to supported:

1. define trust boundary for global vs project TOML tools;
2. decide whether project tools require approval/trust confirmation;
3. decide mode behavior (`read-only`, `full`, restricted/headless);
4. surface loaded tools in a discoverable command/help output;
5. add integration tests proving registry construction loads expected roots;
6. add security review for arbitrary command execution and interpolation rules.

## Storage-topology implication for epic 264

For the normalized storage contract:

- prompts and shell tools should be listed as **reserved/experimental surfaces**, not core active surfaces;
- their roots can remain in `storage.rs` to avoid future drift;
- migration tooling should not auto-create, import, or rewrite these directories unless a concrete consumer exists;
- docs should distinguish “path helper exists” from “runtime uses this path.”

This avoids preserving dead baggage as if it were shipped behavior while still recording the intended extension points.

## Recommended follow-on work

Only create implementation work if product direction wants these surfaces active.

Potential prompt follow-up:

- “Wire prompt templates into CLI shell command discovery” with explicit precedence and tests.

Potential shell-tool follow-up:

- “Design policy-gated TOML shell-tool loading” before enabling automatic project/global loading.

Security-sensitive note: shell-tool loading should not be enabled just because storage roots exist. It executes configured commands and therefore needs clear trust and approval semantics.

## Conclusion

Both prompt templates and TOML shell tools are source-defined and tested, but neither has a production call site in the inspected shipped paths. Treat them as experimental/reserved storage surfaces in the topology. Do not migrate or promise them as active runtime behavior until separate product and policy work wires them deliberately.
