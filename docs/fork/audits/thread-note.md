# Thread Note Documentation Audit

## Summary

Audit scope: the fork-specific `thread-note` documentation package under `docs/fork/`.

Audited surfaces:

- feature contract
- implementation dossier
- release research note
- verification matrix

The package is now in the right namespace and internally linked, but it still has contract-quality
gaps against the fork documentation rules in `AGENTS.md`.

## Findings

### P1: Feature contract is missing required fork-contract sections

Document: `docs/fork/features/thread-note.md`

Problem:

- no explicit feature passport fields beyond `Goal`
- no status field
- no scope-in / scope-out section
- no dedicated integration / compatibility notes section
- no doc changelog section

Why it matters:

The feature doc is supposed to be the canonical fork contract. Right now important boundaries are
present, but they are embedded across prose sections instead of being stated as contract fields.
That makes release review and future ports harder because implementers have to infer scope from
narrative text.

Remediation:

- add a short passport block with code name, status, goal, scope in, scope out
- add explicit compatibility notes
- add a verification matrix section
- add a concise dated doc changelog

### P2: Verification doc is scenario-rich but not yet a real verification matrix

Document: `docs/fork/projects/thread-note/verification.md`

Problem:

- validation commands are listed, but not mapped to the exact surfaces they validate
- runtime checklist contains several internal checks such as “initial model context does not contain
  note metadata” without saying how that should be observed in practice

Why it matters:

The current document is useful for humans who already know the implementation, but weaker as
release evidence. A matrix should tell a reviewer which command or scenario proves which contract
statement, and how to observe pass/fail conditions.

Remediation:

- convert the command list into a table `command -> validated surface -> expected evidence`
- for runtime-only checks, add the observation method
- separate automated checks from manual runtime validation

### P2: Research note misses explicit risky integration points / source-of-truth map

Document: `docs/fork/research/0.114.0/thread-note.md`

Problem:

The research note has baseline, gap summary, adaptation, and verification, but it does not name
the conflict-prone source-of-truth files or risky integration points that future rebases should
watch.

Why it matters:

This is one of the main purposes of the research package in the fork workflow. Without those
anchors, a future porter still has to rediscover where the contract is most likely to drift.

Remediation:

- add a short “Risky integration points” section
- name the canonical code/doc surfaces that own the thread-note contract
- call out rebase-sensitive paths explicitly

### P3: Terminology is internally consistent, but contract boundaries are spread across documents

Documents:

- `docs/fork/features/thread-note.md`
- `docs/fork/projects/thread-note/design.md`
- `docs/fork/research/0.114.0/thread-note.md`

Problem:

Important statements such as “metadata-only”, “no model-visible note”, “restart-safe via
thread_note_index.jsonl”, and “no app-server note surface” are repeated across multiple docs, but
each document currently carries a slightly different subset of the boundary story.

Why it matters:

There is no direct contradiction now, but the package is vulnerable to drift because the reader has
to piece together the full contract from multiple files.

Remediation:

- keep the feature doc as the single contract summary
- keep design.md implementation-only
- keep research.md release-delta-only
- reduce repeated boundary prose in project/research docs and replace it with references back to
  the feature contract where appropriate

## Remediation Order

1. Upgrade `docs/fork/features/thread-note.md` to the full fork-contract template.
2. Rewrite `docs/fork/projects/thread-note/verification.md` into a true verification matrix.
3. Add risky integration points and source-of-truth notes to `docs/fork/research/0.114.0/thread-note.md`.
4. Trim duplicated boundary prose from `design.md` and `research.md` after the feature contract is strengthened.
