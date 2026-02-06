# Decisions: Release Audit 0.98 (Dx log)

> Owner: <team/owner> | Scope: fork/colab-agents upgrade decisions | Audience: devs
> Status: draft | Last reviewed: 2026-02-06 | Related: `docs/fork/native-first-audit.md`, `docs/fork/release-audit/0.98/REPORT.md`

Этот документ фиксирует решения по пунктам `D1..Dn` (что решили и почему), чтобы на их основе строить корректный план
последующих работ.

## D1 (зафиксированное решение) Upstream-first канон wire API

**Контекст**

- Цель форка: максимально следовать upstream (Codex) как источнику истины ("канон = upstream").
- В апстриме `rust-v0.98.0` каноничный wire это **Responses API** (`/v1/responses`). Chat Completions (`/v1/chat/completions`)
  как основной путь в upstream не используется и не должен становиться дефолтом в форке.

**Решение**

1. **Канон/дефолт в форке: `WireApi::Responses` и `/v1/responses`**, полностью совпадая с upstream.
2. **Chat Completions (`/v1/chat/completions`) не считаем критичной функциональностью** и **не поддерживаем как обязательный
   путь**, пока не проведен отдельный аудит потребности.
3. Любые fork-изменения, которые:
- делают "default = Chat"
- перепрошивают тестовую/интеграционную обвязку (например MCP) на chat/completions
- добавляют/расширяют Chat wire как основной пользовательский путь
  считаем **нежелательным дрейфом** и кандидатом на откат/дефолтный disable.

**Почему так (плюсы/минусы)**

- Плюсы: минимальный дрейф от upstream; меньше тестовой матрицы; меньше риска subtle-bugs в стриминге/tool calls; ускоряет
  будущие апгрейды.
- Минусы: потенциально теряем поддержку провайдеров/локальных моделей, которые требуют chat/completions, но:
  - это не upstream-фича,
  - и сейчас мы не считаем ее обязательной без отдельного подтверждения.

**Отдельный аудит (TODO, чтобы решение было "качественным")**

- Аудит вопроса "нужен ли Chat wire форку" должен ответить:
1. Есть ли у нас реально используемые сценарии/пользователи, которым нужен chat/completions.
2. Можно ли закрыть эти сценарии upstream-каноничным Responses (или через upstream-supported провайдеры).
3. Если Chat все же нужен: можно ли оставить его как строго opt-in режим, не затрагивающий дефолты и базовые тесты.

**Связанные карточки, которые должны следовать этому решению**

- `NF-CORE-001`, `NF-CODEX-API-001/002/003`, `NF-MCP-001` (и частично `NF-EXEC-002`, `NF-TUI-006`): при планировании работ
  по ним default должен быть Responses и "upstream-first".

## D1 (исследование / обоснование)

**Факт: upstream 0.98 канон = Responses-only**

- В upstream `WireApi` фактически содержит только `Responses`, и конфиг `wire_api="chat"` **не "deprecated"**, а **invalid**
  (ошибка десериализации + допустимые варианты только `["responses"]`).
- Следствие: "канон = upstream" означает:
  - дефолт и основной путь должны быть `/v1/responses`
  - Chat wire не должен включаться "по умолчанию" и вообще должен считаться не поддержанным (пока не принято отдельное
    продуктовое решение и не сделан отдельный аудит необходимости).

**Текущее состояние форка: Chat wire не просто есть, он стал дефолтом и проник в инфраструктуру**

Критичные точки дрейфа:

- `codex-rs/core/src/model_provider_info.rs`: `WireApi::Chat` сейчас **default**, а `wire_api` в schema тоже default=`chat`.
- `codex-rs/core/src/client.rs`: есть реальный runtime путь `WireApi::Chat => stream_chat_completions_api(...)`.
- `codex-rs/codex-api/*`: добавлен wire-switch (`Provider.wire`, `WireApi::{Responses,Chat,Compact}`), и `ResponsesClient`
  при `WireApi::Chat` ходит в `/chat/completions`.
- Тесты/фикстуры: значимая доля тестов и моков завязана на `wire_api="chat"` и `/v1/chat/completions` (core tests,
  codex-api tests, `mcp-server` suite, `cli_stream` и тестовые фикстуры в config).

**Impact (что сломается при возврате к канону без адаптации)**

- Непосредственно начнут падать/станут нерелевантны:
  - тесты, которые мокают `/v1/chat/completions`
  - тесты, которые ожидают default `wire_api=chat`
  - тесты, которые используют `ChatClient` или chat-SSE парсер
- Часть fork-UX вокруг `ollama-chat`/chat deprecation notice тоже попадет под пересмотр, потому что upstream-позиция: chat
  "removed/invalid", а не "deprecated".

**Корректный план перехода к канону (минимальный по смыслу)**

Шаг 1 (core): вернуть upstream semantics "Responses-only"

- `WireApi` оставить только `Responses`, `chat` сделать invalid при парсинге конфига (сообщение уровня upstream).
- убрать `WireApi::Chat` ветку в `ModelClientSession::stream` и связанные chat-пути/tool-json/deprecation notice.
- регенерировать `codex-rs/core/config.schema.json` так, чтобы enum и default были только `responses`.

Шаг 2 (`codex-api`): убрать wire-switch и chat поверхность

- убрать `Provider.wire`/`WireApi` из `codex-api` (как в upstream).
- `ResponsesClient` всегда path=`responses`.
- удалить/откатить chat endpoint/requests/sse модули.
- (желательно) вернуть upstream websocket события (etag/rate_limits), которые форк урезал.

Шаг 3 (tests/fixtures): удалить/переписать chat-зависимые тесты на Responses

- удалить/переписать `core` и `codex-api` chat tests.
- перевести `mcp-server` mock wire обратно на Responses SSE.
- убрать `wire_api="chat"` из тестовых config fixtures и "openai-chat-completions" провайдера.

**Верификация (как убедиться, что переход корректный)**

- `cargo test -p codex-api`
- `cargo test -p codex-core`
- `cargo test -p codex-mcp-server`
- Проверка: `wire_api="chat"` в config должен **падать на парсинге** с явным сообщением; нигде не должен остаться мок
  `/v1/chat/completions`; schema `wire_api` допускает только `responses`.

**Дополнительный TODO (по требованию)**

- Перед реализацией/в процессе реализации задать исполнителю/ревьюеру отдельный вопрос: "Impact на пользовательский
  функционал форка" (не только тесты), и провести отдельный аудит необходимости chat-провайдеров.

## D2 (зафиксированное решение): A

**Решение**

- `.codex/rules` из trust-disabled (untrusted) layers **полностью игнорируем**. Эти слои **не влияют** на exec/tool policy
  ни в сторону "allow", ни в сторону "deny".

**Почему (контекст)**

- Это security-критичная зона: недоверенный проект не должен иметь возможность повлиять на правила выполнения инструментов.
- Стратегия форка: upstream-first (канон = upstream), минимизируем дрейф и сложные модели "deny-only".

**Следствие для плана работ**

- Любое текущие fork-изменения, из-за которых `.codex/rules` из trust-disabled слоев начинают учитываться, должны быть
  откатаны/приведены к upstream поведению.
- Тесты: должен быть явный тест, что в trust-disabled слой положенный `.codex/rules` не меняет effective policy.

## D3 (pending): requirements enforcement + provenance vs agents tool overrides

Контекст:
- Есть переопределение доступности инструментов из файлов в `~/.codex/agents/*.md` и `.codex/agents/*.md`.

Текущее состояние решения:
- Целевой вариант: upstream-first enforcement + provenance (Option A), если совместимо.
- Если выяснится, что агентские overrides невозможно реализовать без ослабления requirements (или это приводит к слишком
  большому diff/регрессиям), допускается узкий hybrid (Option B) как временная совместимость.
- Требуется отдельное расследование, чтобы сделать выбор A/B корректно.

### D3 (результат расследования)

Источник: `explorer` `019c31c9-62b9-7f81-81c3-68e17a8e11a1` (read-only анализ кода).

**Где вычисляется effective toolset (agents -> config -> tools runtime)**

- Seeding builtin agents: `codex-rs/core/src/codex.rs:312-315` -> `codex-rs/core/src/agent/registry.rs:892-915`
- Roots `.codex/agents` vs `~/.codex/agents`: `codex-rs/core/src/agent/registry.rs:320-357` + discovery/override logic
- Agent YAML применяет allow/deny через `AgentDefinition::apply_to_config()`:
  - `codex-rs/core/src/agent/registry.rs:121-159`
  - allowlist/denylist merge/intersect: `codex-rs/core/src/agent/registry.rs:858-884`
- Реальная фильтрация зарегистрированных tools:
  - `codex-rs/core/src/tools/spec.rs:1557-1574` (`build_specs()` + `tool_allowed()`)

**Как это входит в pipeline requirements/exec-policy**

- Requirements enforcement применяется через `Constrained::set(...)` (approval_policy/sandbox_policy):
  - `codex-rs/core/src/config/mod.rs:1555-1571`
- Exec-policy грузится отдельно и включает requirements rules:
  - `codex-rs/core/src/codex.rs:339-341`
  - `codex-rs/core/src/exec_policy.rs:246-295`
- Agent tool overrides не являются частью `requirements.toml` (они напрямую мутируют `Config.tool_allowlist/tool_denylist`).

**Критичный риск: bypass tool-policy при `spawn_agent` (почему D3=A “не проходит” сейчас)**

- `codex-rs/core/src/tools/handlers/collab.rs:814-854`: `build_agent_spawn_config()` делает:
  - `config.tool_allowlist = config.tool_allowlist_policy.clone();`
  - `config.tool_denylist = config.tool_denylist_policy.clone();`
- При этом `tool_allowlist_policy/tool_denylist_policy` по текущему коду не инициализируются в обычном пути (часто `None`),
  из-за чего при `agent_type=None` (а значит без применения agent профиля) ребёнок получает `tool_allowlist=None` и фактически
  “все tools”.
- Provenance по effective tool policy не фиксируется: есть `AgentDefinition.path/scope`, но нет объяснения “как получился итоговый
  toolset”.

**Можно ли сделать D3=A совместимым с агентскими overrides без ослабления requirements? Да**

Предложенная схема (минимальный смысловой diff):

1. Ввести/инициализировать “tool policy ceiling” через `tool_allowlist_policy/tool_denylist_policy` (глобальный максимум).
2. Сделать agent tool overrides “narrowing only” относительно ceiling:
   - allowlist = intersect(ceiling_allow, agent_allow)
   - denylist = union(ceiling_deny, agent_deny)
3. Запретить “расширение до полного toolset” при `spawn_agent` без `agent_type`:
   - либо не сбрасывать allow/deny в `None`,
   - либо требовать `agent_type` (строже, но поведенчески заметнее).
4. Добавить provenance минимально достаточного уровня (например в turn metadata/tracing): какой ceiling и какой agent профиль
   применились.

**Итог**

- В текущем состоянии: D3=A (upstream-first enforcement + provenance) несовместим с текущей семантикой спавна (есть bypass).
- После минимальных правок выше: D3=A становится совместимым (requirements не ослабляем; agent overrides оформляем отдельным
  narrowing-layer поверх policy ceiling + provenance).

## D4 (зафиксированное решение): defer, upstream-first target

**Контекст**

- Upstream-first стратегия (D1/D2): upstream chunking/commit_tick предпочтительнее.
- Но есть риск зависимости fork-TUI (включая multi-agent overlay / Ctrl+N) от текущей упрощенной семантики стриминга.
- Churn снапшотов `codex-tui` допустим.

**Решение**

1. **Целевое состояние (target): D4 = A (вернуть upstream adaptive chunking + commit_tick).**
2. **Сейчас не внедряем.** Ставим как **последнюю задачу** и **отдельный PR/этап**, чтобы:\n
- изолировать риск регрессий,\n
- проще было понять "что сломалось" если сломается,\n
- легче откатить, не затрагивая остальной апгрейд.

**Backlog item (обязательная подготовка)**

- Отдельно проверить/зафиксировать зависимости multi-agent overlay от стриминга:
- какие компоненты TUI читают "частично закоммиченный" текст,
- не ломает ли upstream commit_tick lifecycle: переключение summary/details overlay, backtrack, transcript pager.

**Верификация для будущей отдельной задачи**

- `cargo test -p codex-tui` + снапшоты (`cargo insta pending-snapshots -p codex-tui`)
- ручной прогон: длинный стрим с бурстами + открыть/закрыть Agents overlay во время стрима + переключение summary/details.
