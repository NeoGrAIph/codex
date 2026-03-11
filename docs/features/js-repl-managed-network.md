# js_repl Managed Network

## Feature passport

- Code name: `js_repl Managed Network`.
- Status: `implemented`.
- Scope in:
  - `codex-rs/core/src/tools/js_repl/mod.rs`
  - `codex-rs/core/tests/suite/js_repl.rs`
- Scope out:
  - transport changes in `camo-fleet`;
  - public API/schema changes;
  - automatic runtime proxy bootstrap for test harnesses that do not attach `turn.network`.

## Goal

Сделать так, чтобы `js_repl` стартовал Node kernel с тем же managed-network runtime contract, что и обычные `exec`-пути:

- использовать runtime proxy из `TurnContext.network`;
- прокидывать proxy env в kernel process;
- передавать runtime proxy в sandbox transform;
- fail-fast, если managed network policy обязательна, а runtime proxy не attached.

## User-visible behavior

### Positive path

Если в turn присутствует runtime managed proxy (`turn.network`), `js_repl`:

- получает `HTTP_PROXY` / `HTTPS_PROXY`;
- получает `WS_PROXY` / `WSS_PROXY`;
- получает `ALL_PROXY` / `NO_PROXY`;
- стартует kernel под platform sandbox с `enforce_managed_network = true`.

Это позволяет JS code внутри `js_repl` выполнять сетевые операции и использовать Playwright `firefox.connect(...)` к `wss`-endpoint'ам в managed-network сессиях.

### Failure path

Если managed-network requirements активны, но runtime proxy не attached, `js_repl` не должен деградировать в поздние sandbox/network ошибки (`ENOTFOUND`, `EPERM`, и т.п.).

Вместо этого tool call завершается сразу с явной диагностикой:

`js_repl managed network is required for this session, but no runtime network proxy is attached`

## Internal design notes

- `JsReplManager::start_kernel(...)` больше не определяет managed-network поведение только по `requirements_toml().network`.
- Подготовка kernel launch разбита на два шага:
  - `build_kernel_command_spec(...)` собирает `CommandSpec` и proxy env;
  - `prepare_kernel_exec_request(...)` выполняет sandbox transform с тем же runtime proxy.
- Канонический источник runtime network context для `js_repl`: `TurnContext.network`.
- Политика остаётся fail-closed:
  - managed network без runtime proxy считается ошибкой конфигурации/runtime plumbing;
  - silent fallback к unrestricted network не допускается.

## Validation

Минимальная проверка:

- `cargo test -p codex-core js_repl_prepare_kernel_exec_request -- --nocapture`
- `cargo test -p codex-core --test all suite::js_repl:: -- --nocapture`

Ручной smoke test после пересборки runtime:

- внутри `js_repl` выполнить `dns.lookup(...)` до целевого hostname;
- внутри `js_repl` выполнить `playwright.firefox.connect("wss://...")`;
- подтвердить, что чтение `page.url()` / `page.title()` работает без `ENOTFOUND` и без `EPERM`.

## Related features

- `docs/features/saw.md`
