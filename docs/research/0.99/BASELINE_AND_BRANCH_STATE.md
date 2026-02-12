# Baseline и состояние веток

## Зафиксированная база

- Tag: `rust-v0.99.0`
- Tag object: `cad0bf09f749831b2f1109cf86a122c0a0025075`
- Peeled commit: `ec9f76ce4f854c7d4f3c78c9b1bacbe128df286e`
- Tagger/date: `Josh McKinney | 2026-02-11 11:54:42 -0800 | Release 0.99.0`
- Текущий checkout в рабочем каталоге: `detached HEAD` на `rust-v0.99.0`

## Remotes

- `origin -> https://github.com/NeoGrAIph/codex`
- `upstream -> https://github.com/openai/codex`

Это корректная схема для форка: `upstream` как источник релизов, `origin` как ваш рабочий репозиторий.

## Расхождение ключевых refs

Команды:

```bash
git rev-list --left-right --count main...upstream/main
git rev-list --left-right --count main...rust-v0.99.0
git rev-list --left-right --count origin/main...rust-v0.99.0
```

Результаты:

1. `main...upstream/main = 1 217`
2. `main...rust-v0.99.0 = 1 193`
3. `origin/main...rust-v0.99.0 = 1 240`

Интерпретация `left right`:

- `left`: уникальные коммиты левого ref.
- `right`: уникальные коммиты правого ref.

То есть текущий `main` вашего форка заметно отстает от апстрим-ветки и от релизного тега `0.99.0`.

## Состояние fork-веток относительно `rust-v0.99.0`

Команды:

```bash
git rev-list --count rust-v0.99.0..fork/colab-agents
git diff --name-only rust-v0.99.0..fork/colab-agents | wc -l
# аналогично для других веток
```

Срез:

| Branch | Коммитов поверх `rust-v0.99.0` | Измененных файлов vs `rust-v0.99.0` |
|---|---:|---:|
| `fork/colab-agents` | 238 | 1049 |
| `fork/multi-agent` | 244 | 1057 |
| `fork/skill-agents` | 1 | 561 |
| `main` | 1 | 498 |

### Вывод для minimal diff

1. Уже существующие ветки форка несут очень большой patch stack.
2. Для нового функционала с целью "минимальный diff + легкий future rebase" безопаснее стартовать от чистой базы (`rust-v0.99.0`) в новой ветке.
3. Если функционал обязательно продолжает `fork/multi-agent` или `fork/colab-agents`, нужно заранее принять стоимость регулярного конфликт-резолва в `core/tui/app-server-protocol`.

## Где сейчас основной churn (по последним 120 коммитам)

Команда:

```bash
git log --name-only --pretty=format: -n 120 | sed '/^$/d' \
  | awk -F/ '{print $1"/"$2"/"$3}' | sort | uniq -c | sort -nr | head -n 25
```

Топ по затрагиванию:

1. `codex-rs/core/src` (271)
2. `codex-rs/tui/src` (72)
3. `codex-rs/core/tests` (59)
4. `codex-rs/app-server-protocol/schema` (56)
5. `codex-rs/state/src` (32)
6. `codex-rs/app-server/tests` (31)

Это означает: изменения в этих зонах имеют повышенный риск конфликтов при синхронизации с upstream.

## Рекомендованный baseline для новой работы

1. Отталкиваться от `rust-v0.99.0` (`ec9f76ce4...`).
2. Создать отдельную рабочую ветку под фичу (без смешивания с текущим большим patch stack).
3. Держать документированный reference baseline (commit hash + дата + команды построения diff).

