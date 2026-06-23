# Актуализация multi-agents research

## Назначение

Prompt используется, когда `docs/fork/research/multi-agents/` уже существует и нужно поддержать документы в актуальном состоянии после нового upstream stable-релиза Codex.

## Ready-to-paste goal

```text
/goal Destination: в ~/repo/AGENTS/codex-fork актуализировать docs/fork/research/multi-agents/{timeline.md,evidence.md,architecture.md} от текущего documented stable release до latest upstream stable rust-vX.Y.0. Прочитай docs, найди последний покрытый tag и добавь только недостающие stable sections, не пересоздавая историю.

Context: docs-only, русский текст; upstream commit subjects, PR ids, API/type/config/tool/wire names сохранять как есть. Не смешивать stable timeline с alpha-only или fork-only историей.

Scope: трогать только multi-agents research docs, кроме явной правки ссылки/индекса. Для каждого нового stable release добавить metadata, relevant commits, developer/user info, compatibility/risks и evidence refs. architecture.md обновлять только по current HEAD source-of-truth.

Preserve: не менять runtime/code/API, не commit/push, не выполнять destructive actions. Не заявлять why без release notes, commit/PR title, diff, tests/docs или local metadata; иначе писать "не установлено локально".

Verify: при stale refs выполнить git fetch upstream --tags --prune. Для entries проверить git show --stat, git show, merge-base containment и git describe --contains. Проверить no release gaps, no missing Evidence refs, no stale architecture paths; выполнить git diff --check и ручной обзор diff.

Done/stop: готово, когда документы согласованы, новые релизы покрыты, gaps перечислены, финальная сводка сообщает диапазон, число release sections, commit/group entries и remaining gaps. Остановиться при недоступных upstream refs, remote publish, destructive actions, web PR metadata need или неоднозначном scope.
```

## Примечания к применению

- Этот prompt не предназначен для первичного создания research-базы.
- Если latest upstream stable уже покрыт, агент должен ничего не дописывать в timeline и вернуть проверочную сводку.
- Если sub-agents или конкретная модель triage недоступны, агент продолжает локальной проверкой и фиксирует это в evidence gaps.
- Для новых prompt-шаблонов добавляй отдельный файл в тематический подкаталог и строку в `docs/fork/prompts/README.md`.
