# Upstream main commits (not yet in fork/colab-agents)

> Owner: <team/owner> | Scope: upstream/main delta | Audience: devs
> Status: active | Last reviewed: 2026-02-03 | Related: docs/fork/colab-agents.md

## Context

This document captures the set of commits that exist on `upstream/main` but are not yet
present on `fork/colab-agents` as of 2026-02-03. It is intended to guide rebase/merge
planning and conflict triage. Cherry-picked upstream commits are tracked separately
below and excluded from the main list.

Source command:

```
git log --reverse --oneline upstream/main ^fork/colab-agents
```

## Already integrated (cherry-picked)

1. 3dd9a37e0 Improve plan mode interaction rules (#10329) → `44bd6a128`

## Commit list (chronological)

1. d3514bbdd Bump thread updated_at on unarchive to refresh sidebar ordering (#10280)
2. 5fb46187b fix: System skills marker includes nested folders recursively (#10350)
3. 8b95d3e08 fix(rules) Limit rules listed in conversation (#10351)
4. 03fcd12e7 Do not append items on override turn context (#10354)
5. 6c22360bc fix(core) Deduplicate prefix_rules before appending (#10309)
6. a90ff831e chore(core) gpt-5.2-codex personality template (#10373)
7. 08a5ad95a fix(personality) prompt patch (#10375)
8. 974355cfd feat: vendor app-server protocol schema fixtures (#10371)
9. 1644cbfc6 Session picker shows thread_name if set (#10340)
10. 9513f18bf chore: collab experimental (#10381)
11. 3cc9122ee feat: experimental flags (#10231)
12. 4971e96a9 nit: shell snapshot retention to 3 days (#10382)
13. e9a774e7a fix: thread listing (#10383)
14. 4f1cfaf89 fix: Rfc3339 casting (#10386)
15. d1e71cd20 feat: add MCP protocol types and rmcp adapters (#10356)
16. 3392c5af2 Nicer highlighting of slash commands, /plan accepts prompt args and pasted images (#10269)
17. 9d976962e Add credits tooltip (#10274)
18. 0b460eda3 chore: ignore synthetic messages (#10394)
19. 34c0534f6 feat: drop sqlx logging (#10398)
20. 74327fa59 Select experimental features with space (#10281)
21. 059d386f0 feat: add `--experimental` to `generate-ts` (#10402)
22. f50c8b2f8 fix: unsafe auto-approval of git commands (#10258)
23. 0f15ed432 Updated labeler workflow prompt to include "app" label (#10411)
24. a5066bef78 emit a separate metric when the user cancels UAT during elevated setup (#10399)
25. 98debeda8 chore(tui) /personalities tip (#10377)
26. fb2df99cf [feat] persist thread_dynamic_tools in db (#10252)
27. e24058b7a feat: Read personal skills from .agents/skills (#10437)
28. 019d89ff8 make codex better at git (#10145)
29. d02db8b43 Add `codex app` macOS launcher (#10418)
30. 1096d6453 Fix plan implementation prompt reappearing after /agent thread switch (#10447)
31. 8f5edddf7 TUI: Render request_user_input results in history and simplify interrupt handling (#10064)
32. 66447d5d2 feat: replace custom mcp-types crate with equivalents from rmcp (#10349)
33. 8dd41e229 Fixed sandbox mode inconsistency if untrusted is selected (#10415)
34. 97ff09010 Hide short worked-for label in final separator (#10452)
35. 891ed8740 chore: remove deprecated mcp-types crate (#10357)
36. 0999fd82b app tool tip (#10454)
37. fc0537434 chore: add phase to message responseitem (#10455)
38. b8addcddb Require models refresh on cli version mismatch (#10414)
39. 7e07ec8f7 [Codex][CLI] Gate image inputs by model modalities (#10271)
40. cbfd2a37c Trim compaction input (#10374)
41. 8b280367b Updated bug and feature templates (#10453)
42. bf87468c2 Restore status after preamble (#10465)
43. 59707da85 fix: clarify deprecation message for features.web_search (#10406)

## Likely conflict zones for fork/colab-agents

These areas overlap with fork changes or high-churn files and typically require manual review:

- `codex-rs/core/src/codex.rs`
- `codex-rs/core/src/features.rs`
- `codex-rs/core/src/tools/spec.rs`
- `codex-rs/core/src/tools/handlers/collab.rs`
- `codex-rs/tui/src/chatwidget.rs`
- `codex-rs/tui/src/history_cell.rs`
- `codex-rs/app-server-protocol/schema/*` and related schema fixtures
- `codex-rs/Cargo.lock`
