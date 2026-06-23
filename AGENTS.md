# Rust/codex-rs

In the codex-rs folder where the rust code lives:

- Crate names are prefixed with `codex-`. For example, the `core` folder's crate is named `codex-core`
- When using format! and you can inline variables into {}, always do that.
- Install any commands the repo relies on (for example `just`, `rg`, or `cargo-insta`) if they aren't already available before running instructions here.
- Never add or modify any code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.
  - You operate in a sandbox where `CODEX_SANDBOX_NETWORK_DISABLED=1` will be set whenever you use the `shell` tool. Any existing code that uses `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` was authored with this fact in mind. It is often used to early exit out of tests that the author knew you would not be able to run given your sandbox limitations.
  - Similarly, when you spawn a process using Seatbelt (`/usr/bin/sandbox-exec`), `CODEX_SANDBOX=seatbelt` will be set on the child process. Integration tests that want to run Seatbelt themselves cannot be run under Seatbelt, so checks for `CODEX_SANDBOX=seatbelt` are also often used to early exit out of tests, as appropriate.
- Always collapse if statements per https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
- Always inline format! args when possible per https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args
- Use method references over closures when possible per https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls
- Avoid bool or ambiguous `Option` parameters that force callers to write hard-to-read code such as `foo(false)` or `bar(None)`. Prefer enums, named methods, newtypes, or other idiomatic Rust API shapes when they keep the callsite self-documenting.
- When you cannot make that API change and still need a small positional-literal callsite in Rust, follow the `argument_comment_lint` convention:
  - Use an exact `/*param_name*/` comment before opaque literal arguments such as `None`, booleans, and numeric literals when passing them by position.
  - A method's sole non-self argument is exempt when the method and parameter names match, such as `.enabled(false)` for `fn enabled(&self, enabled: bool)`.
  - Do not add these comments for string or char literals unless the comment adds real clarity; those literals are intentionally exempt from the lint.
  - The parameter name in the comment must exactly match the callee signature.
  - You can run `just argument-comment-lint` to run the lint check locally. This is powered by Bazel, so running it the first time can be slow if Bazel is not warmed up, though incremental invocations should take <15s. Most of the time, it is best to update the PR and let CI take responsibility for checking this (or run it asynchronously in the background after submitting the PR). Note CI checks all three platforms, which the local run does not.
- When possible, make `match` statements exhaustive and avoid wildcard arms.
- Newly added traits should include doc comments that explain their role and how implementations are expected to use them.
- Discourage both `#[async_trait]` and `#[allow(async_fn_in_trait)]` in Rust traits.
  - Prefer native RPITIT trait methods with explicit `Send` bounds on the returned future, as in `3c7f013f9735` / `#16630`.
  - Preferred trait shape:
    `fn foo(&self, ...) -> impl std::future::Future<Output = T> + Send;`
  - Implementations may still use `async fn foo(&self, ...) -> T` when they satisfy that contract.
  - Do not use `#[allow(async_fn_in_trait)]` as a shortcut around spelling the future contract explicitly.
- When writing tests, prefer comparing the equality of entire objects over fields one by one.
- Do not add tests for values that are statically defined.
- Do not add negative tests for logic that was removed.
- Do not add general product or user-facing documentation to the `docs/` folder. The official Codex documentation lives elsewhere. The exception is app-server API documentation, which is covered by the app-server guidance below.
- Prefer private modules and explicitly exported public crate API.
- If you change `ConfigToml` or nested config types, run `just write-config-schema` to update `codex-rs/core/config.schema.json`.
- When working with MCP tool calls, prefer using `codex-rs/codex-mcp/src/mcp_connection_manager.rs` to handle mutation of tools and tool calls. Aim to minimize the footprint of changes and leverage existing abstractions rather than plumbing code through multiple levels of function calls.
- Do not call `reset_client_session` unnecessarily; let the incremental check logic decide whether to reuse the previous request.
- If you change Rust dependencies (`Cargo.toml` or `Cargo.lock`), run `just bazel-lock-update` from the
  repo root to refresh `MODULE.bazel.lock`, and include that lockfile update in the same change.
- After dependency changes, run `just bazel-lock-check` from the repo root so lockfile drift is caught
  locally before CI.
- Bazel does not automatically make source-tree files available to compile-time Rust file access. If
  you add `include_str!`, `include_bytes!`, `sqlx::migrate!`, or similar build-time file or
  directory reads, update the crate's `BUILD.bazel` (`compile_data`, `build_script_data`, or test
  data) or Bazel may fail even when Cargo passes.
- Do not create small helper methods that are referenced only once.
- For tracing async work, instrument the function or method definition with
  `#[tracing::instrument(...)]` instead of attaching spans to futures with
  `.instrument(...)` at call sites. Before adding instrumentation, check whether the callee—or
  the implementation method it immediately delegates to—is already instrumented.
- Avoid large modules:
  - Prefer adding new modules instead of growing existing ones.
  - Target Rust modules under 500 LoC, excluding tests.
  - If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending
    the existing file unless there is a strong documented reason not to.
  - This rule applies especially to high-touch files that already attract unrelated changes, such
    as `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/bottom_pane/chat_composer.rs`,
    `codex-rs/tui/src/bottom_pane/footer.rs`, `codex-rs/tui/src/chatwidget.rs`,
    `codex-rs/tui/src/bottom_pane/mod.rs`, and similarly central orchestration modules.
  - When extracting code from a large module, move the related tests and module/type docs toward
    the new implementation so the invariants stay close to the code that owns them.
  - Avoid adding new standalone methods to `codex-rs/tui/src/chatwidget.rs` unless the change is
    trivial; prefer new modules/files and keep `chatwidget.rs` focused on orchestration.
- When running Rust commands (e.g. `just fix` or `just test`) be patient with the command and never try to kill them using the PID. Rust lock can make the execution slow, this is expected.

Run `just fmt` (in the `codex-rs` directory) automatically after you have finished making code changes anywhere in this repository; do not ask for approval to run it. Additionally, run the tests:

1. Do not run `cargo test` directly. Use `just test` so test execution follows the repo defaults.
2. Run the test for the specific project that was changed. For example, if changes were made in `codex-rs/tui`, run `just test -p codex-tui`.
3. Once those pass, if any changes were made in common, core, or protocol, run the complete test suite with `just test`. Avoid `--all-features` for routine local runs because it expands the build matrix and can significantly increase `target/` disk usage; use it only when you specifically need full feature coverage. project-specific or individual tests can be run without asking the user, but do ask the user before running the complete test suite.

Before finalizing a large change to `codex-rs`, run `just fix -p <project>` (in `codex-rs` directory) to fix any linter issues in the code. Prefer scoping with `-p` to avoid slow workspace‑wide Clippy builds; only run `just fix` without `-p` if you changed shared crates. Do not re-run tests after running `fix` or `fmt`.

## Fork Governance

Для создания новой ветки форка от upstream Codex release используй `docs/fork/release-branching.md`; процедуру создания веток держи там, не дублируй её в этом файле.

### Fork Documentation Language

Fork-specific документация в этом репозитории ведётся на русском языке. Upstream-документацию и upstream-текст сохраняй на исходном языке, если задача явно не требует перевода или fork-specific замены.

### Fork Binary Update Scripts

- Use `scripts/codex-fork-build.sh` from the repository root to build the fork release binary at `codex-rs/target/release/codex` and record the build hash.
- Use `scripts/codex-fork-install-binary.sh` after a successful build to актуализировать the executable `codex` binary resolved from `PATH` with the binary built from this fork.
- Use `scripts/codex-fork-install-binary.sh --dry-run` to verify source and target paths without changing files.
- Use `scripts/codex-fork-sync-runtime.sh` after a successful build when the Codex app managed app-server runtime should pick up the fork binary. This script актуализирует the managed standalone runtime plus `~/.nvm/versions/node/v22.22.0/bin/codex` and `~/.nvm/versions/node/v22.22.0/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex`; it must not stop, kill, start, or restart existing app-server processes.
- Use `codex-fork restore-upstream-runtime` only as an explicit rollback/test action when local app-server runtime targets must be restored to the upstream npm package; pass `--dry-run` to preview and `--skip-npm-install` to reuse the currently installed npm package. This delegates to `scripts/codex-fork-restore-upstream-runtime.sh`, stops existing `codex app-server` processes, runs `npm install -g @openai/codex` unless skipped, copies the upstream npm vendor binary into the managed standalone runtime, the nvm `codex` entrypoint, the npm vendor binary and `codex-rs/target/release/codex`, and does not explicitly start or restart the daemon.
- Keep build and binary replacement as explicit separate steps; do not silently replace the system `codex` binary as part of ordinary tests or feature implementation.

### Parallel Agent Development Workflow

Эти правила применяются, когда работа ведётся через Codex CLI, Codex app, SSH remote project, app-server remote mode или sub-agent workflow и задачу можно распараллелить. Цель: ускорить разработку без конфликтующих правок, потерянных веток, неподтверждённых runtime assumptions, неуправляемого agent fan-out и преждевременного попадания непроверенных изменений в основной checkout.

#### Universal rules

- Держи основной checkout как место планирования, последовательной интеграции, пользовательской приёмки и финальной проверки. Для этого репозитория основной checkout: `~/repo/AGENTS/codex-fork`; если работа идёт через SSH или remote app-server, этот путь относится к filesystem/shell выбранного remote host. В этом workflow "основная ветка" означает основной integration checkout/thread текущей работы, а не обязательно Git branch `main`.
- Для независимых write-heavy задач используй отдельные Git worktrees / рабочие деревья: один task, одно рабочее дерево, одна branch или detached HEAD, один владелец изменений. В Codex CLI это обычно означает заранее создать `git worktree` или запускать `codex` из уже выбранного рабочего дерева; в Codex app можно использовать Worktree mode.
- Для read-only или research-heavy задач используй отдельный background thread, когда нужно сохранить основной thread чистым, дать задаче свежий контекст или продолжать работу асинхронно. Background thread должен иметь явный prompt со scope, запретом/разрешением на edits, expected output и правилом вернуться в основной thread с thread id и кратким summary.
- Используй sub-agents в первую очередь для read-heavy и bounded задач: codebase exploration, risk review, test-log analysis, docs/evidence sweep, API/contract audit. Не поручай нескольким sub-agents параллельно менять одни и те же ownership surfaces.
- Если нужен sub-agent внутри текущего root thread и видимый в потоке этого thread, запускай его через `multi_agent_v1.spawn_agent`; `create_thread`/`fork_thread` оставляй для отдельных Codex threads, которые должны жить вне текущего потока как самостоятельные background/sidebar threads. Для простых одноразовых sub-agent задач вроде `pwd`, короткой shell-команды или точечной проверки не используй full-history fork без необходимости: в V2 предпочитай `fork_turns="none"`, а в V1 не включай `fork_context`. Держи prompt минимальным и явно говори worker-у, что он уже является целевым sub-agent, не должен вызывать `spawn_agent`, `wait_agent`, `interrupt_agent` или другие agent-control tools, а должен выполнить конкретную команду/проверку и вернуть stdout/status.
- Сообщение пользователя в основной/root диалог steer-ится в активный parent turn: оно может завершить `wait_agent` или `sleep` в основном thread раньше срока, но само по себе не доставляет задачу spawned sub-agent-у и не прерывает его. Для доставки child-agent-у используй `send_message` или `followup_task`, для остановки child-agent-а используй `interrupt_agent`, а после прерванного ожидания проверяй `list_agents` или явное sub-agent notification перед выводом о состоянии worker-а.
- Main/root thread остаётся orchestration owner: он формулирует план, разделяет tracks, выбирает execution surface, принимает или отклоняет выводы background threads/sub-agents, запускает аудит, решает конфликты и пишет финальную сводку. Parallel workers должны возвращать file/symbol refs, команды, итоговый diff/patch, проверки и known risks, а не только narrative summary.
- Ограничивай параллельность человеческой review-capacity. Если результат нельзя быстро проверить и смержить, не создавай дополнительные worktrees/sub-agents; сначала заверши или закрой текущие tracks.
- Перед запуском worktree-задач проверь, какие ignored/local/remote-host files нужны для воспроизводимости (`.env`, local config, generated fixtures, tool caches). Используй documented setup, `direnv`, setup scripts или явную инструкцию по подготовке рабочего дерева; не копируй secrets в Git и не полагайся на скрытое состояние основного checkout.
- Не распараллеливай изменение одного source-of-truth contract: protocol/schema/config/runtime ownership, migrations, release branching, generated artifacts и broad refactors сначала требуют единого integration plan и одного владельца merge.
- После каждого параллельного track проверяй `git status`, diff и релевантные tests/checks в его рабочем дереве. Интегрируй feature tracks в основной checkout строго по одному, затем запускай общий verification для surfaces, которые могли регрессировать из-за совмещения результатов.
- Если parallel workflow меняет fork capability, documented contract или app-server/API behavior, сохрани обычные fork gates из этого файла: research baseline, feature/project docs, propagation matrix, compatibility notes, focused tests и `git diff --check`.

#### Parallel feature lifecycle

1. Main/root thread декомпозирует задачу и явно решает, какие feature tracks независимы и могут выполняться параллельно.
2. Каждая parallel-safe feature получает отдельный worker context: Codex app worktree/background thread или CLI `git worktree` + `codex` из этого рабочего дерева.
3. Worker может делать локальные commits в своей track branch/worktree как checkpoints для итераций и review, но не делает push, merge, rebase или cherry-pick в основной checkout. Результат worker-а: итоговый diff/patch или commit range для переноса без коммита, summary, checks, evidence и known risks.
4. До интеграции main/root thread запускает независимый auditor pass. Выбирай project-local auditor roles из `.codex/agents/` по их `description`; `developer_instructions` задаёт базовое поведение роли, но в prompt каждому аудитору всё равно нужно явно передать scope, inspected diff/files, relevant feature contract/docs, relevant research architecture baseline, evidence/checks и конкретный фокус проверки. Если feature добавляет или меняет capability/state/metadata/config/runtime attribute, обязательно включай `propagation-auditor`.
5. Все auditor findings должны быть исправлены worker-ом или явно признаны false-positive главным агентом с обоснованием. Unresolved High/Medium findings блокируют перенос feature в основной checkout.
6. После зелёного аудита main/root thread переносит результат worker-а в основной checkout только через patch/diff application как обычные незакоммиченные изменения; `git cherry-pick --no-commit <base>..<worker-branch>` допустим как удобный способ применить worker commits без создания коммитов в основной ветке. Worker branch/worktree служит источником итогового diff, но worker commits не являются source-of-truth и не переносятся в историю через обычный merge, cherry-pick или rebase.
7. Пользователь и главный агент оценивают интегрированный результат в основном checkout. Молчание пользователя или зелёные проверки не считаются acceptance.
8. Если найдены проблемы, candidate patch откатывается только в пределах diff этой feature; unrelated dirty changes не откатываются. Feedback отправляется worker thread/worktree, и цикл worker -> audit -> serial integration повторяется.
9. Feature считается принятой только после явного подтверждения пользователя. Фактический commit, merge или push выполняется только по отдельному явному запросу.

#### Project-local auditor roles

Локальные auditor roles для этого репозитория живут в `.codex/agents/*.toml`. Codex custom-agent schema требует `name`, `description` и `developer_instructions`: `description` помогает выбрать подходящую роль, `developer_instructions` задаёт её базовый audit contract, а конкретный audit prompt задаёт входные данные и границы текущей проверки. Не выбирай аудитора по filename, nickname или устной договорённости, если это расходится с `description`.

- `architecture-auditor`: architecture fit, research-backed native architecture baseline, native integration points, source-of-truth ownership, module boundaries, upstream-shaped extension points.
- `propagation-auditor`: end-to-end propagation of new or changed capabilities from the research-backed native source of truth through producers, consumers, projections, metadata, permissions/sandbox/tool-access checks, persistence/resume, UI/API/model-visible surfaces, generated artifacts, and verification coverage.
- `contract-auditor`: app-server APIs, CLI/config surfaces, protocol/wire shape, generated schemas, compatibility, documented behavior.
- `security-auditor`: permissions, sandbox behavior, cwd/env handling, MCP/tool exposure, secrets, persistence trust boundaries, unsafe fallback paths.
- `runtime-persistence-auditor`: runtime state, thread/session lifecycle, resume/restart behavior, rollout/session files, migrations, imports, backward compatibility.
- `tests-regression-auditor`: test coverage, regression risk, research-derived upstream paths, snapshot/generated artifact handling, missing negative/compatibility checks.
- `docs-evidence-auditor`: fork documentation, research evidence, research architecture baselines, propagation matrices, verification matrices, status claims, source-backed rationale.
- `integration-auditor`: patch readiness, dirty-worktree isolation, conflict risk, unrelated changes, verification order before serial integration.

`propagation-auditor` отвечает за проверку, что реализация не осталась локальной правкой в одном модуле: новый или изменённый атрибут должен иметь один source of truth, известных producers, всех обязательных consumers, changed projections, intentionally unaffected surfaces и verification для каждой affected surface. Используй его, например, если новая рабочая директория агента должна попасть в agent metadata/listing, permission checks, sandbox/tool access, persistence/resume и UI/API/model-visible surfaces.

Не используй эти роли как замену конкретному audit prompt. При запуске аудитора явно передай inspected diff/files, relevant feature contract/docs, команды или логи проверки, severity scale и правило: High/Medium findings блокируют интеграцию до исправления или documented false-positive решения main/root thread.

Если для feature track существует релевантный `docs/fork/research/*/architecture.md`, используй его как normalized native architecture baseline: source of truth, lifecycle/data flow, producers, consumers, projections, invariants, developer guidance и intentionally separate/adjacent surfaces. `timeline.md` и `evidence.md` остаются provenance/evidence слоем; для проектирования и аудита основной рабочий artifact - текущая architecture map, проверенная против HEAD.

#### Serial integration gate

- Одновременно интегрируй в основной checkout только одну feature track.
- Перед применением worker patch зафиксируй pre-integration state основного checkout через `git status --short` и targeted diff по ожидаемым files/surfaces.
- Перенос из worker branch/worktree в основной checkout выполняй как patch application: `git diff`/`git apply` или `git cherry-pick --no-commit` по точно выбранному commit range допустимы, а обычные merge/cherry-pick/rebase worker branch запрещены. Цель - получить один проверенный интеграционный diff без лишних промежуточных worker commits в основной истории.
- Если worker patch конфликтует с текущим основным checkout или unrelated dirty changes, не выполняй неоднозначный ручной merge; верни feature worker-у с конфликтом и текущим контекстом.
- После применения patch проверь diff этой feature отдельно от уже существующих изменений, затем запусти релевантные checks и только потом переходи к пользовательской приёмке.

#### Tool selection

- Текущий thread: используй для коротких правок, финальной интеграции, user-facing decisions и проверок, которые должны учитывать полный текущий контекст.
- Background project thread: используй для независимого read-only исследования, документационного sweep, repo-map, long-running analysis или отдельного вопроса, который не должен засорять основной контекст. Не разрешай edits без явного worktree/checkout ownership.
- Worktree/background thread: используй для независимых code changes, spikes, upgrade attempts и конфликтных экспериментов. Не мержи результат, пока основной thread не прочитал summary, diff и проверки.
- Sub-agents: используй для параллельных проверок внутри одного orchestration turn, когда нужны независимые роли или быстрый audit. После `wait` основной thread обязан синтезировать findings, отбросить неподтверждённые выводы и зафиксировать, какие замечания блокируют интеграцию.
- Automations: используй только для recurring/background контроля с durable prompt после ручной проверки prompt в обычном thread. Не используй automation как замену первичному root-cause анализу или review.

#### Codex app specifics

- В Codex app для независимых write-heavy задач предпочитай Worktree mode, чтобы рабочее дерево было создано через нативный app flow. Если потом нужен app review/commit/push/Handoff, не создавай ad hoc `git worktree add` внутри уже активного app thread вместо app-managed worktree.
- Для managed background threads используй native thread-management tools: `create_thread` с `worktree` для параллельной write-heavy feature work, `create_thread` с `local` или read-only prompt для background research, `set_thread_title` для понятного имени, `set_thread_pinned` только для активных важных tracks, `read_thread` для проверки результата без переключения контекста, `send_message_to_thread` для точечного follow-up, `fork_thread` когда нужно сохранить историю текущего thread, и `handoff_thread` только когда нужно перенести другой thread между checkout/worktree/host. Не создавай background thread без явного owner, scope и stop condition.
- Для параллельной write-heavy feature work в Codex app, где worker должен наследовать контекст текущего thread, используй branch-backed three-phase flow: сначала создай отдельную track branch от текущей основной ветки форка (`git branch codex/<main-fork-branch-normalized>/<track-name> <main-fork-branch>`), затем вызови `fork_thread` с `environment.type = "worktree"` и `startingState: { type: "branch", branchName: "<track-branch>" }`, дождись появления child thread через `list_threads`, проверь его `cwd`, переключи checkout на эту же branch через `git -C <cwd> switch <track-branch>`, затем задай понятный `set_thread_title`, при необходимости `set_thread_pinned`, и только после этого отправляй worker prompt через `send_message_to_thread`.
- Имена track branches для Codex app worktrees должны начинаться с нормализованного имени основной ветки форка: для `fork/141` используй `codex/fork-141/<track-name>`. Не используй форму `fork/141/<track-name>`, потому что она конфликтует с существующей Git ref `fork/141`.
- Для app-managed branch-backed worktree detached HEAD является промежуточным состоянием, которое нужно устранить до старта worker-а, а не допустимым режимом основной разработки: перед запуском worker-а проверь `git -C <cwd> status -sb` и `git -C <cwd> branch --show-current`; продолжай только если статус показывает ожидаемую `<track-branch>`, а `branch --show-current` возвращает эту branch. Если checkout остался `## HEAD (no branch)`, выполни `git -C <cwd> switch <track-branch>` и повтори проверку; если переключение невозможно, не начинай write-heavy работу и верни проблему main/root thread.
- `startingState: { type: "working-tree" }` используй только когда worker должен унаследовать текущие незакоммиченные изменения для отдельной проверки или доработки; для основной независимой feature-разработки предпочитай заранее созданную track branch от текущей основной ветки форка без случайного копирования unrelated dirty state.
- Same-directory forks не используй для основной write-heavy параллельной разработки; они допустимы для коротких context forks, read-only проверок, явных runtime/tool тестов, а также после выполнения основной разработки, когда нужен отдельный контекст для финальной проверки, отладки или точечной доработки уже подготовленного diff.
- На SSH remote project Codex app Worktree mode должен создавать или использовать рабочее дерево на remote host, а не на машине с UI. Проверяй это по пути рабочего дерева, shell prompt и `git worktree list` на remote host.
- `.worktreeinclude` и автоматическое копирование ignored files документированы для local Codex-managed worktrees. Для remote worktrees и Git worktrees, созданных вручную из CLI, не предполагай такое же поведение без проверки setup на конкретном host.
- Для recurring/background задач используй automations только с durable prompt, проверенным вручную в обычном thread. В Git repo предпочитай dedicated background worktree, если automation может менять файлы.

### Fork Change Integration Agreement

Форк расширяет upstream Codex, а не заменяет его поведение. Любое fork-изменение должно сначала использовать нативные механизмы Codex:

- existing config/profile surfaces;
- permission and sandbox flows;
- app-server protocol;
- TUI flows;
- MCP/tool plumbing;
- extension/hooks/plugin APIs;
- runtime state;
- persistence;
- diagnostics.

В этом форке "нативный функционал" означает существующую upstream-архитектуру, источники истины, контракты и точки расширения, через которые Codex уже решает похожую задачу. Если для задачи есть штатный upstream-механизм, fork-изменение должно встраиваться в него, а не создавать:

- параллельную конфигурацию;
- отдельный обходной runtime path;
- дублирующее хранилище состояния;
- собственную модель разрешений;
- альтернативную protocol surface.

Fork-изменения по умолчанию должны быть аддитивными. Сохраняй текущее upstream-поведение, если явно описанный fork contract не требует документированного divergence, для:

- существующих пользователей;
- конфигов;
- сессий;
- app-server clients;
- CLI flags;
- tool calls;
- persisted data.

Не заменяй нативную подсистему только потому, что так проще реализовать fork-фичу. Предпочитай минимальные integration points, feature-specific adapters и существующие extension points широким переписываниям. Если нативной точки расширения недостаточно, сначала опиши недостающий контракт, добавь минимальную upstream-shaped точку расширения и явно покрой compatibility behavior.

Silent fallback для fork-поведения запрещён. Если нативный контракт не позволяет безопасно реализовать поведение, нужно fail fast с контролируемой диагностикой или требовать явно документированный migration/rebuild/restart path.

Каждое material fork-изменение должно называть свою нативную integration point в описании изменения или implementation notes и включать проверку нового fork-поведения плюс upstream-пути, который вероятнее всего может регрессировать.

### Cross-Surface Native Propagation

Fork-фичи должны добавляться в нативный source of truth для соответствующей capability, а не только в первый UI/API/runtime-модуль, где эта capability понадобилась. Перед реализацией нужно определить все существующие native consumers этого источника и owning source of truth:

- registry;
- manager;
- config type;
- runtime state;
- protocol type;
- другой upstream-owned источник истины.

Если fork-фича расширяет capability, которая уже проецируется в другие модули, новое поведение по умолчанию должно проходить через тот же projection path:

- provider/model extension: через provider-aware catalog и thread runtime state до TUI/app-server UI;
- tool extension: через MCP/tool registry и permission/sandbox projections;
- config extension: через `ConfigToml`, schema и runtime config loader, а не через per-client flags.

Feature dossier должен содержать propagation matrix:

- source of truth;
- native producers;
- native consumers;
- changed projections;
- intentionally unaffected surfaces;
- required generated artifacts;
- verification для каждой affected surface.

Не добавляй следующие anti-patterns, если существующий registry, event, protocol, config или manager может нативно пронести capability:

- hardcoded feature lists;
- duplicated enums;
- parallel caches;
- one-off UI/API switches;
- прямые module-to-module calls.

Если нативный путь не может безопасно пронести поведение, добавь минимальную upstream-shaped extension point и задокументируй причину.

Completion требует доказать не только локальное поведение, но и propagation behavior:

- хотя бы один тест или manual verification должен показать, что feature видна или используется через каждую существующую native surface, которая должна её унаследовать;
- намеренно не затронутые surfaces нужно явно указать в документации.

### Fork Feature Delivery Contract

- Treat each fork feature as a documented contract, not as an implicit code-only customization.
- `AGENTS.md` is the policy source of truth for fork docs until a release-specific `docs/fork/CONTRIBUTING.md` is deliberately restored with the relevant porting work.
- New fork Markdown documentation must be written in Russian unless a file is intentionally preserving upstream English text or external quoted material.
- Every new fork feature or material fork behavior change must ship with `docs/fork/features/<code-name>.md`.
- Every new fork feature or material fork behavior change must ship with `docs/fork/projects/<code-name>/` as the implementation dossier.
- Every fork implementation aligned to a release baseline must also maintain `docs/fork/research/<release>/` as an active research package.
- If API, wire shape, config semantics, TUI behavior, or operational behavior changes, update the relevant user/developer docs in the same change set.
- Do not add database columns or migrations for fork features unless the feature contract proves that session/thread JSON metadata, rollout/session files, or another existing backward-compatible persisted structure cannot represent the state safely.
- Keep old sessions and persisted data readable. Missing new fields in older persisted data must have documented behavior.
- Do not include unrelated diffs, generated noise, build hashes, lockfile changes, or docs for unrelated features. Keep `Cargo.lock` untouched unless dependency changes explicitly require it.

Minimum requirements for `docs/fork/features/<code-name>.md`:

- Feature passport: code name, status, goal, scope in/out.
- User contract: exact behavior, transitions, empty/error states, and critical strings where wording matters.
- Integration and compatibility notes: what remains upstream behavior, what is fork-specific, and what intentionally diverges.
- Propagation matrix: native source of truth, producers, consumers, projections, intentionally unaffected surfaces, generated artifacts, and verification.
- Verification matrix: required commands/tests and the surfaces they validate.
- Doc changelog: concise dated entries when contract or implementation expectations change.

Minimum requirements for `docs/fork/projects/<code-name>/`:

- `README.md`: entry point, current status, canonical links, and a compact map of the implementation surfaces.
- `design.md`: canonical state, native architecture baseline from relevant research docs, native propagation path, projections, data flow, invariants, and intentional implementation tradeoffs.
- `verification.md`: scenario-focused test matrix, validation commands, and known coverage gaps.

Minimum requirements for `docs/fork/research/<release>/`:

- baseline release/tag and commit reference;
- gap analysis between upstream baseline and fork behavior;
- notes for risky integration points and conflict-prone source-of-truth files;
- release-specific verification notes for the changed contract.

### Quality-First Fork Feature Workflow

Use this workflow for every fork feature or material fork behavior change:

1. Research the current upstream baseline and integration points before changing source files. Select relevant `docs/fork/research/*/architecture.md` files and treat them as the normalized native architecture baseline; if no relevant or current research architecture map exists, verify the native architecture from HEAD before designing the feature.
2. Define the contract:
   - goal;
   - scope in/out;
   - native architecture baseline used;
   - canonical source of truth;
   - affected subsystems;
   - compatibility expectations;
   - permissions/security implications;
   - persistence behavior;
   - failure modes;
   - no-fallback rules.
3. Build an integration and propagation map from the research-derived native architecture baseline that covers:
   - native source of truth;
   - producers and consumers;
   - parsing/schema and generated artifacts;
   - config loading and runtime state;
   - permissions/sandbox and environment selection;
   - persistence, resume, and restart behavior;
   - events, API, UI, and model-visible projections;
   - tests, docs, and intentionally unaffected surfaces.
4. Create or update the verification matrix before finalizing the implementation.
5. Implement in small coherent slices and keep each changed line traceable to the feature contract.
6. Run focused tests/checks for the feature and the affected integration points.
7. Use sub-agents for independent review. Prefer reusing an existing free agent with the appropriate or adjacent role/context before spawning a new one.
8. Run an architecture/contract audit before completion. Also run a permissions/security/runtime audit when the feature touches cwd, execution, sandbox, config, persistence, or resume behavior.
9. Fix all High and Medium audit findings before completing the feature. A High or Medium finding may remain only with explicit owner approval, and the feature is blocked until that approval and the deferred risk are documented in the feature dossier.
10. Commit only the implemented, audited, documented, and verified feature before starting the next feature.

Completion requires evidence that:

- new behavior is tested;
- important upstream behavior that could regress is covered or explicitly verified;
- security boundaries include positive and negative assertions;
- standard access policies still work without widened permissions;
- runtime/config/persistence/resume state remains consistent;
- docs match the implemented contract;
- implementation matches the relevant native architecture baseline, or any intentional architecture delta is documented and the research map is updated when it became stale;
- `git diff --check` passes;
- the relevant project checks from this file pass.

### Fork Feature Integration Rules

- Use an `Upstream-first` approach by default: prefer the target release's architecture, control flow, and source-of-truth patterns as the baseline. When a relevant `docs/fork/research/*/architecture.md` exists, use it as the normalized map of that upstream-native architecture rather than reasoning from historical commits directly.
- Prefer additive and localized fork changes when that keeps maintenance and reasoning simpler.
- If a fork feature must modify an existing upstream-owned path directly, document the tradeoff in the feature doc instead of treating the divergence as self-evident.
- Preserve existing upstream behavior by default. If existing behavior must change, document that change explicitly as part of the fork contract.
- Do not introduce silent fallback behavior for fork logic. Any fallback must be explicit, controlled, observable to the caller or operator, documented in the feature contract, and covered by verification.
- When choosing between a literal backport and an upstream-shaped adaptation, prefer the upstream-shaped implementation unless it would break a documented fork contract.

### Compatibility And Regression Gates

- Every fork feature change must include evidence for the new behavior and for the most important existing path that could regress because of the change.
- If protocol, schema, state, or config contracts change, regenerate the canonical artifacts in the same change set and keep generated diffs minimal.
- If a regenerated artifact differs only by formatting, ordering noise, or trailing whitespace without a real contract change, revert it and keep only meaningful generated deltas.
- If fork work changes app-server protocol or behavior, document the affected client surfaces, stable vs experimental classification, and the compatibility matrix between separately distributed client versions and app-server/schema versions.
- If fork work regenerates `insta` snapshots, accept them through the normal `cargo insta review` / `cargo insta accept` flow. Do not manually rename `.snap.new` files into `.snap`, and do not keep review-only metadata such as `assertion_line` in final accepted snapshots.
- If a fork feature extends a native capability, verify every existing native surface that should inherit it; do not validate only the module where the change was first implemented.
- Do not treat "tests passed" as sufficient on its own; ensure the feature docs and affected public docs reflect the implemented contract.
- Before finalizing, confirm that the fork change still has one clear source of truth, one baseline release context, and one documented explanation for any intentional upstream divergence.

### Porting Decision Record

- Before implementing or porting a fork feature, record the upstream baseline and the expected fork gap in `docs/fork/research/<release>/`.
- Before changing source-of-truth files, decide whether the target behavior is a literal port or an upstream-shaped adaptation, and capture that choice in the feature/research docs.
- After implementation, verify contract surfaces explicitly: protocol/wire behavior, runtime behavior, UI/API text, generated artifacts, and persistence/config semantics when applicable.
- If the final implementation differs materially from the historical fork commit because of upstream adaptation, document the reason so future ports do not re-open the same decision.

## The `codex-core` crate

Over time, the `codex-core` crate (defined in `codex-rs/core/`) has become bloated because it is the largest crate, so it is often easier to add something new to `codex-core` rather than refactor out the library code you need so your new code neither takes a dependency on, nor contributes to the size of, `codex-core`.

To that end: **resist adding code to codex-core**!

Particularly when introducing a new concept/feature/API, before adding to `codex-core`, consider whether:

- There is an existing crate other than `codex-core` that is an appropriate place for your new code to live.
- It is time to introduce a new crate to the Cargo workspace for your new functionality. Refactor existing code as necessary to make this happen.

Likewise, when reviewing code, do not hesitate to push back on PRs that would unnecessarily add code to `codex-core`.

## Code Review Rules

### Crate API surface

Keep crate API surfaces as small as possible. Avoid proliferating test-only helpers.

### Model visible context

Codex maintains a context (history of messages) that is sent to the model in inference requests.

1. No history rewrite - the context must be built up incrementally.
2. Avoid frequent changes to context that cause cache misses.
3. No unbounded items - everything injected in the model context must have a bounded size and a hard cap.
4. No items larger than 10K tokens.
5. Highlight new individual items that can cross >1k tokens as P0. These need an additional manual review.
6. All injected fragments must be defined as structs in `core/context` and implement ContextualUserFragment trait

### Breaking changes

Search for breaking changes in external integration surfaces:

- app-server APIs
- raw response item events (`rawResponseItem/*`), even while experimental
- CLI parameters
- configuration loading
- resuming sessions from existing rollouts

### Test authoring guidance

For agent changes prefer integration tests over unit tests. Integration tests are under `core/suite` and use `test_codex` to set up a test instance of codex.

Features that change the agent logic MUST add an integration test:

- Provide a list of major logic changes and user-facing behaviors that need to be tested.

If unit tests are needed, put them in a dedicated test file (\*\_tests.rs).
Avoid test-only functions in the main implementation.

Check whether there are existing helpers to make tests more streamlined and readable.

### Change size guidance (800 lines)

Unless the change is mechanical the total number of changed lines should not exceed 800 lines.
For complex logic changes the size should be under 500 lines.

If the change is larger, explore whether it can be split into reviewable stages and identify the smallest coherent stage to land first.
Base the staging suggestion on the actual diff, dependencies, and affected call sites.

## TUI style conventions

See `codex-rs/tui/styles.md`.

## TUI code conventions

- Use concise styling helpers from ratatui’s Stylize trait.
  - Basic spans: use "text".into()
  - Styled spans: use "text".red(), "text".green(), "text".magenta(), "text".dim(), etc.
  - Prefer these over constructing styles with `Span::styled` and `Style` directly.
  - Example: patch summary file lines
    - Desired: vec!["  └ ".into(), "M".red(), " ".dim(), "tui/src/app.rs".dim()]

### TUI Styling (ratatui)

- Prefer Stylize helpers: use "text".dim(), .bold(), .cyan(), .italic(), .underlined() instead of manual Style where possible.
- Prefer simple conversions: use "text".into() for spans and vec![…].into() for lines; when inference is ambiguous (e.g., Paragraph::new/Cell::from), use Line::from(spans) or Span::from(text).
- Computed styles: if the Style is computed at runtime, using `Span::styled` is OK (`Span::from(text).set_style(style)` is also acceptable).
- Avoid hardcoded white: do not use `.white()`; prefer the default foreground (no color).
- Chaining: combine helpers by chaining for readability (e.g., url.cyan().underlined()).
- Single items: prefer "text".into(); use Line::from(text) or Span::from(text) only when the target type isn’t obvious from context, or when using .into() would require extra type annotations.
- Building lines: use vec![…].into() to construct a Line when the target type is obvious and no extra type annotations are needed; otherwise use Line::from(vec![…]).
- Avoid churn: don’t refactor between equivalent forms (Span::styled ↔ set_style, Line::from ↔ .into()) without a clear readability or functional gain; follow file‑local conventions and do not introduce type annotations solely to satisfy .into().
- Compactness: prefer the form that stays on one line after rustfmt; if only one of Line::from(vec![…]) or vec![…].into() avoids wrapping, choose that. If both wrap, pick the one with fewer wrapped lines.

### Text wrapping

- Always use textwrap::wrap to wrap plain strings.
- If you have a ratatui Line and you want to wrap it, use the helpers in tui/src/wrapping.rs, e.g. word_wrap_lines / word_wrap_line.
- If you need to indent wrapped lines, use the initial_indent / subsequent_indent options from RtOptions if you can, rather than writing custom logic.
- If you have a list of lines and you need to prefix them all with some prefix (optionally different on the first vs subsequent lines), use the `prefix_lines` helper from line_utils.

## Tests

### Test module organization

- When adding a new test module, define its contents in a separate sibling file rather than inline in the implementation file.
- Use an explicit `#[path = "..._tests.rs"]` attribute so the test filename is descriptive and easy to locate:

  ```rust
  #[cfg(test)]
  #[path = "parser_tests.rs"]
  mod tests;
  ```

- This applies only when introducing a new test module. Do not move or rewrite existing inline `#[cfg(test)] mod tests { ... }` modules solely to follow this convention.

### Snapshot tests

This repo uses snapshot tests (via `insta`), especially in `codex-rs/tui`, to validate rendered output.

**Requirement:** any change that affects user-visible UI (including adding new UI) must include
corresponding `insta` snapshot coverage (add a new snapshot test if one doesn't exist yet, or
update the existing snapshot). Review and accept snapshot updates as part of the PR so UI impact
is easy to review and future diffs stay visual.

When UI or text output changes intentionally, update the snapshots as follows:

- Run tests to generate any updated snapshots:
  - `just test -p codex-tui`
- Check what’s pending:
  - `cargo insta pending-snapshots -p codex-tui`
- Review changes by reading the generated `*.snap.new` files directly in the repo, or preview a specific file:
  - `cargo insta show -p codex-tui path/to/file.snap.new`
- Only if you intend to accept all new snapshots in this crate, run:
  - `cargo insta accept -p codex-tui`

If you don’t have the tool:

- `cargo install --locked cargo-insta`

### Benchmarks

cargo benchmarks can be run with `just bench`, use the divan crate to write new ones.

Use `just bench-smoke` to dry-run the benchmark for a single iteration to ensure it works.

### Test assertions

- Tests should use pretty_assertions::assert_eq for clearer diffs. Import this at the top of the test module if it isn't already.
- Prefer deep equals comparisons whenever possible. Perform `assert_eq!()` on entire objects, rather than individual fields.
- Avoid mutating process environment in tests; prefer passing environment-derived flags or dependencies from above.

### Spawning workspace binaries in tests (Cargo vs Bazel)

- Prefer `codex_utils_cargo_bin::cargo_bin("...")` over `assert_cmd::Command::cargo_bin(...)` or `escargot` when tests need to spawn first-party binaries.
  - Under Bazel, binaries and resources may live under runfiles; use `codex_utils_cargo_bin::cargo_bin` to resolve absolute paths that remain stable after `chdir`.
- When locating fixture files or test resources under Bazel, avoid `env!("CARGO_MANIFEST_DIR")`. Prefer `codex_utils_cargo_bin::find_resource!` so paths resolve correctly under both Cargo and Bazel runfiles.

### Integration tests (core)

- Prefer the utilities in `core_test_support::responses` when writing end-to-end Codex tests.

- All `mount_sse*` helpers return a `ResponseMock`; hold onto it so you can assert against outbound `/responses` POST bodies.
- Use `ResponseMock::single_request()` when a test should only issue one POST, or `ResponseMock::requests()` to inspect every captured `ResponsesRequest`.
- `ResponsesRequest` exposes helpers (`body_json`, `input`, `function_call_output`, `custom_tool_call_output`, `call_output`, `header`, `path`, `query_param`) so assertions can target structured payloads instead of manual JSON digging.
- Build SSE payloads with the provided `ev_*` constructors and the `sse(...)`.
- Prefer `wait_for_event` over `wait_for_event_with_timeout`.
- Prefer `mount_sse_once` over `mount_sse_once_match` or `mount_sse_sequence`

- Typical pattern:

  ```rust
  let mock = responses::mount_sse_once(&server, responses::sse(vec![
      responses::ev_response_created("resp-1"),
      responses::ev_function_call(call_id, "shell", &serde_json::to_string(&args)?),
      responses::ev_completed("resp-1"),
  ])).await;

  codex.submit(Op::UserTurn { ... }).await?;

  // Assert request body if needed.
  let request = mock.single_request();
  // assert using request.function_call_output(call_id) or request.json_body() or other helpers.
  ```

## App-server API Development Best Practices

These guidelines apply to app-server protocol work in `codex-rs`, especially:

- `app-server-protocol/src/protocol/common.rs`
- `app-server-protocol/src/protocol/v2.rs`
- `app-server/README.md`

### Core Rules

- All active API development should happen in app-server v2. Do not add new API surface area to v1.
- Follow payload naming consistently:
  `*Params` for request payloads, `*Response` for responses, and `*Notification` for notifications.
- Expose RPC methods as `<resource>/<method>` and keep `<resource>` singular (for example, `thread/read`, `app/list`).
- Always expose fields as camelCase on the wire with `#[serde(rename_all = "camelCase")]` unless a tagged union or explicit compatibility requirement needs a targeted rename.
- Always expose string enum values as camelCase on the wire with matching serde and TS `rename_all = "camelCase"` annotations unless an explicit compatibility requirement needs targeted renames.
- Exception: config RPC payloads are expected to use snake_case to mirror config.toml keys (see the config read/write/list APIs in `app-server-protocol/src/protocol/v2.rs`).
- Always set `#[ts(export_to = "v2/")]` on v2 request/response/notification types so generated TypeScript lands in the correct namespace.
- Never use `#[serde(skip_serializing_if = "Option::is_none")]` for v2 API payload fields.
  Exception: client->server requests that intentionally have no params may use:
  `params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>`.
- Keep Rust and TS wire renames aligned. If a field or variant uses `#[serde(rename = "...")]`, add matching `#[ts(rename = "...")]`.
- For discriminated unions, use explicit tagging in both serializers:
  `#[serde(tag = "type", ...)]` and `#[ts(tag = "type", ...)]`.
- Prefer plain `String` IDs at the API boundary (do UUID parsing/conversion internally if needed).
- Timestamps should be integer Unix seconds (`i64`) and named `*_at` (for example, `created_at`, `updated_at`, `resets_at`).
- For experimental API surface area:
  use `#[experimental("method/or/field")]`, derive `ExperimentalApi` when field-level gating is needed, and use `inspect_params: true` in `common.rs` when only some fields of a method are experimental.

### Client->server request payloads (`*Params`)

- Every optional field must be annotated with `#[ts(optional = nullable)]`. Do not use `#[ts(optional = nullable)]` outside client->server request payloads (`*Params`).
- Optional collection fields (for example `Vec`, `HashMap`) must use `Option<...>` + `#[ts(optional = nullable)]`. Do not use `#[serde(default)]` to model optional collections, and do not use `skip_serializing_if` on v2 payload fields.
- When you want omission to mean `false` for boolean fields, use `#[serde(default, skip_serializing_if = "std::ops::Not::not")] pub field: bool` over `Option<bool>`.
- For new list methods, implement cursor pagination by default:
  request fields `pub cursor: Option<String>` and `pub limit: Option<u32>`,
  response fields `pub data: Vec<...>` and `pub next_cursor: Option<String>`.

### Development Workflow

- Update app-server docs/examples when API behavior changes (at minimum `app-server/README.md`).
- Regenerate schema fixtures when API shapes change:
  `just write-app-server-schema`
  (and `just write-app-server-schema --experimental` when experimental API fixtures are affected).
- Validate with `just test -p codex-app-server-protocol`.
- Avoid boilerplate tests that only assert experimental field markers for individual
  request fields in `common.rs`; rely on schema generation/tests and behavioral coverage instead.

## Python Development Best Practices

### Ignore Python 2 compatibility

This project uses Python 3+. You should not use the `__future__` module.

If you need to worry about feature compatibility between different 3.xx point releases, check the
closest `pyproject.toml`'s `requires-python` field to see what minimum runtime version is supported.

## Platform Support

Tests and features must support Linux, macOS and Windows unless feature is explicitly OS-specific.
