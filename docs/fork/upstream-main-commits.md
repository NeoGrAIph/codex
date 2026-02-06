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
34. 8dd41e229 Fixed sandbox mode inconsistency if untrusted is selected (#10415) — status: integrated (cherry-pick `750ea03af`); notes: reload config after any explicit trust decision so `/status` reflects untrusted selection
35. 97ff09010 Hide short worked-for label in final separator (#10452) — status: integrated (cherry-pick `0e94a7c3e`); notes: hide “Worked for” under 60s, add tests for <60s and >=61s
36. 891ed8740 chore: remove deprecated mcp-types crate (#10357) — status: integrated (cherry-pick `46b9517ea`); notes: удалён legacy `mcp-types` крейт и схемы после миграции на `rmcp`
37. 0999fd82b app tool tip (#10454) — status: integrated (cherry-pick `f884674f6`); notes: tooltip теперь предлагает `codex app`
38. fc0537434 chore: add phase to message responseitem (#10455) — status: integrated (cherry-pick `4ec9b6452`); notes: добавлен `MessagePhase` и опциональный `phase` в `ResponseItem::Message`, обновлены schema/fixtures и тесты
39. b8addcddb Require models refresh on cli version mismatch (#10414) — status: integrated (cherry-pick `111af2e47`); notes: добавлен `client_version` в models cache + refresh при несовпадении версии CLI
40. 7e07ec8f7 [Codex][CLI] Gate image inputs by model modalities (#10271) — status: integrated (cherry-pick `a806118dd`); notes: input_modalities в моделях + TUI блокирует/предупреждает для неподдерживаемых изображений
41. cbfd2a37c Trim compaction input (#10374) — status: integrated (cherry-pick `b2ae7be71`); notes: учитываем trailing tool output в оценке контекста и режем воспроизводимые хвосты перед remote compaction
42. 8b280367b Updated bug and feature templates (#10453) — status: integrated (cherry-pick `28f19969e`); notes: обновлены issue templates (версия/вариант продукта)
43. bf87468c2 Restore status after preamble (#10465) — status: integrated (cherry-pick `486730989`); notes: при exec begin восстанавливаем status indicator после preamble
44. 59707da85 fix: clarify deprecation message for features.web_search (#10406) — status: integrated (cherry-pick `506086635`); notes: уточнён текст о `web_search` вне секции `[features]`
45. 1dc06b6ff fix: ensure resume args precede image args (#10709) — status: integrated (adopt; cherry-pick `955ed3aa8`); notes: порядок `resume`/`--image` восстановлен, тест SDK обновлён
46. fe8b474ac fix(core,app-server) resume with different model (#10719) — status: integrated (adapt; cherry-pick `e69f69886` + fork fix в `core/src/codex.rs`); notes: warning + одноразовые model‑instructions при resume
47. e48297826 fix(core) switching model appends model instructions (#10651) — status: integrated (adapt; cherry-pick `d9e361da1` + fork fix в `core/src/codex.rs`); notes: добавление developer‑инструкций при смене модели
48. dc7007bea Fix remote compaction estimator/payload instruction small mismatch (#10692) — status: integrated (adapt; cherry-pick `bf45fa8cd` + fork fix в `core/src/context_manager/history.rs`); notes: базовые инструкции унифицированы для оценки и payload
49. 1b153a3d4 Cloud Requirements: take precedence over MDM (#10633) — status: integrated (adopt; cherry-pick `3a365a707`); notes: порядок слоёв cloud→MDM
50. 1f47e08d6 Cloud Requirements: increase timeout and retries (#10631) — status: integrated (adopt; cherry-pick `d55e080d5`); notes: timeout 15s + ретраи с backoff
51. 1e1146cd2 Reload cloud requirements after user login (#10725) — status: integrated (adopt; cherry-pick `42df1d8cd`); notes: reload после логина/обновления резидентности
52. 95269ce88 Increase cloud req timeout (#10659) — status: skip (empty; уже покрыт 1f47e08d6)
53. acdbd8edc [apps] Cache MCP actions from apps. (#10662) — status: integrated (adapt; cherry-pick `778fdf685` + fork guard для non‑blocking startup); notes: кэш инструментов для codex_apps_mcp
54. 4ed8d74aa fix: ensure status indicator present earlier in exec path (#10700) — status: integrated (adopt; cherry-pick `38456a1b9`); notes: восстановление статуса + новые снапшоты TUI
55. 71e63f8d1 fix: flaky test (#10644) — status: integrated (adopt; cherry-pick `12f4a397d`); notes: стабилизация unit‑теста
56. 7c6d21a41 Fix test_shell_command_interruption flake (#10649) — status: integrated (adopt; cherry-pick `bb1fe3bdf`); notes: стабилизация app‑server теста
57. 73f32840c chore(core) personality migration tests (#10650) — status: integrated (adopt; cherry-pick `01f131e1c`); notes: расширены тесты миграции personality
58. 224c9f768 chore(app-server): document experimental API opt-in (#10667) — status: integrated (adopt; cherry-pick `bf876dac5`); notes: документация про experimental opt‑in
59. 82464689c ## New Features - Steer mode is now stable and enabled by default, so `Enter` sends immediately during running tasks while `Tab` explicitly queues follow-up input. (#10690) — status: integrated (adopt; cherry-pick `18171a255`); notes: bump версии workspace до 0.98.0
60. cddfd1e67 feat(core): add configurable log_dir (#10678) — status: integrated (adapt; manual `022816ce2`); notes: log_dir + относительные пути в overrides, восстановлен doc
61. d452bb3ae Add /debug-config slash command (#10642) — status: integrated (adapt; cherry-pick `959edc1a0` + fork fix в `tui/src/debug_config.rs`); notes: /debug-config в TUI, источники для constraint-полей показываются как `<unspecified>` в fork
62. d589ee05b fix(tui): list selection view should maintain horizontal scroll offset on selection (#10693) — status: integrated (adopt; cherry-pick `d122907a8`); notes: фиксы scroll offset + снапшоты list selection
63. d876f3b94 fix(tui): restore working shimmer after preamble output (#10701) — status: integrated (adapt; cherry-pick `777ae7313` + fork fix в `tui/src/chatwidget.rs`); notes: восстановление status indicator при idle, адаптация под текущий commit-tick код
64. cd5f49a61 chore(core): Steer mode stable by default in config (#10691) — status: integrated (adopt; cherry-pick `d606a606a`); notes: steer default для новых конфигов
65. a05aadfa1 chore(config) Default Personality Pragmatic (#10705) — status: integrated (adopt; cherry-pick `b599ce9e1`); notes: дефолтная personality = pragmatic + новые тесты
66. 5ea107a08 feat(app-server, core): allow text + image content items for dynamic tool outputs (#10567) — status: integrated (adapt; cherry-pick `681348fc2` + fork fixes в `codex-api/src/requests/chat.rs`, `core/src/tools/handlers/agents.rs`); notes: FunctionCallOutputPayload.body + контент-айтемы, schema/fixtures перегенерированы
67. 282f42c0c Add option to approve and remember MCP/Apps tool usage (#10584) — status: integrated (adopt; cherry-pick `5df704351`); notes: можно подтвердить и запомнить MCP/apps tool usage
68. e9335374b feat: add phase 1 mem client (#10629) — status: integrated (adapt; cherry-pick `aadb57231` + fork fix в `codex-api/src/endpoint/memories.rs`); notes: mem client построен на provider+transport без EndpointSession
69. 4922b3e57 feat: add phase 1 mem db (#10634) — status: integrated (adopt; cherry-pick `4bad02f1d`); notes: thread_memory таблица + runtime API + тесты
70. 7f2035761 Stop client from being state carrier (#10595) — status: integrated (adapt; cherry-pick `5feb90704`); notes: TurnContext стал носителем turn-конфига/контекста вместо ModelClient; минимальные конфликт‑резолвы в core
71. 0e8d359da Session-level model client (#10664) — status: integrated (adapt; cherry-pick `24ea26c00`); notes: session-level ModelClient + per-turn ModelClientSession, удалён transport_manager; fork-фиксы: восстановили поддержку WireApi::Chat и обновили handlers под TurnContext без client
72. ae4de43cc feat(linux-sandbox): add bwrap support (#9938) — status: integrated (adapt; cherry-pick `1ccdbae25`); notes: bubblewrap-based pipeline (bwrap) + preflight `/proc` mount fallback; synced `codex-rs/linux-sandbox/src/lib.rs` with upstream module layout (drop legacy `mounts`)
73. 7bcc55232 Added support for live updates to skills (#10478) — status: integrated (adopt; cherry-pick `5eeb5848e`); notes: file watcher + SkillsUpdateAvailable event; updates skill manager + thread_manager
74. 7a253076f Persist pending input user events (#10656) — status: integrated (adopt; cherry-pick `037e5643f`); notes: pending input events are persisted as user events so history/UI can reflect queued user messages
75. 41b4962b0 Sync collaboration mode naming across Default prompt, tools, and TUI (#10666) — status: integrated (adapt; cherry-pick `1a16490f7`); notes: added `ModeKind::Default` (alias `code`) while preserving fork `ModeKind::Custom`; updated builtin presets + request_user_input messaging + TUI labels

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
