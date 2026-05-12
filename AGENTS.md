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
- When making a change that adds or changes an API, ensure that the documentation in the `docs/` folder is up to date if applicable.
- Prefer private modules and explicitly exported public crate API.
- If you change `ConfigToml` or nested config types, run `just write-config-schema` to update `codex-rs/core/config.schema.json`.
- When working with MCP tool calls, prefer using `codex-rs/codex-mcp/src/mcp_connection_manager.rs` to handle mutation of tools and tool calls. Aim to minimize the footprint of changes and leverage existing abstractions rather than plumbing code through multiple levels of function calls.
- If you change Rust dependencies (`Cargo.toml` or `Cargo.lock`), run `just bazel-lock-update` from the
  repo root to refresh `MODULE.bazel.lock`, and include that lockfile update in the same change.
- After dependency changes, run `just bazel-lock-check` from the repo root so lockfile drift is caught
  locally before CI.
- Bazel does not automatically make source-tree files available to compile-time Rust file access. If
  you add `include_str!`, `include_bytes!`, `sqlx::migrate!`, or similar build-time file or
  directory reads, update the crate's `BUILD.bazel` (`compile_data`, `build_script_data`, or test
  data) or Bazel may fail even when Cargo passes.
- Do not create small helper methods that are referenced only once.
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
- When running Rust commands (e.g. `just fix` or `cargo test`) be patient with the command and never try to kill them using the PID. Rust lock can make the execution slow, this is expected.

## Fork Feature Delivery Contract

- Treat each fork feature as a documented contract, not as an implicit code-only customization.
- Use `docs/fork/CONTRIBUTING.md` as the practical authoring guide for fork docs; `AGENTS.md`
  remains the policy source of truth.
- Every new fork feature or material fork behavior change must ship with `docs/fork/features/<code-name>.md`.
- Every new fork feature or material fork behavior change must ship with
  `docs/fork/projects/<code-name>/` as the implementation dossier.
- Every fork implementation aligned to a release baseline must also maintain `docs/fork/research/<release>/` as an active research package.
- If API, wire shape, config semantics, TUI behavior, or operational behavior changes, update the relevant user/developer docs in the same change set.
- Do not add database columns or migrations for fork features unless the feature contract proves that
  session/thread JSON metadata, rollout/session files, or another existing backward-compatible
  persisted structure cannot represent the state safely.
- Keep old sessions and persisted data readable. Missing new fields in older persisted data must have
  documented behavior.
- Do not include unrelated diffs, generated noise, build hashes, lockfile changes, or docs for
  unrelated features. Keep `Cargo.lock` untouched unless dependency changes explicitly require it.

Minimum requirements for `docs/fork/features/<code-name>.md`:

- Feature passport: code name, status, goal, scope in/out.
- User contract: exact behavior, transitions, empty/error states, and critical strings where wording matters.
- Integration and compatibility notes: what remains upstream behavior, what is fork-specific, and what intentionally diverges.
- Verification matrix: required commands/tests and the surfaces they validate.
- Doc changelog: concise dated entries when contract or implementation expectations change.

Minimum requirements for `docs/fork/projects/<code-name>/`:

- `README.md`: entry point, current status, canonical links, and a compact map of the implementation surfaces.
- `design.md`: canonical state, projections, data flow, invariants, and intentional implementation tradeoffs.
- `verification.md`: scenario-focused test matrix, validation commands, and known coverage gaps.

Minimum requirements for `docs/fork/research/<release>/`:

- baseline release/tag and commit reference;
- gap analysis between upstream baseline and fork behavior;
- notes for risky integration points and conflict-prone source-of-truth files;
- release-specific verification notes for the changed contract.

## Quality-First Fork Feature Workflow

Use this workflow for every fork feature or material fork behavior change:

1. Research the current upstream baseline and integration points before changing source files.
2. Define the contract: goal, scope in/out, canonical source of truth, affected subsystems,
   compatibility expectations, permissions/security implications, persistence behavior, failure
   modes, and no-fallback rules.
3. Build an integration map covering parsing/schema, config loading, runtime state,
   permissions/sandbox, environment selection, persistence, resume/restart, events/API/UI
   projections, tests, and docs.
4. Create or update the verification matrix before finalizing the implementation.
5. Implement in small coherent slices and keep each changed line traceable to the feature contract.
6. Run focused tests/checks for the feature and the affected integration points.
7. Use sub-agents for independent review. Prefer reusing an existing free agent
   with the appropriate or adjacent role/context before spawning a new one.
8. Run an architecture/contract audit before completion. Also run a permissions/security/runtime
   audit when the feature touches cwd, execution, sandbox, config, persistence, or resume behavior.
9. Fix all High and Medium audit findings before completing the feature. A High or Medium finding may
   remain only with explicit owner approval, and the feature is blocked until that approval and the
   deferred risk are documented in the feature dossier.
10. Commit only the implemented, audited, documented, and verified feature before starting the next
    feature.

Completion requires evidence that new behavior is tested, important upstream behavior that could
regress is covered or explicitly verified, security boundaries include positive and negative
assertions, standard access policies still work without widened permissions, runtime/config/
persistence/resume state remains consistent, docs match the implemented contract, `git diff --check`
passes, and the relevant project checks from this file pass.

## Fork Release Branching

- When starting work against a new upstream release, create the branch pair from the release tag or release commit itself, not from the current fork working branch.
- Resolve the upstream release commit from the canonical release tag format `rust-vX.Y.Z` and use the dereferenced commit object (`^{}` for annotated tags) as the branch point.
- Create a local baseline branch named `fork/<major-minor>-upstream` that points exactly at that upstream release commit and keep it unchanged as the local source-of-truth baseline for the release.
- Create the working branch named `fork/<major-minor>` from the same upstream release commit and configure its git upstream to the local `fork/<major-minor>-upstream` branch.
- The canonical local setup is:
  - `git branch fork/<major-minor>-upstream <release-commit>`
  - `git switch -C fork/<major-minor> <release-commit>`
  - `git branch --set-upstream-to fork/<major-minor>-upstream fork/<major-minor>`
- Verify the relationship before doing fork work:
  - `git rev-parse --abbrev-ref --symbolic-full-name @{u}` must print `fork/<major-minor>-upstream`.
  - `git status --short --branch` must show `fork/<major-minor>...fork/<major-minor>-upstream`.
- Do not configure the working fork branch to track `origin/*` or `upstream/*` directly for release-baseline comparisons; the local `fork/<major-minor>-upstream` branch is the canonical baseline.
- Treat `fork/<major-minor>..fork/<major-minor>-upstream` as the default comparison range for local fork work on that release.
- Example for release `0.118.0`: create `fork/118-upstream` and `fork/118` from `rust-v0.118.0^{}`, then set the upstream of `fork/118` to `fork/118-upstream`.

## Fork Feature Integration Rules

- Use an `Upstream-first` approach by default: prefer the target release's architecture, control flow, and source-of-truth patterns as the baseline.
- Prefer additive and localized fork changes when that keeps maintenance and reasoning simpler.
- If a fork feature must modify an existing upstream-owned path directly, document the tradeoff in the feature doc instead of treating the divergence as self-evident.
- Preserve existing upstream behavior by default. If existing behavior must change, document that change explicitly as part of the fork contract.
- Do not introduce silent fallback behavior for fork logic. Any fallback must be explicit,
  controlled, observable to the caller or operator, documented in the feature contract, and covered
  by verification.
- When choosing between a literal backport and an upstream-shaped adaptation, prefer the upstream-shaped implementation unless it would break a documented fork contract.

## Compatibility And Regression Gates

- Every fork feature change must include evidence for the new behavior and for the most important existing path that could regress because of the change.
- If protocol, schema, state, or config contracts change, regenerate the canonical artifacts in the same change set and keep generated diffs minimal.
- If a regenerated artifact differs only by formatting, ordering noise, or trailing whitespace without a real contract change, revert it and keep only meaningful generated deltas.
- If fork work changes app-server protocol or behavior, document the affected client surfaces, stable vs experimental classification, and the compatibility matrix between separately distributed client versions and app-server/schema versions.
- If fork work regenerates `insta` snapshots, accept them through the normal `cargo insta review` / `cargo insta accept` flow. Do not manually rename `.snap.new` files into `.snap`, and do not keep review-only metadata such as `assertion_line` in final accepted snapshots.
- Do not treat "tests passed" as sufficient on its own; ensure the feature docs and affected public docs reflect the implemented contract.
- Before finalizing, confirm that the fork change still has one clear source of truth, one baseline release context, and one documented explanation for any intentional upstream divergence.

## Porting Decision Record

- Before implementing or porting a fork feature, record the upstream baseline and the expected fork gap in `docs/fork/research/<release>/`.
- Before changing source-of-truth files, decide whether the target behavior is a literal port or an upstream-shaped adaptation, and capture that choice in the feature/research docs.
- After implementation, verify contract surfaces explicitly: protocol/wire behavior, runtime behavior, UI/API text, generated artifacts, and persistence/config semantics when applicable.
- If the final implementation differs materially from the historical fork commit because of upstream adaptation, document the reason so future ports do not re-open the same decision.

Run `just fmt` (in `codex-rs` directory) automatically after you have finished making Rust code changes; do not ask for approval to run it. Additionally, run the tests:

1. Run the test for the specific project that was changed. For example, if changes were made in `codex-rs/tui`, run `cargo test -p codex-tui`.
2. Once those pass, if any changes were made in common, core, or protocol, run the complete test suite with `cargo test` (or `just test` if `cargo-nextest` is installed). Avoid `--all-features` for routine local runs because it expands the build matrix and can significantly increase `target/` disk usage; use it only when you specifically need full feature coverage. project-specific or individual tests can be run without asking the user, but do ask the user before running the complete test suite.

Before finalizing a large change to `codex-rs`, run `just fix -p <project>` (in `codex-rs` directory) to fix any linter issues in the code. Prefer scoping with `-p` to avoid slow workspace‑wide Clippy builds; only run `just fix` without `-p` if you changed shared crates. Do not re-run tests after running `fix` or `fmt`.

## The `codex-core` crate

Over time, the `codex-core` crate (defined in `codex-rs/core/`) has become bloated because it is the largest crate, so it is often easier to add something new to `codex-core` rather than refactor out the library code you need so your new code neither takes a dependency on, nor contributes to the size of, `codex-core`.

To that end: **resist adding code to codex-core**!

Particularly when introducing a new concept/feature/API, before adding to `codex-core`, consider whether:

- There is an existing crate other than `codex-core` that is an appropriate place for your new code to live.
- It is time to introduce a new crate to the Cargo workspace for your new functionality. Refactor existing code as necessary to make this happen.

Likewise, when reviewing code, do not hesitate to push back on PRs that would unnecessarily add code to `codex-core`.

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

### Snapshot tests

This repo uses snapshot tests (via `insta`), especially in `codex-rs/tui`, to validate rendered output.

**Requirement:** any change that affects user-visible UI (including adding new UI) must include
corresponding `insta` snapshot coverage (add a new snapshot test if one doesn't exist yet, or
update the existing snapshot). Review and accept snapshot updates as part of the PR so UI impact
is easy to review and future diffs stay visual.

When UI or text output changes intentionally, update the snapshots as follows:

- Run tests to generate any updated snapshots:
  - `cargo test -p codex-tui`
- Check what’s pending:
  - `cargo insta pending-snapshots -p codex-tui`
- Review changes by reading the generated `*.snap.new` files directly in the repo, or preview a specific file:
  - `cargo insta show -p codex-tui path/to/file.snap.new`
- Only if you intend to accept all new snapshots in this crate, run:
  - `cargo insta accept -p codex-tui`

If you don’t have the tool:

- `cargo install --locked cargo-insta`

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

- `app-server-protocol/src/protocol/common.rs` for JSON-RPC method/notification wiring and experimental gating.
- `app-server-protocol/src/protocol/v2/` for v2 payload types.
- `app-server/README.md`

### Client Compatibility Contract

- Treat the stable `codex app-server` wire/API surface as a public contract for Codex app clients.
- Preserve compatibility for existing stable clients by default: keep wire names, payload types, field optionality, method semantics, notifications, and legacy compatibility projections stable unless a documented fork contract explicitly requires otherwise.
- Do not make breaking stable app-server changes without a fork-governance decision, a documented deprecation or compatibility path, and a client compatibility matrix.
- Prefer additive app-server changes. Put risky, fork-specific, or unstable capabilities behind `experimentalApi` instead of changing stable behavior.
- Production clients must not enable `initialize.params.capabilities.experimentalApi` by default; opt in only for an owned integration with tests and a rollback path.

### Core Rules

- All active API development should happen in app-server v2. Do not add new API surface area to v1.
- Follow payload naming consistently:
  `*Params` for request payloads, `*Response` for responses, and `*Notification` for notifications.
- Expose RPC methods as `<resource>/<method>` and keep `<resource>` singular (for example, `thread/read`, `app/list`).
- Always expose fields as camelCase on the wire with `#[serde(rename_all = "camelCase")]` unless a tagged union or explicit compatibility requirement needs a targeted rename.
- Exception: config RPC payloads are expected to use snake_case to mirror config.toml keys (see the config read/write/list APIs in `app-server-protocol/src/protocol/v2/`).
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
- Experimental methods, fields, and enum variants must be rejected or omitted for clients that did not opt into `initialize.params.capabilities.experimentalApi = true`.
- For server-initiated requests or notifications, annotate experimental payload fields the same way and ensure app-server does not send those fields to non-opted-in clients.

### Client->server request payloads (`*Params`)

- Every optional field must be annotated with `#[ts(optional = nullable)]`. Do not use `#[ts(optional = nullable)]` outside client->server request payloads (`*Params`).
- Optional collection fields (for example `Vec`, `HashMap`) must use `Option<...>` + `#[ts(optional = nullable)]`. Do not use `#[serde(default)]` to model optional collections, and do not use `skip_serializing_if` on v2 payload fields.
- When you want omission to mean `false` for boolean fields, use `#[serde(default, skip_serializing_if = "std::ops::Not::not")] pub field: bool` over `Option<bool>`.
- For new list methods, implement cursor pagination by default:
  request fields `pub cursor: Option<String>` and `pub limit: Option<u32>`,
  response fields `pub data: Vec<...>` and `pub next_cursor: Option<String>`.

### Development Workflow

- Update docs/examples when API behavior changes (at minimum `app-server/README.md`).
- Regenerate schema fixtures when API shapes change:
  `just write-app-server-schema`
  (and `just write-app-server-schema --experimental` when experimental API fixtures are affected).
- Validate with `cargo test -p codex-app-server-protocol`.
- For app-server runtime behavior changes, add or run focused app-server suite tests that cover stable clients without `experimentalApi`; also cover opted-in experimental clients when the change affects experimental surface.
- Avoid boilerplate tests that only assert experimental field markers for individual
  request fields in `common.rs`; rely on schema generation/tests and behavioral coverage instead.
