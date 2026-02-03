# Upstream main commits (not yet in fork/colab-agents)

> Owner: <team/owner> | Scope: upstream/main delta | Audience: devs
> Status: active | Last reviewed: 2026-02-03 | Related: docs/fork/colab-agents.md

## Context

This document captures the set of commits that exist on `upstream/main` but are not yet
present on `fork/colab-agents` as of 2026-02-03. It is intended to guide rebase/merge
planning and conflict triage. Each entry includes an integration status note.

Source command:

```
git log --reverse --oneline upstream/main ^fork/colab-agents
```

## Commit list (chronological)

1. 3dd9a37e0 Improve plan mode interaction rules (#10329) — status: integrated (cherry-pick `44bd6a128`)
2. d3514bbdd Bump thread updated_at on unarchive to refresh sidebar ordering (#10280) — status: integrated (cherry-pick `60abba13d`)
3. 5fb46187b fix: System skills marker includes nested folders recursively (#10350) — status: integrated (cherry-pick `b46295b5d`)
4. 8b95d3e08 fix(rules) Limit rules listed in conversation (#10351) — status: integrated (cherry-pick `ff8bccafa`, чистый cherry-pick); notes: ограничили вывод allow‑prefixes (сортировка по “широте”, лимит по количеству и байтам) и добавили тесты, чтобы список правил не раздувал контекст
5. 03fcd12e7 Do not append items on override turn context (#10354) — status: integrated (manual adaptation `4f647d392`); doc: `docs/fork/commit-03fcd12e7.md`; notes: перешли на полный `CollaborationMode` в `TurnContext`, override не пишет update‑items до следующего user‑turn, тесты обновлены по upstream‑контракту
6. 6c22360bc fix(core) Deduplicate prefix_rules before appending (#10309) — status: integrated (manual adaptation `cee4e2915`); notes: дедуп `prefix_rule` перед append, добавлен тест
7. a90ff831e chore(core) gpt-5.2-codex personality template (#10373) — status: integrated (cherry-pick `a90ff831e`)
8. 08a5ad95a fix(personality) prompt patch (#10375) — status: integrated (cherry-pick `08a5ad95a`)
9. 974355cfd feat: vendor app-server protocol schema fixtures (#10371) — status: integrated (manual adaptation `40ceb947c`); notes: добавлены schema fixtures, тест на синхронизацию, `just write-app-server-schema`, fixtures перегенерированы под fork
10. 1644cbfc6 Session picker shows thread_name if set (#10340) — status: integrated (cherry-pick `1644cbfc6`)
11. 9513f18bf chore: collab experimental (#10381) — status: integrated (cherry-pick `9513f18bf`)
12. 3cc9122ee feat: experimental flags (#10231) — status: integrated (manual adaptation `cd08d9932`); notes: experimental_api capability + gating, schema filtering, новые fixtures
13. 4971e96a9 nit: shell snapshot retention to 3 days (#10382) — status: integrated (cherry-pick `4971e96a9`)
14. e9a774e7a fix: thread listing (#10383) — status: integrated (cherry-pick `e9a774e7a`)
15. 4f1cfaf89 fix: Rfc3339 casting (#10386) — status: integrated (cherry-pick `4f1cfaf89`)
16. d1e71cd20 feat: add MCP protocol types and rmcp adapters (#10356) — status: integrated (cherry-pick `d1e71cd20`)
17. 3392c5af2 Nicer highlighting of slash commands, /plan accepts prompt args and pasted images (#10269) — status: integrated (cherry-pick `3392c5af2`)
18. 9d976962e Add credits tooltip (#10274) — status: integrated (manual adaptation `aa1635241`)
19. 0b460eda3 chore: ignore synthetic messages (#10394) — status: integrated (manual adaptation `bba09767a`); notes: исключили synthetic user‑messages (AGENTS instructions + session prefix) без логирования содержимого
20. 34c0534f6 feat: drop sqlx logging (#10398) — status: integrated (manual adaptation `9f95bc334`)
21. 74327fa59 Select experimental features with space (#10281) — status: integrated (manual adaptation `842af0c3c`)
22. 059d386f0 feat: add `--experimental` to `generate-ts` (#10402) — status: integrated (manual adaptation `b6d2fab1f`)
23. f50c8b2f8 fix: unsafe auto-approval of git commands (#10258) — status: integrated (manual adaptation `298b1552b`)
24. 0f15ed432 Updated labeler workflow prompt to include "app" label (#10411) — status: integrated (manual adaptation `de3e2ea50`)
25. a5066bef78 emit a separate metric when the user cancels UAT during elevated setup (#10399) — status: integrated (cherry-pick `a5066bef78`)
26. 98debeda8 chore(tui) /personalities tip (#10377) — status: integrated (cherry-pick `98debeda8`)
27. fb2df99cf [feat] persist thread_dynamic_tools in db (#10252) — status: integrated (cherry-pick `fb2df99cf`)
28. e24058b7a feat: Read personal skills from .agents/skills (#10437) — status: integrated (cherry-pick `e24058b7a`)
29. 019d89ff8 make codex better at git (#10145) — status: integrated (cherry-pick `695cb113a`); notes: добавлен `x-codex-turn-metadata` (git root + remotes + HEAD), кэш заголовка на сессию, обновлены тесты
30. d02db8b43 Add `codex app` macOS launcher (#10418) — status: integrated (cherry-pick `106e18ab3`); notes: добавлен macOS launcher, автоскачивание DMG, tooltip `codex app`
31. 1096d6453 Fix plan implementation prompt reappearing after /agent thread switch (#10447) — status: integrated (cherry-pick `620df12b9`); notes: `saw_plan_item_this_turn` сбрасывается после live TurnComplete, чтобы prompt не дублировался; добавлен тест на replay→live
32. 8f5edddf7 TUI: Render request_user_input results in history and simplify interrupt handling (#10064) — status: integrated (cherry-pick `45c72845e`); notes: добавлен history‑cell с Q/A, interrupt теперь просто `Op::Interrupt` без частичной отправки, тесты обновлены
33. 66447d5d2 feat: replace custom mcp-types crate with equivalents from rmcp (#10349) — status: integrated (cherry-pick `2745ebdf7`); notes: rmcp типы + schema/fixtures обновлены; в `mcp_process` оставили `serverInfo.version` = `env!("CARGO_PKG_VERSION")` чтобы совпасть с фактическим ответом сервера
34. 8dd41e229 Fixed sandbox mode inconsistency if untrusted is selected (#10415) — status: pending
35. 97ff09010 Hide short worked-for label in final separator (#10452) — status: pending
36. 891ed8740 chore: remove deprecated mcp-types crate (#10357) — status: pending
37. 0999fd82b app tool tip (#10454) — status: pending
38. fc0537434 chore: add phase to message responseitem (#10455) — status: pending
39. b8addcddb Require models refresh on cli version mismatch (#10414) — status: pending
40. 7e07ec8f7 [Codex][CLI] Gate image inputs by model modalities (#10271) — status: pending
41. cbfd2a37c Trim compaction input (#10374) — status: pending
42. 8b280367b Updated bug and feature templates (#10453) — status: pending
43. bf87468c2 Restore status after preamble (#10465) — status: pending
44. 59707da85 fix: clarify deprecation message for features.web_search (#10406) — status: pending

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
