# Fork Docs Contributing Guide

Use these docs to keep fork behavior explicit and portable across upstream release ports.

## Required Documents

Every fork feature must have:

- `docs/fork/features/<code-name>.md`
- `docs/fork/projects/<code-name>/README.md`
- `docs/fork/projects/<code-name>/design.md`
- `docs/fork/projects/<code-name>/verification.md`
- `docs/fork/research/<release>/<code-name>.md`

## Authoring Rules

- Keep feature docs user-contract focused: behavior, scope, compatibility, verification.
- Keep project docs implementation focused: state, data flow, source-of-truth files, tests.
- Keep research docs release-specific: upstream baseline, gaps, risky files, conflict points.
- Do not describe planned behavior as already implemented.
- Use an upstream-first implementation shape unless the feature contract explicitly documents a fork
  divergence.
- Preserve upstream behavior, APIs, public surfaces, and stable wire compatibility by default.
- Do not add database columns or migrations unless the feature contract proves they are required.
  Prefer existing session/thread JSON metadata, rollout/session files, and backward-compatible
  persisted structures.
- Document how older sessions and persisted data behave when new fields are missing.
- Do not introduce silent fallbacks. Any fallback must be explicit, controlled, observable to the
  caller or operator, documented in the feature contract, and covered by verification.
- When stable app-server protocol changes, state whether each field/method is stable or experimental.
- When protocol, config schema, app-server schema, snapshots, or other canonical generated artifacts
  change, regenerate them through the normal project command and keep only meaningful generated
  deltas.
- When TUI behavior changes, include snapshot coverage in the verification plan.
- Add a dated changelog entry when a contract or implementation decision changes.
- Keep unrelated diffs, generated noise, build hashes, lockfile changes, and docs for unrelated
  features out of the feature change set.

## Quality Delivery Loop

For each feature:

1. Research the upstream baseline and risky integration points.
2. Define the feature contract: goal, scope in/out, source of truth, affected subsystems,
   compatibility, permissions/security, persistence, failure modes, and no-fallback rules.
3. Build an integration map that covers parsing/schema, config loading, runtime state,
   permissions/sandbox, environment selection, persistence, resume/restart, events/API/UI
   projections, tests, and docs.
4. Create or update the verification matrix before finalizing the implementation.
5. Implement in small coherent slices.
6. Run focused tests for the feature and affected integration points.
7. Use independent sub-agent audits. Reuse an existing free agent with relevant
   context before spawning a new one.
8. Run an architecture/contract audit for every feature. Add a permissions/security/runtime audit
   when the feature touches cwd, execution, sandbox, config, persistence, or resume.
9. Fix all High and Medium audit findings. A High or Medium finding may remain only with explicit
   owner approval, and the feature is blocked until that approval and the deferred risk are
   documented in the feature dossier.
10. Commit the implemented, audited, fixed, documented, and verified feature before starting the
    next feature.

Done means the diff only contains files required for the feature, `git diff --check` passes, required
project checks pass, docs match the implementation, security boundaries have positive and negative
coverage, permissions are not widened accidentally, and runtime/config/persistence/resume behavior
remains consistent.

## Release Ports

For a new upstream release, create or update `research/<release>/` before coding. Use the
release tag commit as the baseline and prefer upstream-shaped adaptations over literal backports
unless the feature contract requires otherwise.
