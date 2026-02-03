# Интеграция коммита 6c22360bc (fork/colab-agents)

> Owner: <team/owner> | Scope: execpolicy prefix_rule append | Audience: devs  
> Status: draft | Last reviewed: 2026-02-03 | Related: docs/fork/colab-agents.md, docs/fork/upstream-main-commits.md

## Цель

Перенести upstream‑коммит `6c22360bc` ("Deduplicate prefix_rules before appending") в
`fork/colab-agents`, сохранив каноничное поведение upstream и не ломая fork‑специфику
(exec‑policy и approval‑потоки).

## Метаданные коммита

- Hash: `6c22360bcbbcf97a75725a6cab1c5c7da2848085`
- Тема: "fix(core) Deduplicate prefix_rules before appending"
- Автор: Dylan Hurd <dylan.hurd@openai.com>
- Дата: 2026-02-01
- Затронутые файлы (upstream):
  - `codex-rs/execpolicy/src/amend.rs`
  - `codex-rs/execpolicy/tests/basic.rs`

## Код коммита (ключевые фрагменты)

Полный diff:

```
git show 6c22360bc
```

### `append_locked_line`: дедупликация перед записью

```diff
fn append_locked_line(policy_path: &Path, line: &str) -> Result<(), AmendError> {
    ...
-    let len = file.metadata()?.len();
-    if len > 0 { ... read last byte ... }
+    file.seek(SeekFrom::Start(0))?;
+    let mut contents = String::new();
+    file.read_to_string(&mut contents)?;
+
+    if contents.lines().any(|existing| existing == line) {
+        return Ok(());
+    }
+
+    if !contents.is_empty() && !contents.ends_with('\n') {
+        file.write_all(b"\n")?;
+    }
    file.write_all(format!("{line}\n").as_bytes())?;
}
```

### Новый тест: повторная запись той же prefix_rule

```diff
#[test]
fn append_allow_prefix_rule_dedupes_existing_rule() -> Result<()> {
    let tmp = tempdir()?;
    let policy_path = tmp.path().join("rules").join("default.rules");
    let prefix = tokens(&["python3"]);

    blocking_append_allow_prefix_rule(&policy_path, &prefix)?;
    blocking_append_allow_prefix_rule(&policy_path, &prefix)?;

    let contents = fs::read_to_string(&policy_path)?;
    assert_eq!(contents, r#"prefix_rule(pattern=["python3"], decision="allow")\n"#);
    Ok(())
}
```

## Что меняет коммит (разбор по функциям)

### `codex-rs/execpolicy/src/amend.rs`

1) `append_locked_line`
- **Upstream‑изменение**: файл читается целиком, и если строка `line` уже есть,
  запись не производится.
- **Эффект**: предотвращает дублирование `prefix_rule` при повторных append.
- **Влияние на fork**: стабилизирует файл правил при повторных запросах
  на автоматическое добавление allow‑prefix, снижает рост файла.

2) Проверка конца файла
- **Upstream‑изменение**: вместо чтения последнего байта теперь используется
  `contents.ends_with('\n')`.
- **Эффект**: упрощает логику; меньше операций seek/read.
- **Влияние на fork**: нейтрально; формат файла остаётся идентичным.

### `codex-rs/execpolicy/tests/basic.rs`

1) `append_allow_prefix_rule_dedupes_existing_rule`
- **Upstream‑изменение**: новый тест гарантирует, что повторный append не добавит
  дубликат строки.
- **Влияние на fork**: зафиксирован контракт, полезен для наших approval‑потоков.

## Влияние на fork/colab-agents

- Прямых конфликтов с fork‑изменениями не ожидается: это локальная логика
  `execpolicy` и тест.
- Улучшает стабильность при повторных `prefix_rule` запросах (например, когда одна
  и та же команда многократно получает auto‑approval).
- Никаких изменений форматов или API не вводится.

## Проект интеграции (пошагово)

1) **Код**
   - Обновить `append_locked_line` в `codex-rs/execpolicy/src/amend.rs` по upstream‑логике:
     чтение файла целиком, дедуп, корректный перенос строки перед append.

2) **Тесты**
   - Добавить тест `append_allow_prefix_rule_dedupes_existing_rule` в
     `codex-rs/execpolicy/tests/basic.rs` (с `tempdir`, `blocking_append_allow_prefix_rule`).

3) **Форматирование**
   - `cd codex-rs && just fmt`

4) **Проверка (по запросу)**
   - `cd codex-rs && cargo test -p codex-execpolicy`
   - `cargo test --all-features` — только с явным подтверждением.

## Ожидаемый результат после интеграции

- Повторный append одной и той же `prefix_rule` не приводит к дубликатам.
- Поведение exec‑policy остаётся каноничным, без изменения форматов.
- Файл правил не растёт из‑за повторных auto‑approval.

## Риски и нюансы

- Чтение файла целиком добавляет O(n) на append — приемлемо для небольших
  policy‑файлов (ожидаемый размер мал).
- Дедуп делается по точной строке `line`; если в файле есть эквивалентная строка
  с другими пробелами/форматом, она не будет считаться дубликатом (каноничное
  поведение upstream).

## Открытые вопросы

- Нужно ли дополнительно документировать это поведение в `execpolicy/README.md`
  или в fork‑документах? Пока upstream не требует — оставляем как есть.
