# Интеграция коммита 03fcd12e7 (fork/colab-agents)

> Owner: <team/owner> | Scope: core session updates + collab instructions | Audience: devs
> Status: draft | Last reviewed: 2026-02-03 | Related: docs/fork/colab-agents.md, docs/fork/upstream-main-commits.md

## Цель

Перенести upstream‑коммит `03fcd12e7` ("Do not append items on override turn context") в
`fork/colab-agents` так, чтобы сохранить каноничное поведение upstream и при этом
сохранить/усилить fork‑специфику (collab + agent registry).

## Метаданные коммита

- Hash: `03fcd12e77fedf4fa327af27e2e476e1ebc5f651`
- Тема: "Do not append items on override turn context"
- Автор: pakrym-oai <pakrym@openai.com>
- Дата: 2026-02-01
- Затронутые файлы (upstream):
  - `codex-rs/core/src/codex.rs`
  - `codex-rs/core/src/compact.rs`
  - `codex-rs/core/src/compact_remote.rs`
  - `codex-rs/core/src/stream_events_utils.rs`
  - `codex-rs/core/src/tasks/user_shell.rs`
  - `codex-rs/core/src/tools/handlers/plan.rs`
  - `codex-rs/core/tests/suite/collaboration_instructions.rs`
  - `codex-rs/core/tests/suite/override_updates.rs`
  - `codex-rs/core/tests/suite/permissions_messages.rs`
  - `codex-rs/core/tests/suite/personality.rs`
  - `codex-rs/core/tests/suite/prompt_caching.rs`

## Код коммита (ключевые фрагменты)

Ниже — минимальные, но показательные фрагменты upstream‑diff, определяющие поведение.
Полный diff обязателен к просмотру при реализации:

```
git show 03fcd12e7
```

### TurnContext: полный CollaborationMode

```diff
pub(crate) struct TurnContext {
    ...
-    pub(crate) collaboration_mode_kind: ModeKind,
+    pub(crate) collaboration_mode: CollaborationMode,
    ...
}
```

### build_collaboration_mode_update_item / build_settings_update_items

```diff
-fn build_collaboration_mode_update_item(
-    &self,
-    previous_collaboration_mode: &CollaborationMode,
-    next_collaboration_mode: Option<&CollaborationMode>,
-) -> Option<ResponseItem> {
-    if let Some(next_mode) = next_collaboration_mode {
-        if previous_collaboration_mode == next_mode {
-            return None;
-        }
-        Some(DeveloperInstructions::from_collaboration_mode(next_mode)?.into())
-    } else {
-        None
-    }
-}
+fn build_collaboration_mode_update_item(
+    &self,
+    previous: Option<&Arc<TurnContext>>,
+    next: &TurnContext,
+) -> Option<ResponseItem> {
+    let prev = previous?;
+    if prev.collaboration_mode != next.collaboration_mode {
+        Some(DeveloperInstructions::from_collaboration_mode(&next.collaboration_mode)?.into())
+    } else {
+        None
+    }
+}
```

```diff
-fn build_settings_update_items(
-    &self,
-    previous_context: Option<&Arc<TurnContext>>,
-    current_context: &TurnContext,
-    previous_collaboration_mode: &CollaborationMode,
-    next_collaboration_mode: Option<&CollaborationMode>,
-) -> Vec<ResponseItem> {
+fn build_settings_update_items(
+    &self,
+    previous_context: Option<&Arc<TurnContext>>,
+    current_context: &TurnContext,
+) -> Vec<ResponseItem> {
    ...
-    if let Some(collaboration_mode_item) = self.build_collaboration_mode_update_item(
-        previous_collaboration_mode,
-        next_collaboration_mode,
-    ) {
+    if let Some(collaboration_mode_item) =
+        self.build_collaboration_mode_update_item(previous_context, current_context)
+    {
        update_items.push(collaboration_mode_item);
    }
    ...
}
```

### Override больше не пишет update‑items

```diff
pub async fn override_turn_context(...) {
-    let previous_context = sess.new_default_turn_with_sub_id(...).await;
-    let previous_collaboration_mode = sess.state.lock().await.session_configuration.collaboration_mode.clone();
-    let next_collaboration_mode = updates.collaboration_mode.clone();
     if let Err(err) = sess.update_settings(updates).await { ... }
-    let initial_context_seeded = sess.state.lock().await.initial_context_seeded;
-    if !initial_context_seeded { return; }
-    let current_context = sess.new_default_turn_with_sub_id(sub_id).await;
-    let update_items = sess.build_settings_update_items(
-        Some(&previous_context),
-        &current_context,
-        &previous_collaboration_mode,
-        next_collaboration_mode.as_ref(),
-    );
-    if !update_items.is_empty() {
-        sess.record_conversation_items(&current_context, &update_items).await;
-    }
}
```

### Plan‑mode проверки через `collaboration_mode.mode`

```diff
-let plan_mode = turn_context.collaboration_mode_kind == ModeKind::Plan;
+let plan_mode = turn_context.collaboration_mode.mode == ModeKind::Plan;
```

### Тесты (пример изменения ожиданий)

```diff
-assert_eq!(permissions_2.len(), 3);
+assert_eq!(permissions_2.len(), 2);
```

### Примечание по rollout (TurnContextItem)

В upstream этот коммит не меняет формирование `TurnContextItem` в rollout: там
используется `sess.current_collaboration_mode()`, а не `turn_context.collaboration_mode`.
В рамках интеграции в fork **оставляем это каноничное поведение без изменений**,
чтобы не менять протокольный/исторический формат.

## Что меняет коммит (разбор по функциям)

Ниже — точная карта изменений и их смысл.

### codex-rs/core/src/codex.rs

1) `struct TurnContext`
- **Upstream‑изменение**: заменить `collaboration_mode_kind: ModeKind` на полный
  `collaboration_mode: CollaborationMode`.
- **Эффект**: контекст хода хранит полный режим (mode + settings + dev‑instructions),
  что позволяет корректно сравнивать режимы.
- **Влияние на fork**: лучшее сравнение при изменении параметров режима без смены `ModeKind`.

2) `Session::make_turn_context`
- **Upstream‑изменение**: сохранять полный `CollaborationMode` в `TurnContext`.
- **Эффект**: меньше зависимости от session config при вычислении режима.
- **Влияние на fork**: корректнее наследование параметров режима в `spawn_agent` и иных местах.

3) `Session::build_collaboration_mode_update_item`
- **Upstream‑изменение**: сравнение `previous_context.collaboration_mode` и
  `current_context.collaboration_mode` (полные объекты), а не `ModeKind`.
- **Эффект**: update‑items появляются только при реальном изменении режима.
- **Влияние на fork**: меньше дублей collab‑инструкций.

4) `Session::build_settings_update_items`
- **Upstream‑изменение**: сигнатура становится
  `build_settings_update_items(previous_context, current_context)`.
- **Эффект**: единый способ вычисления изменений (env/permissions/collab/personality).
- **Влияние на fork**: упрощение и каноничность логики.

5) `handlers::override_turn_context`
- **Upstream‑изменение**: **не** писать update‑items в rollout на override.
- **Эффект**: override становится «тихим» до следующего user‑turn.
- **Влияние на fork**: меньше лишних записей при частых override в collab/agents.

6) `handlers::user_input_or_turn`
- **Upstream‑изменение**: update‑items формируются только из
  `previous_context` vs `current_context`.
- **Эффект**: одна серия update‑items на user‑turn, даже после override.
- **Влияние на fork**: корректный и недублируемый контекст.

7) `spawn_review_thread`
- **Upstream‑изменение**: передавать полный `CollaborationMode` в review‑поток.
- **Эффект**: review наследует настройки режима, а не только тип.
- **Влияние на fork**: согласуется с fork‑collab настройками (model/effort/instructions).

8) Проверки plan‑mode через `turn_context.collaboration_mode.mode`
- **Upstream‑изменение**: заменяются обращения к `collaboration_mode_kind` в:
  - `run_turn`
  - `compact.rs`
  - `compact_remote.rs`
  - `stream_events_utils.rs`
  - `tasks/user_shell.rs`
  - `tools/handlers/plan.rs`
- **Эффект**: механическое выравнивание с новым полем, поведение не меняется.

### codex-rs/core/tests/suite/*

Тесты фиксируют новый контракт поведения:

- `collaboration_instructions.rs`: инструкции добавляются только при реальном изменении режима.
- `collaboration_instructions.rs`: override + последующий `Op::UserInput` должен применять
  обновлённые collab‑инструкции (не только `Op::UserTurn`).
- `collaboration_instructions.rs`: если `UserTurn` задаёт свой `collaboration_mode`, базовые
  инструкции от override **не** дублируются в том же запросе.
- `collaboration_instructions.rs`: смена `ModeKind` (например Code → Plan) должна добавлять
  новую инструкцию, повторение того же режима — не должно.
- `collaboration_instructions.rs`: добавляется helper `collab_mode_with_mode_and_instructions`,
  чтобы явно тестировать смену `ModeKind` (Code/Plan) с разными инструкциями.
- `override_updates.rs`: override без user‑turn не пишет env/permissions/collab updates.
- `permissions_messages.rs`: меньше дублей после override.
- `personality.rs`: повторное значение personality не добавляет update‑message.
- `prompt_caching.rs`: после override появляется ровно одно обновление.

## Адаптация fork‑изменений (каноничность прежде всего)

1) **TurnContext хранит полный CollaborationMode** — это канонично и полезно для fork.
2) **Override без update‑items** — соответствует upstream и уменьшает дубль‑инструкции.
3) **Update‑items только по контекст‑дельте** — единая каноничная логика.
4) **Agent registry** сохраняется, но выигрывает от более точной передачи режима.

## Проект интеграции (минимальный diff, канонично)

## Процедура интеграции (обязательные правила форка)

1) **Анализ → план → подтверждение → внедрение.** Реализацию выполнять только после явного подтверждения.
2) **Трекинг статуса коммита.** В `docs/fork/upstream-main-commits.md` обновить статус и
   добавить краткий комментарий о способе интеграции (чистый cherry‑pick / ручная адаптация).
   Коммит из списка не удалять.

### Шаг 1: TurnContext
- Заменить `collaboration_mode_kind: ModeKind` → `collaboration_mode: CollaborationMode`.
- Обновить все конструкторы `TurnContext` (включая review threads).
- Все план‑проверки перевести на `turn_context.collaboration_mode.mode`.

### Шаг 2: update‑items
- Заменить `build_collaboration_mode_update_item(...)` на версию по контекстам.
- Заменить `build_settings_update_items(...)` на версию по контекстам.

### Шаг 3: Override‑семантика
- В `override_turn_context` удалить запись update‑items в rollout.
- Оставить только `update_settings`.

### Шаг 4: user_input_or_turn
- Строить update‑items только по `previous_context` vs `current_context`.
- Сохранить fork‑логику collab/registry без изменений.
- Убедиться, что обновления применяются и для `Op::UserInput`, а не только `Op::UserTurn`.

### Шаг 5: Тесты
- Принять upstream‑ожидания: никаких update‑items на override без user‑turn,
  без дублей инструкций и personality.

## Подробное описание действий (по файлам)

Эта секция — пошаговый, детальный чек‑лист «что именно менять», без cherry‑pick.

### `codex-rs/core/src/codex.rs`

1) `TurnContext`
- Заменить поле `collaboration_mode_kind` на `collaboration_mode`.
- В `make_turn_context` сохранить полный режим:
  `collaboration_mode: session_configuration.collaboration_mode.clone()`.

2) Места, где используется `ModeKind`
- Заменить на `turn_context.collaboration_mode.mode`:
  - `run_turn`
  - `try_run_sampling_request` (plan‑mode ветка)
  - `spawn_review_thread` (при создании `TurnContext`)
  - `plan_mode` проверки в `run_turn`

3) Update‑items
- Переписать `build_collaboration_mode_update_item` и `build_settings_update_items`
  на сравнение `previous_context` vs `current_context`.
- Удалить явные аргументы `previous_collaboration_mode` и `next_collaboration_mode`.

4) Override‑семантика
- В `handlers::override_turn_context` удалить запись update‑items в rollout.
- Оставить только `update_settings` и обработку ошибок.

5) User input / turn
- В `handlers::user_input_or_turn` строить update‑items только по контекстам.
- Убедиться, что fork‑логика collab/agent registry остаётся без изменений.
- Не потерять поведение: обновления должны срабатывать на следующем `Op::UserInput`
  после override, даже без `Op::UserTurn`.

### `codex-rs/core/src/compact.rs` и `compact_remote.rs`
- Заменить `turn_context.collaboration_mode_kind` на
  `turn_context.collaboration_mode.mode` при формировании `TurnStartedEvent`.

### `codex-rs/core/src/stream_events_utils.rs`
- Plan‑mode проверка должна опираться на `turn_context.collaboration_mode.mode`.

### `codex-rs/core/src/tasks/user_shell.rs`
- Plan‑mode проверка/поле `TurnStartedEvent` — через `collaboration_mode.mode`.

### `codex-rs/core/src/tools/handlers/plan.rs`
- Проверку запрета `update_plan` в plan‑mode перевести на
  `turn_context.collaboration_mode.mode`.

### Тесты

Принять upstream‑ожидания в следующих файлах:
- `collaboration_instructions.rs`: обновить кейсы на смену/не‑смену режима.
- `collaboration_instructions.rs`: обновить ожидание в кейсе `user_turn_overrides_collaboration_instructions_after_override`
  (базовые инструкции не дублируются, ожидается `0`).
- `override_updates.rs`: override без user‑turn не пишет env/permissions/collab.
- `permissions_messages.rs`: количество сообщений после override уменьшается.
- `personality.rs`: повторная personality не пишет update.
- `prompt_caching.rs`: после override появляется одно обновление.

## Конфликтные зоны и решения

1) `codex-rs/core/src/codex.rs`
- Высокая вероятность конфликтов с fork‑collab/registry.
- **Решение**: принимать upstream‑логику, а fork‑поведение накладывать поверх
  (никаких откатов к старому механизму).

2) `spawn_review_thread`
- Передавать полный `CollaborationMode`.

3) `tools/handlers/collab.rs`
- Прямых правок не требует, но все обращения к режиму должны использовать
  `turn.collaboration_mode`.

## Проверка (если будет запрошена)

- `cd codex-rs && cargo test -p codex-core` (обязательно для core‑изменений)
- `cargo test --all-features` — только с явным подтверждением, после накопления core‑коммитов

## Ожидаемый результат после интеграции

- Override не пишет update‑items; они появляются на следующем user‑turn.
- Collab‑инструкции обновляются только при фактическом изменении режима.
- Personality/permissions не дублируются, если значения не менялись.
- Fork‑поведение collab/agent registry остаётся, но становится более стабильным.

## Открытые вопросы

- Нужна ли краткая пометка в `docs/fork/colab-agents.md` о «тихих» override?
- Нужны ли fork‑специфические тесты на не‑дублирование collab‑инструкций?
 - Нужно ли зафиксировать в отдельной заметке, что протокол событий (включая
   `TurnStartedEvent` и формат rollout) не меняется этим коммитом?
