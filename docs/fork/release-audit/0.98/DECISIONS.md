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

**Статус**

- ✅ **DONE** (Stage 2 / Security)
- Code commits:
  - `73681a6e5` — D2: disabled layers больше не участвуют в загрузке `.codex/rules` (exec-policy), т.е. trust-disabled layers не влияют на policy.
  - `5ab253881` — D2 hardening: trust-gating project layers независимо от наличия `.codex/config.toml` (закрывает bypass “rules без config.toml”) + интеграционный тест.
- Docs commits:
  - `04187a729` — audit docs: отметить D2/D3 как DONE (первичная отметка).
  - `c07037981` — audit docs: добавить финальный commit ref для D2 trust-gating.

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

## D3 (зафиксированное решение): A

**Статус**

- ✅ **DONE** (Stage 2 / Security)
- Commits:
  - `73681a6e5` — D3: `spawn_agent` больше не может расширить toolset (ребёнок наследует runtime allow/deny родителя).
  - `2107dc485` — D3: восстановлены requirements enforcement + provenance (включая fallback дефолтов на requirement-default).
  - `04187a729` — audit docs: отметить D2/D3 как DONE.
  - `c07037981` — audit docs: добавить финальный commit ref (см. также D2).

Контекст:
- Есть переопределение доступности инструментов из файлов в `~/.codex/agents/*.md` и `.codex/agents/*.md`.

Текущее состояние решения:
- ✅ Решение = **A**: upstream-first enforcement + provenance, без ослабления requirements из-за agent overrides.
- Агентские tool overrides должны быть “narrowing only” (не расширять доступные инструменты относительно политики/родителя).

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

- Update (2026-02-06): ✅ **DONE** — см. commits в статусе секции.
- Исторический контекст расследования:
  - В исходном состоянии: D3=A (upstream-first enforcement + provenance) был несовместим с семантикой спавна (был bypass).
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

## D5 (зафиксированное решение): upstream-native local skills + native experimental/live reload; remote disabled

**Решение**

- Переходим на upstream-native механики **local skills** и используем upstream-native experimental механики **live updates/reload**
  (без remote downloads).
- Upstream-native remote skills (downloader + RPC/events) **не включаем**, пока не будет отдельного security-дизайна (policy gate +
  provenance + allowlist источников).
- Допускаем surface-совместимость: remote skills RPC методы могут существовать в протоколе для совместимости с upstream, но
  **при дефолтной политике должны возвращать явную ошибку** и не выполнять network egress (см. D13/NF-APP-SERVER-002).
- Для корректности live reload фиксируем security-инвариант: `.agents` (и следовательно `.agents/skills`) должен быть read-only в
  sandboxed workspace roots (связано с D8).

### D5 (результат расследования / обоснование)

Источник: `explorer` `019c31c9-62b9-7f81-81c3-68e17a8e11a1` (read-only анализ кода и release-audit патчей).

**Upstream-native local skills (и у нас по сути уже есть)**

- Roots/discovery (слои + директории): `codex-rs/core/src/skills/loader.rs:168-270`
  - Project: `.codex/skills`
  - User: `$CODEX_HOME/skills` (deprecated, но поддерживается)
  - User: `$HOME/.agents/skills`
  - System embedded cache: `$CODEX_HOME/skills/.system`
  - System(admin): `/etc/codex/skills`
  - Repo: `.agents/skills` (ищется между project-root и cwd)
- Live updates / reload (нативная experimental механика без remote):
  - watcher event: `codex-rs/core/src/codex.rs:786-800` (`EventMsg::SkillsUpdateAvailable`)
  - TUI auto reload: `codex-rs/tui/src/chatwidget.rs:3538-3543` (`Op::ListSkills { force_reload: true }`)
  - cache bypass: `codex-rs/core/src/skills/manager.rs:63` (`force_reload`)
- Нативные deps/metadata mechanics (experimental):
  - `agents/openai.yaml` metadata: `codex-rs/core/src/skills/loader.rs:509+`
  - MCP deps install prompt: `codex-rs/core/src/codex.rs:3659+` + `codex-rs/core/src/mcp/skill_dependencies.rs:148+`
  - env var deps prompt: `codex-rs/core/src/codex.rs:3650-3657`

**Upstream-native remote skills (в 0.98)**

- Downloader: `codex-rs/core/src/skills/remote.rs` (в форке удалён, см. `NF-CORE-006`)
- Protocol wiring: `Op::{ListRemoteSkills,DownloadRemoteSkill}` и `EventMsg::{ListRemoteSkillsResponse,RemoteSkillDownloaded}`
  (в форке удалены, см. `NF-PROTO-001`)
- JSON-RPC: `skills/remote/read|write` (в форке удалены, см. `NF-APP-SERVER-002`)
- Важное: по анализу upstream 0.98 remote skills не выглядят feature-gated через `Feature::*` и включаются фактом вызова + URL
  (то есть это сетевой egress внутри core/app-server).

**Security вывод (почему remote skills риск)**

- Remote skills = сетевой download внутри core/app-server, то есть потенциальный обход fork posture (sandbox/network disabled через tools).
- Дополнительно: `.agents/skills` является нативным root; значит нужно обеспечить, что агент не может писать туда в sandboxed workspace,
  иначе это канал персистентной prompt-инъекции (watcher подхватит изменения).

**Рекомендация расследования (upstream-first + security)**

- Предпочесть вариант **B + часть C**:
  - upstream-native local-only skills + upstream-native experimental/live reload
  - восстановить upstream security инвариант “`.agents` read-only” в sandbox workspace roots
  - remote skills (downloader + RPC/events) не включать без отдельного security-дизайна (policy gate + provenance + allowlist источников).

## D6 (зафиксированное решение): A (не сериализовать `ModeKind::Custom` on-wire)

**Статус**

- Реализовано в commit `e692cb2e9` (см. также D13): `Custom` исключён из schemars/TS exports и не появляется on-wire; сериализация/rollout не падают.

**Решение**

- `ModeKind::Custom` считается **строго internal sentinel** и **никогда не сериализуется on-wire** (в app-server протокол /
  внешние ответы).
- Для внешнего протокола состояние "режим не выбран" представлять нативно/upstream-совместимо: `None`/отсутствие override или
  `Default` (в зависимости от существующего контракта).

**Почему**

- Минимизируем breaking для внешних клиентов.
- Upstream-first: избегаем служебных значений в публичном протоколе.
- Снижаем дрейф и упрощаем будущие апгрейды.

**Требование к реализации**

- Реализовать максимально нативно и с минимальным diff:
- не добавлять новые поля/версии протокола, если можно выразить через существующие `Option`/default semantics
- изменить сериализацию/маппинг так, чтобы `Custom` не попадал наружу
- обновить тесты (в т.ч. `NF-APP-SERVER-004`/`NF-PROTO-004`) под новую on-wire семантику

## D7 (зафиксированное решение): A (TS optional/nullable align с upstream)

**Статус**

- Отложено: намеренно не реализовано в рамках стабилизации D13 (вынесено в отдельный этап/PR, чтобы не смешивать большой churn схем).

**Решение**

- Выравниваем правила optional/nullable в TS schema и генерации типов строго по upstream (канон = upstream).

**Почему**

- Минимизируем дрейф и будущую стоимость апгрейдов.
- Снижаем риск compile-time/regression break для TS клиентов.

**Требование к реализации**

- Делать максимально нативно и с минимальным diff.
- Обновить/перегенерировать соответствующие схемы/артефакты, и обновить тесты/клиентов, которые завязаны на старые правила.

## D8 (зафиксированное решение): A (sandbox read-only for `.agents` and agent profiles)

**Решение**

- В sandboxed workspace roots делаем `.agents/**` read-only (upstream-first security posture).
- В части форка с агент-профилями: `.codex/agents/**` и/или любые альтернативные каталоги с агентскими профилями должны быть
  read-only в sandboxed режимах, чтобы исключить self-modifying поведение.

**Почему**

- Синергия с D5: `.agents/skills` участвует в upstream-native discovery + live reload; если `.agents` writable, это канал
  персистентной prompt-инъекции.
- Upstream-first: возвращаем/сохраняем security-инвариант, который upstream применял для `.agents` и `.codex`.

**Требование к реализации**

- Реализовать максимально нативно и с минимальным diff (через существующий список sandbox read-only subpaths).
- Добавить проверку/тест, что `.agents` попадает в read-only subpaths в соответствующих sandbox policies.

## D9 (зафиксированное решение): A (вернуть upstream build-time vendored bubblewrap pipeline)

**Контекст**

- Канон = upstream (`rust-v0.98.0`).
- В апстриме Linux sandbox поддерживает bwrap-путь через vendored bubblewrap FFI, собираемый на build-time (гейтится
  `CODEX_BWRAP_ENABLE_FFI=1`).
- В форке bwrap-путь сейчас фактически “мертв”: включение `features.use_linux_sandbox_bwrap` приводит к runtime ветке
  `--use-bwrap-sandbox`, которая упирается в `vendored_bwrap`, но сборочных артефактов/исходников нет.

**Решение**

- D9 = **A**: возвращаем **полностью нативную upstream** supply chain для bubblewrap:
  - восстановить `codex-rs/linux-sandbox/build.rs` (как в upstream),
  - вернуть build-deps (`cc`, `pkg-config`) в `codex-rs/linux-sandbox/Cargo.toml`,
  - восстановить `codex-rs/vendor/bubblewrap/**` (как в upstream).

**Почему**

- Это не ломает fork-фичи по умолчанию: `features.use_linux_sandbox_bwrap` в upstream default=`false`, то есть
  поведение по дефолту не меняется.
- Убираем “footgun”: bwrap фича снова становится работоспособной при правильной сборке, как в upstream.
- Альтернатива (hybrid с system `bwrap`) создаёт дополнительный дрейф относительно upstream и требует отдельного
  дизайна/детектов (возможны фейлы в CI/контейнерах из-за отсутствия/политики userns/setuid).

**Результат расследования / факты**

Источник: `explorer` `019c31f5-c0ab-7070-bf8c-d58f7e996a50` (read-only анализ).

- В форке удалены:
  - `codex-rs/linux-sandbox/build.rs`
  - `[build-dependencies] cc, pkg-config` из `codex-rs/linux-sandbox/Cargo.toml`
  - `codex-rs/vendor/bubblewrap/**`
- Rust-логика bwrap-пайплайна при этом осталась upstream-совместимой, но при запуске `--use-bwrap-sandbox` вызывает
  `exec_vendored_bwrap(...)`, который при отсутствии build-time bubblewrap становится panic/skip.

**Ключевые ссылки на код**

- Фича/прокидывание: `codex-rs/core/src/features.rs`, `codex-rs/core/src/codex.rs`,
  `codex-rs/core/src/sandboxing/mod.rs`, `codex-rs/core/src/landlock.rs`
- Исполнение linux sandbox: `codex-rs/linux-sandbox/src/linux_run_main.rs`,
  `codex-rs/linux-sandbox/src/vendored_bwrap.rs`

**Верификация**

- Минимум: `cd codex-rs && cargo test -p codex-linux-sandbox`
- Опционально (если окружение поддерживает сборку FFI): `CODEX_BWRAP_ENABLE_FFI=1 cargo test -p codex-linux-sandbox --test landlock`

## D10 (зафиксированное решение): A (вернуть upstream Windows sandbox security boundary)

**Контекст**

- Канон = upstream (`rust-v0.98.0`).
- `codex-rs/windows-sandbox-rs` определяет security boundary для выполнения команд на Windows (ACL + restricted token +
  capability SID).
- Цель форка: security posture **не хуже upstream**, минимальный дрейф.

**Решение**

- D10 = **A**: возвращаем **чистую upstream** реализацию Windows sandbox (включая per-workspace capability и защиту `.codex`),
  а все fork-упрощения, которые расширяют права/делают права накопительными, убираем.

**Почему**

- Текущие fork-изменения выглядят как ослабление boundary:
  - per-workspace capability (keyed by CWD) удалена → capability становится глобальной/накопительной между воркспейсами;
  - убрана защита `CWD/.codex` deny-ACE → `.codex` становится writable из sandbox (важно для форка: agents/rules/config);
  - deny-write маска ослаблена (убраны delete-биты) → возможен обход через destructive delete.
- Upstream-first: возврат к канону снижает риск security регрессий и снижает стоимость будущих апгрейдов.

**Результат расследования / факты**

Источник: `explorer` `019c31f8-eb2b-7412-ba72-4504d9d0fd9c` (read-only анализ патчей).

- NF-WIN-SB-001/002 вместе переводят модель на single capability SID без per-workspace cap, убирают `command_cwd` из payload и
  удаляют `workspace_acl` защиту `.codex`.
- NF-WIN-SB-004 убирает `DELETE` и `FILE_DELETE_CHILD` из deny-write маски.
- NF-WIN-SB-003 в основном рефактор нормализации пути (низкий риск), но в upstream ключ нормализации также использовался для cap-key.

**Требование к реализации (минимальный смысловой diff, но канонично)**

1. Вернуть upstream security boundary пакетно (001+002):
  - восстановить per-workspace capability SID keyed by CWD;
  - вернуть `command_cwd` в payload;
  - восстановить `workspace_acl` и deny-ACE для `CWD/.codex`;
  - вернуть multi-cap токены (`cap_sids: Vec<String>`), соответствующие ACL.
2. Вернуть upstream path normalization модуль/тесты (003), чтобы ключи путей и cap-key совпадали с upstream.
3. Вернуть delete-биты в deny-write mask (004).

**Верификация**

- Unit/serde тесты (без WinAPI):
  - payload содержит `command_cwd`;
  - runner request содержит `cap_sids` (multi-cap);
  - path normalization key стабилен при разном регистре/разных слэшах.
- Ручные проверки (Windows):
  - cross-workspace isolation (права не “накапливаются” между воркспейсами);
  - `.codex` tamper resistance;
  - deny включает delete (удаление deny-путей запрещено).

## D13 (зафиксированное решение): app-server surface compat (NF-APP-SERVER-001..004)

**Статус**

- Реализовано в commit `e692cb2e9`.
- Прогнаны проверки: `cargo test -p codex-app-server-protocol`, `cargo test -p codex-app-server`.

**Контекст**

- Канон = upstream (`rust-v0.98.0`).
- `codex-rs/app-server` предоставляет JSON-RPC surface, который используют клиенты (в т.ч. IDE интеграции).
- Публичные изменения surface без версионирования дорого стоят: ломают типизированных клиентов и увеличивают дрейф.

**Решение**

1. **NF-APP-SERVER-001 (`thread/compact/start`) = A:** вернуть upstream RPC метод `thread/compact/start` и связанную доку/тесты.
2. **NF-APP-SERVER-002 (`skills/remote/read|write`) = B (hybrid):** вернуть методы в протокол для surface-совместимости,
   но **по умолчанию отключить** (без network egress) и возвращать явную ошибку “remote skills disabled by policy” до тех пор,
   пока не будет отдельного security-дизайна (policy gate + provenance + allowlist источников).
3. **NF-APP-SERVER-003 (`model/list` поле `upgrade`) = A:** вернуть optional поле `upgrade` в ответ `model/list`.
4. **NF-APP-SERVER-004 (`ModeKind::Custom` on-wire) = A:** не сериализовать `ModeKind::Custom` наружу (см. D6), выровнять
   on-wire представление с upstream (использовать `Default`/`None`-семантику контракта вместо внутреннего sentinel).

**Почему**

- Минимизируем дрейф от upstream и риск ломать клиентов.\n
- Remote skills остаётся потенциально опасной surface (network download, supply chain), поэтому корректный компромисс:
  surface совместим, но поведение gated политикой.\n
- Внутренние служебные значения (`Custom`) не должны попадать on-wire.

**Требование к реализации**

- Протокол/схемы/TS types: вернуть upstream схемы для методов/полей, но обеспечить, что реализация remote skills не выполняет
  download, пока политика запрещает.\n
- Документация (`codex-rs/app-server/README.md`): явно отметить, что remote skills в форке отключены по умолчанию.\n
- Тесты: обновить app-server suite:
  - `thread/compact/start` снова работает;\n
  - `model/list` содержит `upgrade` как optional;\n
- override collaboration mode не приводит к `ModeKind::Custom` on-wire;\n
- `skills/remote/*` возвращает ожидаемую ошибку при default policy (если методы возвращаем).\n

## D11 (зафиксированное решение): state DB + dynamic tools (NF-STATE-001/002)

**Контекст**

- Канон = upstream (`rust-v0.98.0`), но state DB уже существует у пользователей форка и может содержать данные, которые нельзя
  терять при апгрейде.
- `codex-rs/state` отвечает за путь к SQLite, миграционную/версионную стратегию и персистентность derived данных
  (включая dynamic tools).

**Решение**

1. **NF-STATE-001 = B (hybrid совместимость без потери данных):**
  - при движении к upstream-канону (версионированный `state_<ver>.sqlite` + legacy cleanup) требуется слой совместимости,
    чтобы не потерять/не “осиротить” текущий `state.sqlite`.\n
  - запрещено “просто включить upstream cleanup”, если он может удалить `state.sqlite` как legacy.\n
  - целевое поведение: при старте выполнить детект/перенос/выбор приоритета между `state.sqlite` и `state_<ver>.sqlite`,
    а cleanup запускать только после успешной миграции/согласования.
2. **NF-STATE-002 = A (вернуться к upstream):**
  - вернуть upstream-семантику `persist_dynamic_tools`: idempotent insert по `(thread_id, position)` с `ON CONFLICT DO NOTHING`.\n
  - убрать fork-early-return “если есть хотя бы одна запись, больше не вставлять”, чтобы не получать “залипшие/неполные tools”.

**Почему**

- NF-STATE-001 (B): чистый возврат к upstream почти гарантирует data loss/игнор существующей `state.sqlite` у форк-установок.\n
- NF-STATE-002 (A): upstream семантика безопаснее для инкрементальной записи и самовосстановления, минимальный diff и
  устраняет P0 риск “stuck tools”.

**Требование к реализации**

- Добавить совместимую миграцию путей state DB (rename/copy/приоритеты) и обновить все места, которые делают existence-check
  для backfill.\n
- Добавить тест: повторный persist dynamic tools дописывает недостающие позиции (и не ломает существующие).

## D12 (зафиксированное решение): B (upstream abort semantics for exec + end-to-end cascading shutdown)

**Контекст**

- Канон = upstream: abort/interrupt (`TurnAborted`) должен детерминированно останавливать активные задачи и процессы.\n
- Fork-требование: если закрываем оркестратора, его подагенты должны закрываться рекурсивно (cascading shutdown).

**Решение**

- D12 = **B**:\n
  1. **Вернуть upstream-семантику для `codex exec`: `TurnAborted => InitiateShutdown`**, чтобы `exec` не зависал в event loop.\n
  2. **Сохранить fork-каскадный shutdown**, но довести его до end-to-end: реальные пути закрытия треда (TUI/app-server)
     должны использовать каскадный API (например `AgentControl::shutdown_agent()`), а не обходить его через
     `Op::Shutdown + remove_thread`.

**Почему**

- Это минимизирует дрейф относительно upstream (возвращаем каноничное завершение exec) и при этом сохраняет fork-функцию
  каскадного закрытия.\n
- Текущее состояние: механизм каскада есть, но TUI/app-server его обходят, что приводит к orphan subagents.

**Результат расследования / факты**

Источник: `explorer` `019c3202-f301-7563-891a-0f466f9ec404` (read-only анализ).

- Exec регрессия:\n
  - `codex-rs/exec/src/event_processor_with_human_output.rs` и `..._with_jsonl_output.rs`: `TurnAborted` не инициирует shutdown.\n
  - `codex-rs/exec/src/lib.rs`: `Op::Shutdown` отправляется только при `CodexStatus::InitiateShutdown`.\n
- Каскад реализован в core через `AgentControl::shutdown_agent()` (есть unit-тест), но:\n
  - `codex-rs/tui/src/app.rs`: закрытие треда делает `Op::Shutdown + remove_thread` без каскада.\n
  - `codex-rs/app-server/src/codex_message_processor.rs`: archive/close активного треда делает `remove_thread()` и затем `Op::Shutdown`,
    обходя каскад.\n

**Требование к реализации**

- Добавить/обновить тесты:\n
  - `codex-exec`: unit тест на `TurnAborted => InitiateShutdown` (human + jsonl processors).\n
  - Core/TUI/app-server: end-to-end тест, что закрытие orchestrator через публичный путь закрывает и детей.\n
  - (по возможности) усилить интеграционный тест, что abort действительно убивает subprocess (нет side-effects после interrupt).\n
