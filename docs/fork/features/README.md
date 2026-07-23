# Fork feature ledger

Этот каталог фиксирует пользовательские возможности, которые были добавлены, перенесены или доработаны в fork-ветках. Основная единица учета здесь - не отдельная ветка и не отдельный commit subject, а пользовательская возможность: что стало доступно пользователю, в каких ветках это реализовывалось, и какими коммитами.

## Методика

- Источники: локальные `fork/*`, `origin/fork/*`, `chrome_plugin`, `origin/chrome_plugin`, а также текущий dirty state как `draft/uncommitted`.
- Upstream release commits и upstream PR commits используются как baseline/evidence и не считаются fork-возможностями сами по себе.
- Если один функционал переносился между release-ветками, он описан в одном feature doc с несколькими ветками в таблице коммитов.
- Если назначение commit не установлено из subject, diff или существующих docs/tests, это помечается как `не установлено локально`.

## Возможности

| Возможность | Пользовательский смысл | Статус | Основные ветки |
| --- | --- | --- | --- |
| [Subagent Workbench / agents overlay](subagent-workbench.md) | TUI-окно для просмотра sub-agents, их статусов и контекста. Польза: легче управлять параллельной работой и понимать, кто чем занят. | Переносилась между release-ветками; native `rust-v0.140.0` покрывает частично | `fork/saw`, `fork/101`, `fork/106`, `fork/107`, `fork/118`, `fork/130`, `chrome_plugin` |
| [Interactive collaboration mode](interactive-collaboration-mode.md) | Режим интерактивного взаимодействия с явной индикацией в TUI. Польза: пользователю проще взаимодействовать с агентом ввиду возможности интерактивного выбора из предложенных вариантов. | Историческая fork-возможность; native `rust-v0.140.0` покрывает частично | `fork/101` |
| [Agent role templates](agent-role-templates.md) | Шаблоны ролей для sub-agents: explorer, worker, reviewer и т.п. Польза: agents стартуют с понятной специализацией и не требуют каждый раз ручного описания роли. | Переносилась между release-ветками; native `rust-v0.140.0` покрывает частично | `fork/saw`, `fork/106`, `fork/107`, `fork/111`, `fork/118`, `fork/130`, `chrome_plugin` |
| [Thread notes](thread-notes.md) | Короткие заметки для thread/sub-agent, сохраняющиеся через resume/restart. Польза: агент-оркестратор может быстро понять назначение и состояние каждого agent без перечитывания transcript. | Переносилась и усиливалась; native `rust-v0.140.0` покрывает частично, но без note surface | `fork/105`, `fork/106`, `fork/107`, `fork/111`, `fork/114`, `fork/116`, `fork/118`, `fork/130`, `chrome_plugin` |
| [Spawn agent cwd](spawn-agent-cwd.md) | Запуск sub-agent в явно выбранном рабочем каталоге. Польза: удобно делегировать задачи по разным проектам/worktrees из одного основного thread, а созданный в другой рабочей папке agent использует нативный функционал в директории, где был создан, что снижает риск изменений в некорректном месте. | Переносилась и уточнялась; native `rust-v0.140.0` покрывает частично | `fork/114`, `fork/118`, `fork/130`, `chrome_plugin` |
| [Subagent policy and ownership](subagent-policy-and-ownership.md) | Ограничения прав sub-agent, allow/deny policies, запрет закрывать чужие branches. Польза: безопаснее multi-agent работа, меньше риска, что agent затронет не свой контекст. | Переносилась между release-ветками; native `rust-v0.140.0` покрывает частично | `fork/105`, `fork/106`, `fork/107`, `fork/111`, `fork/colab-agents`, `fork/multi-agent` |
| [Agent runtime limits](agent-runtime-limits.md) | Увеличенные лимиты depth, max threads и wait timeout. Польза: можно запускать более крупные multi-agent задачи без преждевременных ограничений. | Переносилась между release-ветками; native `rust-v0.140.0` покрывает частично | `fork/multi-agent`, `fork/106`, `fork/107`, `fork/114`, `fork/116`, `fork/130`, `chrome_plugin` |
| [MCP on-demand discovery](mcp-on-demand-discovery.md) | Обнаружение MCP servers по требованию. Польза: MCP servers не нагружают контекст agent, если они не нужны agent. | Историческая fork-возможность; native `rust-v0.140.0` покрывает частично | `fork/106`, `fork/107` |
| [run_skill_script tool](run-skill-script.md) | Tool для запуска scripts из skills через unified exec. Польза: увеличение эффективности и качества работы agent, так как skill может не только описывать действия, но и выполнять подготовленный helper. | Экспериментальная fork-возможность; native `rust-v0.140.0` покрывает частично | `fork/multi-agent` |
| [Provider-aware models and DeepSeek](provider-aware-models-deepseek.md) | Поддержка моделей разных providers, включая DeepSeek через Chat Completions. Польза: пользователь/agent выбирает не только model, но и корректный provider. | Реализовано в исторической fork-ветке; native `rust-v0.140.0` покрывает частично | `fork/130`, `chrome_plugin` |

## Текущий draft/uncommitted state

На момент создания ledger текущая ветка `fork/140` не содержит коммитов поверх `fork/140-upstream`; актуальные изменения находятся в working tree. Они отражены только как draft в соответствующих feature docs, пока не будут закоммичены.
