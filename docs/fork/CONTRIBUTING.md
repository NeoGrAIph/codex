# Contributing Fork Documentation

This guide explains how to author and maintain fork-specific documentation under `docs/fork/`.
Use it together with [AGENTS.md](../AGENTS.md) and treat `AGENTS.md` as the policy source of
truth. This file is the practical authoring guide: how to structure docs, what to include, and
how to keep documentation aligned with the actual fork state.

## Purpose

Fork docs are not a changelog dump. They are the contract and research record for the fork.

Use `docs/fork/*` when you need to document:

- a fork-specific feature or material fork behavior change
- a release-specific adaptation against an upstream baseline
- an implementation dossier for a non-trivial fork feature
- a findings-first audit or review record

Do not use `docs/fork/*` for:

- general upstream product documentation
- temporary scratch notes with no contractual or research value
- file-by-file diff narration with no behavior-level contract

## Canonical layout

- `features/<code-name>.md`
  - The fork feature contract.
- `projects/<code-name>/`
  - The implementation dossier for a non-trivial feature.
- `research/<release>/`
  - Release-specific baseline, gap analysis, and risky integration notes.
- `audits/<code-name>.md`
  - Findings-first audit output.

One fork feature should have one canonical code name and one canonical package. Reuse the same
code name across feature, project, research, and audit docs.

## Naming rules

- Use `kebab-case` for code names in paths unless an established canonical name already exists.
- Use the real upstream release number for research folders, for example `0.118.0`.
- Use the feature code name for the project folder, for example:
  - `docs/fork/features/thread-note.md`
  - `docs/fork/projects/thread-note/README.md`
  - `docs/fork/research/0.118.0/thread-note.md`

If the runtime field uses a different shape, keep that distinction explicit in the text:

- code name in docs/path: `thread-note`
- wire/runtime field in code: `thread_note`

## Authoring workflow

Use this workflow for every new fork feature and every material change to an existing fork
feature.

1. Identify the baseline.
   - Resolve the upstream tag and commit.
   - Confirm the local branch pair and intended comparison range.
   - Record the baseline in release research before treating the implementation as done.

2. Determine the real current contract.
   - Read the current target branch and staged state.
   - Do not copy historical docs forward without checking whether the code still matches them.
   - Treat local commits as clues, not as automatic truth.

3. Decide the adaptation mode.
   - Explicitly record whether the feature is a literal port or an upstream-shaped adaptation.
   - If the implementation differs from the historical fork commit, explain why.

4. Write the feature contract.
   - Describe the user-visible and developer-visible behavior.
   - Call out what is upstream behavior, what is fork-specific, and what intentionally diverges.

5. Write the implementation dossier for non-trivial features.
   - Capture canonical state, data flow, invariants, and verification scenarios.

6. Write or update release research.
   - Record the gap against the baseline release.
   - Note conflict-prone files and release-specific verification.

7. Update adjacent docs when the contract changes.
   - If the change affects API, config, wire shape, TUI text, or operational behavior, update the
     relevant user/developer docs in the same change set.

## What each document must contain

### `features/<code-name>.md`

Minimum required sections:

- Feature passport
  - code name
  - status
  - goal
  - scope in/out
- User contract
  - exact behavior
  - transitions or mutations
  - empty/error states
  - critical wording when text matters
- Integration and compatibility notes
  - what remains upstream behavior
  - what is fork-specific
  - what intentionally diverges
- Verification matrix
  - required commands and what each validates
- Doc changelog
  - short dated entries when the contract changes

Recommended additions when useful:

- transcript behavior
- metadata-only vs model-visible contract
- persistence and recovery order
- migration or fallback notes

### `projects/<code-name>/README.md`

This is the entry point for the implementation dossier.

It should include:

- current status in one short block
- canonical links to the feature contract and release research
- a compact map of implementation surfaces
- a reading order for the rest of the dossier

### `projects/<code-name>/design.md`

This is the design/source-of-truth document.

It should include:

- canonical state
- projections derived from that state
- data flow
- invariants
- intentional tradeoffs

Use behavior-level descriptions. Avoid giant symbol inventories unless specific file pointers are
needed to prevent ambiguity.

### `projects/<code-name>/verification.md`

This is the scenario and validation document.

It should include:

- core scenarios that must remain true
- validation commands
- runtime/manual checklist when end-to-end behavior matters
- known coverage gaps or remaining risks

Prefer scenario language over test-file inventory language.

### `research/<release>/<code-name>.md`

This is the release-specific baseline and adaptation record.

It should include:

- upstream baseline tag and commit
- current fork target and research scope
- gap summary between upstream and fork behavior
- adaptation notes
- risky integration points or conflict-prone files
- release-specific verification notes

This document exists to prevent future ports from re-opening already-made decisions.

### `audits/<code-name>.md`

Use this when the main goal is findings-first review rather than feature authoring.

It should lead with:

- findings ordered by severity
- affected files/surfaces
- open questions or risks
- only then summary/context

## Writing rules

- Prefer contract-first wording over implementation-first wording.
- Separate confirmed facts from assumptions.
- Prefer current branch/staged truth over historical commit messages.
- Document intentional divergence explicitly.
- Keep one clear source of truth for each behavior.
- Use concise headings and compact sections.
- Mention file paths only when they disambiguate a non-obvious claim.
- When the code is in flux, describe the contract that the current target branch is actually
  implementing, not a hypothetical ideal.

## Anti-patterns

Avoid these failures:

- treating an old fork doc as current truth without re-checking the code
- documenting an RPC/API surface because it existed in another branch
- hiding fallback or compatibility behavior that affects recovery or replay
- mixing feature contract, audit notes, and release research into one undifferentiated file
- writing docs that are only a diff summary and never state the intended behavior
- claiming “tests passed” as the whole evidence set when the fork contract changed

## Practical review checklist

Before finalizing fork docs, confirm:

- the code name is canonical and reused consistently
- `features/`, `projects/`, and `research/` all agree on the same behavior
- the baseline release is recorded for the current adaptation
- intentional divergence from upstream is explicit
- the docs reflect the current target branch and staged state
- user/developer docs outside `docs/fork/*` were updated when the contract changed
- no outdated claims from older branches survived by copy-paste

## Reference examples

Current examples in this repo:

- Feature package: [features/thread-note.md](features/thread-note.md)
- Implementation dossier: [projects/thread-note/README.md](projects/thread-note/README.md)
- Release research: [research/0.118.0/thread-note.md](research/0.118.0/thread-note.md)
- Deep release research example: [research/0.118.0/cwd-directory.md](research/0.118.0/cwd-directory.md)

Use these as patterns, not as templates to copy verbatim. Always adapt them to the real contract of
the feature you are documenting.
