# Release audit report: 0.98 (rust-v0.98.0 vs fork/colab-agents)

> Owner: <team/owner> | Scope: fork/colab-agents upgrade audit | Audience: devs
> Status: draft | Last reviewed: 2026-02-06 | Related: `docs/fork/native-first-audit.md`

## Executive summary

Аудит диффа `rust-v0.98.0` → `fork/colab-agents` показал **390** file-level отличий, из них значимые риски
сконцентрированы в:

- **core**: wire API (Responses vs Chat) и security-конфиги (exec-policy trust, requirements enforcement).
- **tui**: деградация upstream streaming chunking/commit tick.
- **protocol**: несколько breaking изменений (remote skills RPC, TS schema optional/nullable, on-wire `ModeKind::Custom`,
  sandbox read-only поддиректории).
- **linux-sandbox/vendor**: удаление vendored bubblewrap без явного замещающего механизма.

Карточки с деталями и ссылками на `.patch`: `docs/fork/native-first-audit.md`.

Примечание по полноте: изначально текущий проход карточек **не покрывал** ряд крупных подсекций диффа (см. Coverage gaps в
`docs/fork/native-first-audit.md`). На текущий момент coverage gaps закрыты дополнительными карточками (codex-api,
app-server, windows-sandbox-rs, exec/mcp-server, state/otel/secrets).

## Progress

Обновление после D13-стабилизации (commit `e692cb2e9`, 2026-02-06):

- ✅ **D13 / NF-APP-SERVER-001/002/003/004** — реализовано: возвращены `thread/compact/start`, `skills/remote/read|write`
  (disabled-by-policy), `model/list` содержит optional `upgrade`, а `ModeKind::Custom` не появляется on-wire.
- ✅ **D6 / NF-PROTO-004** — реализовано: `ModeKind::Custom` — внутренний sentinel и никогда не сериализуется on-wire
  (при этом сериализация/rollout не падают).
- ✅ **D1 / NF-CORE-001 + NF-CODEX-API-001 + NF-MCP-001** — ✅ DONE: восстановлена upstream семантика Responses-only
  (удалён Chat wire; `wire_api="chat"` invalid); `codex-api` и MCP test-suite переведены на `/v1/responses` SSE.
  Коммиты: `2965b0bca` (fix(wire): restore upstream Responses-only semantics), `901e3488c`
  (test(mcp-server): migrate suite to Responses SSE).
- Прогнаны проверки:
  - `cd codex-rs && cargo test -p codex-app-server-protocol`
  - `cd codex-rs && cargo test -p codex-app-server`
  - `cd codex-rs && cargo test -p codex-core`
  - `cd codex-rs && cargo test -p codex-api`
  - `cd codex-rs && cargo test -p codex-mcp-server`

## Baseline and scope

- Base: `rust-v0.98.0` (commit `82464689c…`)
- Fork baseline: `fork/colab-agents` (commit `33aa00c7f…`)
- Diff artifacts: `docs/fork/release-audit/0.98/DIFF_BASELINE.md`
- Diff artifacts: `docs/fork/release-audit/0.98/DIFF_INDEX.md`
- Diff artifacts: `docs/fork/release-audit/0.98/diff/*.patch`
- Native-first cards: `docs/fork/native-first-audit.md`

Triage (по `DIFF_INDEX.md`):

- core: 171
- tui: 65
- protocol: 8
- linux-sandbox: 3
- ci: 7
- docs: 15
- vendor: 50
- generated: 71

Generated artifacts verification (do not review line-by-line):

- `app-server-protocol/schema/**`: `just write-app-server-schema` (diff должен совпасть).
- `core/config.schema.json`: `just write-config-schema`.
- `Cargo.lock`: обновляется только через Cargo; подтверждать сборкой/тестами.

## Key risks and regressions

### P0 (blockers)

- **NF-CORE-001**: reintroduce `WireApi::Chat` и смена дефолта wire API на Chat. ✅ DONE (`2965b0bca`).
- **NF-CORE-007 / NF-LS-001**: exec-policy читает `.codex/rules` из trust-disabled project layer. Риск security bypass.
- **NF-CORE-008**: ослабление requirements constraints (убран source-tracking + убран fallback дефолтов на required).
  Риск нарушения security/compliance инвариантов.
- **NF-TUI-003**: удалён upstream streaming chunking/commit_tick orchestration. Риск заметной деградации UX на длинных стримах.
- **NF-PROTO-001**: удалены remote skills RPC/events. Breaking для клиентов.
- **NF-PROTO-003**: TS schema optional/nullable правила. Потенциальный compile-time break для TS-клиентов.
- **NF-PROTO-004**: on-wire `ModeKind::Custom`. Риск несовместимости с внешними клиентами/персистентностью.
  ✅ DONE (`e692cb2e9`).
- **NF-PROTO-007**: `.agents` убран из sandbox read-only subpaths. Потенциальная security-регрессия.
- **NF-META-005**: удаление `codex-rs/vendor/bubblewrap/**` без доказанного замещения (см. также NF-LS-003).
- **NF-APP-SERVER-001/002/003/004**: несколько breaking изменений JSON-RPC контрактов (compact/start, remote skills,
  model/list upgrade, on-wire `ModeKind::Custom`). ✅ DONE (`e692cb2e9`).
- **NF-WIN-SB-001/002**: существенные изменения security boundary/токенов в Windows sandbox.
- **NF-STATE-001/002**: миграционные/контрактные риски state DB (фиксированное имя `state.sqlite`, семантика dynamic tools).
- **NF-EXEC-001 / NF-MCP-001**: семантика `TurnAborted` и тестовый wire API (Responses vs Chat) в смежных подсистемах.
  NF-MCP-001 ✅ DONE (`901e3488c`); NF-EXEC-001 pending.

### P1 (should-do)

- **NF-CORE-002**: agent registry (fork-only) — держать, но минимизировать точки интеграции и долгосрочный churn.
- **NF-CORE-003**: `spawn_agent` расширения (agent_type/name + overrides) — держать, но следить за совместимостью протокола.
- **NF-CORE-004**: tool allow/deny policy floor — держать как security-фичу, но усилить тестами/доками.
- **NF-CORE-006**: удаление remote skills downloader — ок для fork, но подтвердить продуктовую потребность.
- **NF-CORE-009**: backfill `experimental_supported_tools` — временный костыль, стремиться к upstream-native metadata.
- **NF-TUI-001**: multi-agent overlays (Ctrl+N) — fork-ключевая фича, риск регрессий alt-screen/хоткеев.
- **NF-TUI-002**: collaboration modes default (инертный `Custom` vs upstream default). Нужна продуктовая фиксация.
- **NF-TUI-006**: OSS provider “ollama-chat” UX — fork-only, проверить корректность deprecation notice.
- **NF-LS-003**: bwrap FFI pipeline/фича `use_bwrap_sandbox` после удаления vendoring — риск “footgun”.
- **NF-PROTO-002/005/006**: протокольные изменения средней важности (compact/start, personality none, agent info).

### P2 (nice-to-have)

- **NF-CORE-005/010/011**: list/read agents tools, MCP readiness workaround, `CODEX_THREAD_ID` env.
- **NF-TUI-004/005/007/008/009/010**: fps clamp, request_user_input hotkeys, context-left coloring, FN branding,
  debug_config provenance, runtime metrics label.
- **NF-PROTO-008**: prompt/doc форматирование.
- **NF-META-001/004**: issue templates и объём fork-доков.

## Recommendations (native-first / defork)

План действий (native-first) — предложенный порядок:

### P0 plan

1. **Выбрать канон wire API (Responses vs Chat)** и перестать менять upstream-дефолты без необходимости.
   - Target: `WireApi::Responses` как дефолт (upstream), Chat — только как явный opt-in для OSS провайдеров (если нужен).
   - Cards: NF-CORE-001, NF-TUI-006.
2. **Закрыть security-риск trust-disabled `.codex/rules`.**
   - Target: disabled/untrusted layers не могут ослаблять policy; минимум deny-only из disabled слоя.
   - Cards: NF-CORE-007, NF-LS-001.
3. **Восстановить requirements enforcement (и provenance).**
   - Target: constraints должны применяться и к дефолтам; provenance нужен для диагностики (TUI/debug_config).
   - Cards: NF-CORE-008, NF-TUI-009.
4. **Вернуть upstream streaming chunking/commit_tick.**
   - Target: убрать fork UX-регрессии, снизить риск снапшот-чёрна в будущем апгрейде.
   - Cards: NF-TUI-003.
5. **Протокол: зафиксировать breaking изменения и совместимость.**
   - Target: определить стратегию миграции клиентов для remote skills; не сериализовать `ModeKind::Custom` наружу;
     стабилизировать TS nullable/optional правило.
   - Cards: NF-PROTO-001/003/004.
6. **Sandbox read-only subpaths:** решить судьбу `.agents`.
   - Target: либо вернуть `.agents` в read-only, либо перенести форк-артефакты под `.codex/`.
   - Cards: NF-PROTO-007.
7. **Bubblewrap supply chain:** подтвердить замещение vendored bubblewrap или вернуть vendoring.
   - Target: bwrap path должен быть либо полностью работоспособным, либо явно недоступным/удалённым без “падающих” путей.
   - Cards: NF-META-005, NF-LS-003.

### P1 plan

- Стабилизировать fork-only пласты (agent registry, tool policy, multi-agent overlays) и минимизировать merge-conflicts.
  - Cards: NF-CORE-002/003/004, NF-TUI-001/002.
- Решить “инертный Custom” для collaboration modes и синхронизировать это с протоколом (избегать on-wire `custom`).
  - Cards: NF-TUI-002, NF-PROTO-004.
- Подтвердить статус bwrap/landlock: какой путь реально поддерживаем на Linux и как он тестируется в CI/ручном прогоне.
  - Cards: NF-LS-003, NF-META-005.

### P2 plan

- Косметика/UX и docs cleanup по мере касания файлов.
  - Cards: NF-TUI-004/005/007/008/010, NF-META-001/004, NF-PROTO-008, NF-CORE-010/011.

## Manual verification plan (TUI)

Цель: минимальный ручной прогон ключевых сценариев до/после изменений, особенно вокруг multi-agent UX и naming sync.

### Setup

1. Build fork binary (локально).
2. Launch TUI in a clean workspace.
3. Ensure config/agents registry seeding runs (если применимо).

### Smoke flows

1. Basic chat: отправка сообщений, отмена/прерывание, history rendering.
2. Tool calls: shell, apply_patch (если доступно), file writes/reads.
3. Approval UX: request/deny/approve, history отображение решений.
4. Streaming stress: длинный ответ с быстрыми дельтами (и параллельно plan-stream) — оценить лаг/“залипание” backlog.
   - Card: NF-TUI-003.

### Multi-agent / collab flows

1. Toggle/enter overlays (Ctrl+N / Agents summary/details) при включённых флагах.
2. `spawn_agent`: успешный spawn, обработка ошибок (неизвестный agent/model), закрытие потоков.
3. Shutdown orchestration: корректное закрытие потомков, отсутствие “подвисших” overlay/alt-screen.
4. Collaboration modes: после включения фичи проверить, что выбор режима/маски понятен и фактически влияет на поведение.
   - Card: NF-TUI-002.

### Skills / registry flows

1. List agents: проверить перечень профилей, варианты `agent_names`, отображение `reasoning_effort`.
2. Local skills/custom prompts: list/read/write, обработка ошибок валидации, отображение в UI.

### After naming sync (повторно)

После синхронизации нейминга/label-ов (и/или hotkeys) повторить разделы:

- Smoke flows
- Multi-agent / collab flows

Фокус: корректность названий/подсказок в UI, стабильность hotkeys, отсутствие регрессий в overlay lifecycle.

## Appendix

- Карточки: `docs/fork/native-first-audit.md`
- Индекс diff: `docs/fork/release-audit/0.98/DIFF_INDEX.md`
