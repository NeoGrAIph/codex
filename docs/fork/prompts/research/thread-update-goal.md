# Актуализация thread research

## Назначение

Prompt используется, когда `docs/fork/research/thread/` уже существует и нужно поддержать документы в актуальном состоянии после нового upstream stable-релиза Codex.

## Ready-to-paste goal

```text
/goal Destination: в ~/repo/AGENTS/codex-fork актуализировать docs/fork/research/thread/{timeline.md,evidence.md,architecture.md} от текущего documented stable release до latest upstream stable rust-vX.Y.0. Прочитай docs, найди последний покрытый tag и добавь только недостающие stable sections, не пересоздавая историю.

Context: docs-only, русский текст; upstream commit subjects, PR ids, API/type/config/tool/wire names сохранять как есть. Scope строго app-server JSON-RPC namespace thread/*: methods, notifications/events, request/response types, generated schema/TS projections, request processors and app-server README/tests. Core/thread-store/TUI учитывать только как evidence, если diff меняет app-server thread/* contract.

Scope: трогать только thread research docs и prompt index, кроме явной правки ссылки/индекса. Для каждого нового stable release добавить metadata, relevant commits, developer/user info, compatibility/risks и evidence refs. architecture.md обновлять только по current HEAD source-of-truth.

Preserve: не менять runtime/code/API, не commit/push, не выполнять destructive actions. Не включать раннюю историю до rust-v0.86.0 и не смешивать thread research с multi-agents или внутренним session runtime, если это не меняет app-server thread/* surface. Не заявлять why без release notes, commit/PR title, diff, tests/docs или local metadata; иначе писать "не установлено локально".

Verify: при stale refs выполнить git fetch upstream --tags --prune. Для entries проверить git show --stat, git show, merge-base containment и git describe --contains. Проверить no release gaps от последнего documented tag, no missing Evidence refs, no stale architecture paths; выполнить git diff --check -- docs/fork/research/thread docs/fork/prompts и ручной обзор diff.

Done/stop: готово, когда документы согласованы, новые релизы покрыты, gaps перечислены, финальная сводка сообщает диапазон, число release sections, commit/group entries и remaining gaps. Остановиться при недоступных upstream refs, remote publish, destructive actions, web PR metadata need или неоднозначном scope.
```

## Примечания к применению

- Этот prompt не предназначен для первичного создания research-базы.
- Если latest upstream stable уже покрыт, агент должен ничего не дописывать в timeline и вернуть проверочную сводку.
- Если direct `thread/*` diff отсутствует, добавляй no-change section с evidence ID.
- Для новых prompt-шаблонов добавляй отдельный файл в тематический подкаталог и строку в `docs/fork/prompts/README.md`.
