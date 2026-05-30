# Cleanup plan

1. Dependency cleanup: inspect dependency tree for vulnerable packages; remove direct unused vulnerable dependencies or update safe versions.
2. Docs cleanup: ensure README/docs distinguish stable, preview, planned, and out-of-scope surfaces; ACP must not be represented as stable for 0.3.0.
3. Dead code cleanup: run reference scans for prior personality/mana/improve leftovers. Archive only files with zero active references and no default build impact.
4. Archive manifest: if anything moves, create `~/imp-archive/imp-0.3.0-release-candidate/MANIFEST.md` with source path, destination, reason, and restore instructions.
