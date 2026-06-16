# Создание ветки форка от upstream-релиза

Этот документ является обязательной инструкцией для создания новой рабочей ветки форка на основании upstream stable-релиза Codex. Не используй последнюю локально созданную ветку как источник истины: новая release-ветка форка всегда создаётся от upstream release tag или от dereferenced release commit.

## Цель

- создать неизменяемую локальную baseline-ветку для upstream-релиза;
- создать рабочую ветку форка от того же release commit;
- настроить рабочую ветку так, чтобы она отслеживала локальную baseline-ветку, а не `origin/*` или `upstream/*`;
- после переключения baseline актуализировать fork-документацию, которая привязана к последнему покрытому upstream stable-релизу.

## Перед началом

1. Проверь рабочее дерево:
   ```bash
   git status --short --branch
   ```
2. Не удаляй и не откатывай чужие незакоммиченные изменения. Если рабочее дерево мешает переключению ветки, отдельно сохрани нужные изменения в stash или остановись за подтверждением.
3. Обнови и проверь upstream release refs:
   ```bash
   git ls-remote --tags upstream 'refs/tags/rust-v*'
   git fetch upstream --tags --prune
   ```
4. Используй только stable tag вида `rust-vX.Y.Z` для основной ветки форка. Alpha/beta tags можно использовать как evidence, но не как baseline рабочей ветки, если пользователь явно не просит alpha/beta baseline.

## Определение release commit

Для выбранного stable-релиза:

```bash
release_tag=rust-vX.Y.Z
release_commit=$(git rev-parse "${release_tag}^{}")
git show -s --format='%H%n%cI%n%s%n%B' "$release_commit"
```

Проверяй именно dereferenced commit object (`^{}`), потому что release tag может быть annotated tag. Не выводи branch point из имени текущей fork-ветки.

## Имена веток

Для текущей схемы веток форка используется compact release line:

- `rust-v0.140.0` -> `fork/140-upstream` и `fork/140`;
- `rust-v0.141.0` -> `fork/141-upstream` и `fork/141`.

Если upstream когда-нибудь перейдёт на другую major/minor-схему, сначала явно выбери compact release line и зафиксируй его в сообщении пользователю перед созданием веток.

## Создание веток

```bash
release_line=140
git branch "fork/${release_line}-upstream" "$release_commit"
git switch -C "fork/${release_line}" "$release_commit"
git branch --set-upstream-to "fork/${release_line}-upstream" "fork/${release_line}"
```

Если baseline-ветка уже существует, не перезаписывай её молча. Сначала проверь, куда она указывает:

```bash
git rev-parse "fork/${release_line}-upstream"
git show -s --format='%H%n%cI%n%s' "fork/${release_line}-upstream"
```

Пересоздание baseline-ветки допустимо только после явного подтверждения, потому что она является локальным source-of-truth для сравнения fork-изменений с upstream-релизом.

## Проверка после создания

```bash
git rev-parse --abbrev-ref --symbolic-full-name @{u}
git status --short --branch
git rev-parse HEAD
git rev-parse "fork/${release_line}-upstream"
git merge-base --is-ancestor "$release_commit" HEAD
```

Ожидаемо:

- upstream рабочей ветки равен `fork/<release-line>-upstream`;
- `git status --short --branch` показывает `fork/<release-line>...fork/<release-line>-upstream`;
- `HEAD` и `fork/<release-line>-upstream` сразу после создания указывают на один и тот же `release_commit`;
- рабочая ветка не отслеживает `origin/*` или `upstream/*` напрямую.

## Актуализация fork-документации

После создания новой release-ветки проверь документы, которые явно называют последний покрытый upstream stable-релиз или release baseline:

```bash
rg -n 'rust-v0\\.[0-9]+\\.0|fork/[0-9]+|latest stable|последн|baseline release' AGENTS.md README.md docs/fork
```

Минимально поддерживай в актуальном состоянии:

- `AGENTS.md`, если в нём есть пример release branch или ссылка на актуальный workflow;
- `docs/fork/release-branching.md`, если изменилась схема веток или команды проверки;
- `docs/fork/research/**`, если исследовательский документ заявляет покрытие до последнего stable-релиза.

Для исследовательских документов не меняй только статусную строку. Добавь проверяемые release entries, evidence и архитектурные уточнения, либо явно зафиксируй, что документ пока покрывает только предыдущий релиз и требует отдельного research pass.

## Обязательные правила

- Создавай новую fork-ветку только от upstream release tag или dereferenced release commit.
- Настраивай рабочую ветку на отслеживание локальной baseline-ветки `fork/<release-line>-upstream`.
- Определяй последний upstream stable-релиз через upstream refs, а не через последнюю локальную fork-ветку.
- Обновляй release-bound документацию только после проверки tag/commit containment.
