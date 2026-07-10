# Evidence: app-server `thread/*` RPC

Файл фиксирует проверочные команды и решения triage. Основной язык - русский; commit subjects, PR ids, API/type/tool/wire names сохраняются как в upstream.

## Общий scope и refs

- Scope исследования: stable releases `rust-v0.86.0`-`rust-v0.144.1`.
- Start anchor: `rust-v0.86.0` -> `18057fdc30b10168ab18e8c1bb86c7a0a7772191`, `2026-01-15T16:46:06-08:00`.
- Scope boundary: только app-server `thread/*` RPC/protocol/docs/tests and direct backend storage evidence for those RPCs.
- Не включалось: раннее появление app-server и первичный v2 `thread/*` bootstrap до `rust-v0.86.0`.

## Базовые команды проверки

Для release anchors:

```bash
git show --quiet --format='%H|%cI|%s' <release-tag>^{}
git tag --list 'rust-v0.*.0' --sort=v:refname
```

Для functional entries:

```bash
git show --quiet --format='%H%x09%cI%x09%s' <sha>
git show --stat --name-only --oneline <sha>
git show --no-color --patch <sha> -- <selected-paths>
git tag --contains <sha> | rg '^rust-v0\.[0-9]+\.0$' | sort -V | head -1
git describe --contains <sha>
git merge-base --is-ancestor <sha> <release-tag>^{}
git merge-base --is-ancestor <sha> <previous-release-tag>^{}
```

Для release triage использовались path/keyword scans такого вида:

```bash
git log --no-merges --reverse --pretty=format:'%h %H %cI %s' <prev>^{}..<tag>^{} -- codex-rs/app-server codex-rs/app-server-protocol codex-rs/thread-store codex-rs/app-server/README.md
git log --no-merges --reverse --pretty=format:'%h %H %cI %s' <prev>^{}..<tag>^{} --grep='thread/list\|thread/read\|thread/resume\|thread/fork\|thread/archive\|thread/delete\|thread/unarchive\|thread/compact\|thread/rollback\|thread/search\|thread/loaded\|thread/settings\|thread/metadata\|thread/memoryMode\|thread/goal\|thread/realtime\|thread/name\|thread/status\|thread/turns\|thread/start\|thread/unsubscribe\|thread/shellCommand\|thread/background\|thread/increment_elicitation\|thread/decrement_elicitation\|thread/approveGuardianDeniedAction\|elicitation\|Guardian' --regexp-ignore-case
git log --no-merges --reverse --pretty=format:'%h %H %cI %s' <prev>^{}..<tag>^{} --grep='ThreadStore\|thread store\|thread-store' --regexp-ignore-case -- codex-rs/app-server codex-rs/app-server-protocol codex-rs/thread-store
```

## Топология stable boundaries после `rust-v0.142.5`

`A..B` в командах этого документа означает literal Git revision set: commits, достижимые из `B`, но не достижимые из `A`. Это не утверждение, что `A` является ancestor `B`, и не последовательная release ancestry-цепочка. Поэтому для каждой boundary отдельно фиксируются `git rev-list --left-right --count A...B`, `git merge-base A B`, right-only множество `A..B` и disposition left-only commits из `A --not B`.

- `rust-v0.142.5^{}` -> `rust-v0.143.0^{}`: left/right `6/258`; merge-base `27f22b54aef4d7e5eb6c564e969c961c74605461`. Шесть left-only commits — release commit `26de83050b20f7e0ee211b9739e52ae00ce8032a`, patch-only WebSocket logging fix `e019402a9e848c770e4d5a36accbef6d60c8b6e4`, revert `07f70323839c8ecb4312fafd4401fc3021307bb2`, V1 guidance restore `3b8b60a58399ff79a41f8457710ae5a5fae5c39f`, Bedrock catalog addition `a2325b50ff6003e18d549038eee1b21e9a645df1` и Ultra effort addition `aedb8f345a221b22105daad9fdf080988026fbca`. Они остаются историей patch-линии `0.142.x`, не входят в right-only inventory `0.143.0` и не переносятся в секцию `rust-v0.143.0`; прямой thread near-miss `e019402a9e` уже зафиксирован в `EVID-142.5-no-change`.
- `rust-v0.143.0^{}` -> `rust-v0.144.0^{}`: left/right `1/80`; merge-base `6afcf26d5d76c2f88b9096caa758931ffa673745`. Единственный left-only commit — release wrapper `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda`; содержательные изменения следующего релиза исследуются по right-only `rust-v0.143.0^{}..rust-v0.144.0^{}`, но сам wrapper не считается пропавшим feature commit.
- `rust-v0.144.0^{}` -> `rust-v0.144.1^{}`: left/right `1/4`; merge-base `3380969a29134630d56feb6218e8e8dcc5e8196d`. Единственный left-only commit — release wrapper `767822446c7a594caa19609ca435281a9ec67e0d`; right-only множество содержит три backport commits и новый release wrapper, поэтому `EVID-144.1-no-change` классифицирует именно их, не предполагая ancestry от `rust-v0.144.0` tag commit.

Команды topology-проверки:

```bash
git rev-list --left-right --count <A>...<B>
git merge-base <A> <B>
git log --format='%H%x09%s' <A> --not <B>
git log --format='%H%x09%s' <A>..<B>
```

## Release triage evidence

### `EVID-086-no-change`

- Диапазон: `rust-v0.85.0^{}..rust-v0.86.0^{}`.
- Included: none.
- Result: no direct app-server `thread/*` RPC/protocol/docs/test changes.

### `EVID-087-text-metadata`

- Диапазон: `rust-v0.86.0^{}..rust-v0.87.0^{}`.
- Included: `1fa8350ae79c37da62cd2f752a27cd67bc15cf4e`.
- Verification: stable containment first tag is `rust-v0.87.0`; diff touches protocol/app-server/core user-message metadata paths.
- Excluded near-misses: collaboration wait/multi-thread coordination and spawned-thread approval routing are multi-agent/collab behavior, not `thread/*` RPC contract.

### `EVID-088-list-ordering`

- Диапазон: `rust-v0.87.0^{}..rust-v0.88.0^{}`.
- Included: `f1653dd4d369d411f35442b48293816e9a92fb0e`.
- Verification: stable containment first tag is `rust-v0.88.0`; path-filtered diff covers app-server/core thread list ordering.
- Excluded near-misses: forked-session status display and collaboration mode turn overrides are not app-server `thread/list` contract.

### `EVID-089-read-archive`

- Диапазон: `rust-v0.88.0^{}..rust-v0.89.0^{}`.
- Included: `80240b3b6705e04d7b21e645ab5125243e597e49`, `733cb684963b793ce2ffe2de7c13b98a442dac83`.
- Verification: first stable containment is `rust-v0.89.0`; release note explicitly mentions `thread/read` and archived filtering in `thread/list`.
- Excluded near-misses: layered config and skill UI changes are app-server-adjacent, not thread namespace changes.

### `EVID-090-ephemeral`

- Диапазон: `rust-v0.89.0^{}..rust-v0.90.0^{}`.
- Included: `83775f4df117b991e99db962171552fc056395e8`.
- Verification: first stable containment is `rust-v0.90.0`; release note mentions ephemeral threads and diff is thread lifecycle/persistence.
- Excluded near-misses: `515ac2cd19` thread spawn source is multi-agent provenance; included in multi-agent research but not selected as primary `thread/*` RPC change here.

### `EVID-091-no-change`

- Диапазон: `rust-v0.90.0^{}..rust-v0.91.0^{}`.
- Included: none.
- Result: no direct app-server `thread/*` RPC/protocol/docs/test changes.

### `EVID-092-list-unarchive`

- Диапазон: `rust-v0.91.0^{}..rust-v0.92.0^{}`.
- Included: `62266b13f86422627993cc17dd8090afd9b5e77c`, `247fb2de647e60ebdfba41e4f10e317193e39406`.
- Verification: first stable containment is `rust-v0.92.0`; release notes mention `thread/unarchive` and thread list filtering.
- Excluded near-misses: dynamic tools on thread startup are tracked in tool/dynamic-tool surfaces unless the commit changes thread start wire shape directly.

### `EVID-093-no-change`

- Диапазон: `rust-v0.92.0^{}..rust-v0.93.0^{}`.
- Included: none.
- Excluded near-misses: SQLite log DB and rollout timing commits are infrastructure and not direct `thread/*` RPC contract entries.

### `EVID-094-unarchive-order`

- Диапазон: `rust-v0.93.0^{}..rust-v0.94.0^{}`.
- Included: `d3514bbdd221e1c32b381392a11600622d7d5e4f`.
- Verification: first stable containment is `rust-v0.94.0`; diff ties unarchive to `updated_at` ordering behavior.
- Excluded near-misses: schema fixture vendoring is generated artifact infrastructure, not standalone thread behavior.

### `EVID-095-no-change`

- Диапазон: `rust-v0.94.0^{}..rust-v0.95.0^{}`.
- Included: none.
- Excluded near-misses: `66b196a725` injects `CODEX_THREAD_ID` into terminal environment; this is runtime environment context, not app-server `thread/*`.

### `EVID-096-compact`

- Диапазон: `rust-v0.95.0^{}..rust-v0.96.0^{}`.
- Included: `38a47700b526a7ae3954d2dd8a233d166bc0a7b8`.
- Verification: first stable containment is `rust-v0.96.0`; release note names `thread/compact` v2 API.
- Excluded near-misses: websocket rate-limit signaling and requirements provenance do not alter `thread/*`.

### `EVID-097-no-change`

- Диапазон: `rust-v0.96.0^{}..rust-v0.97.0^{}`.
- Included: none.
- Excluded near-misses: dynamic tool output content item work affects app-server items broadly, but no selected direct `thread/*` method or notification change.

### `EVID-098-resume-model`

- Диапазон: `rust-v0.97.0^{}..rust-v0.98.0^{}`.
- Included: `fe8b474acd43cb8894d661944c6b5c8db0ef0ad1`.
- Verification: first stable containment is `rust-v0.98.0`; subject and diff mention core/app-server resume with different model.

### `EVID-099-summary-state`

- Диапазон: `rust-v0.98.0^{}..rust-v0.99.0^{}`.
- Included: `9ee746afd6273306e5f6e9ed1f7f48ac4e212c23`.
- Verification: first stable containment is `rust-v0.99.0`; diff uses state DB metadata for thread summaries.
- Excluded near-misses: app-server transport and opt-out event work is cross-cutting; selected only when it specifically changes thread state/subscription contract.

### `EVID-100-thread-state`

- Диапазон: `rust-v0.99.0^{}..rust-v0.100.0^{}`.
- Included: `b5339a591d8d31e91d74b3092f2c261eae77740b`, `c0ecc2e1e1eca3abaeb3ca1fd6ca1cc2e6173827`.
- Verification: first stable containment is `rust-v0.100.0`; release note mentions connection-aware thread resume subscriptions.

### `EVID-101-no-change`

- Диапазон: `rust-v0.100.0^{}..rust-v0.101.0^{}`.
- Included: none.
- Result: no direct app-server `thread/*` contract entry selected.

### `EVID-102-resume-list`

- Диапазон: `rust-v0.101.0^{}..rust-v0.102.0^{}`.
- Included: `efc8d45750e512f42a2b1de8a80733d593d9df17`, `ebe359b876ab200588f6414f31d6328c191c3565`, `fb0aaf94de7db12e22a293f5b0dd26aa6c8d6a8e`.
- Verification: first stable containment is `rust-v0.102.0`; path-filtered logs show thread list/resume/history changes.

### `EVID-103-no-change`

- Диапазон: `rust-v0.102.0^{}..rust-v0.103.0^{}`.
- Included: none.
- Result: app listing and co-authoring changes did not change `thread/*`.

### `EVID-104-archive-notifications`

- Диапазон: `rust-v0.103.0^{}..rust-v0.104.0^{}`.
- Included: `31cbebd3c20415c286316df2dbb51e073f42d52a`.
- Verification: first stable containment is `rust-v0.104.0`; release note names archive/unarchive notifications.

### `EVID-105-status-search`

- Диапазон: `rust-v0.104.0^{}..rust-v0.105.0^{}`.
- Included: `1f54496c48ffb9678095cc91d40556faa57c99fb`, `b06f91c4fe52088a0ccbdcadeec24997cdfdb770`, `035c4c30bb0a9468ec35ad8a5ae693a0fee6ae46`, `936e744c939071367d1c5e89cb8a7a211cc6c18c`, `37610240ecfa18cee5335d85d77f5ce3a364665b`, `f46b767b7ea82e6f15b6652373cb84e70cd7dc8d`.
- Verification: all included commits first contain in `rust-v0.105.0`; release note names status/search/resume work.
- Excluded near-misses: broad app-server tracing and Windows sandbox setup do not change thread namespace semantics.

### `EVID-106-realtime-unsubscribe`

- Диапазон: `rust-v0.105.0^{}..rust-v0.106.0^{}`.
- Included: `947092283ac3680582fe2fee4b246a61f3c87129`, `a0fd94bde6d7773694b295adb9825a647eaa9b93`, `21f7032dbb5ff6b1535abb44642aedca49f7a4fb`.
- Verification: all included commits first contain in `rust-v0.106.0`; release note names thread realtime and unsubscribe.

### `EVID-107-start-resume`

- Диапазон: `rust-v0.106.0^{}..rust-v0.107.0^{}`.
- Included: `69d7a456bbffd019504ab471a8f2a02abefaa0ae`, `8fa792868c75c7ce7b9b171ff3be9bfa4269f19e`, `8c1e3f3e6470a1b4cb5d8283e561ce55705394c5`.
- Verification: all included commits first contain in `rust-v0.107.0`.

### `EVID-108-name-metadata`

- Диапазон: `rust-v0.107.0^{}..rust-v0.108.0^{}`.
- Included: `14fcb6645c9935fe677ef3a79cbca8e4e2f94c83`, `9022cdc563db0a26f240bedb678539fc1534d7d5`, `935754baa353f13f76149f55c11a3b362a6089db`.
- Verification: all included commits first contain in `rust-v0.108.0`; direct subjects mention thread name/status/metadata.

### `EVID-109-no-change`

- Диапазон: `rust-v0.108.0^{}..rust-v0.109.0^{}`.
- Included: none.
- Result: no direct `thread/*` contract change selected.

### `EVID-110-no-change`

- Диапазон: `rust-v0.109.0^{}..rust-v0.110.0^{}`.
- Included: none.
- Result: no direct `thread/*` contract change selected.

### `EVID-111-gitinfo-resume`

- Диапазон: `rust-v0.110.0^{}..rust-v0.111.0^{}`.
- Included: `22f4113ac15673fe7e70f9651fa9f19df739d2a1`.
- Verification: first stable containment is `rust-v0.111.0`.

### `EVID-112-no-change`

- Диапазон: `rust-v0.111.0^{}..rust-v0.112.0^{}`.
- Included: none.
- Result: no direct `thread/*` contract change selected.

### `EVID-113-name-global`

- Диапазон: `rust-v0.112.0^{}..rust-v0.113.0^{}`.
- Included: `51fcdc760d83c16cc7fa5aeea4503facbadbc9e8`.
- Verification: first stable containment is `rust-v0.113.0`.

### `EVID-114-elicitation-counter`

- Диапазон: `rust-v0.113.0^{}..rust-v0.114.0^{}`.
- Included: `c6343e0649676579f174b6cd0da617d42ab1c58f`.
- Verification: first stable containment is `rust-v0.114.0`; `git describe --contains` returned `rust-v0.114.0-alpha.1~4`; `git merge-base --is-ancestor` is false for `rust-v0.113.0^{}` and true for `rust-v0.114.0^{}`.
- Containment paths: `codex-rs/app-server-protocol/src/protocol/common.rs`, `codex-rs/app-server-protocol/src/protocol/v2.rs`, `codex-rs/app-server/src/codex_message_processor.rs`, `codex-rs/app-server/tests/suite/v2/thread_resume.rs`.

### `EVID-115-fork-ephemeral`

- Диапазон: `rust-v0.114.0^{}..rust-v0.115.0^{}`.
- Included: `8ac27b2a161c08268a774bf8790087a79ac3b119`.
- Verification: first stable containment is `rust-v0.115.0`.

### `EVID-116-no-change`

- Диапазон: `rust-v0.115.0^{}..rust-v0.116.0^{}`.
- Included: none.
- Result: no direct `thread/*` contract change selected.

### `EVID-117-shell-realtime`

- Диапазон: `rust-v0.116.0^{}..rust-v0.117.0^{}`.
- Included: `bb304324216e1305e9b7b5aa59700907c6326bd7`, `01df50cf422b2eb89cb6ad8f845548e8c0d3c60c`, `3431f01776de7ba2c87de294a973e0593d0123a2`.
- Verification: all included commits first contain in `rust-v0.117.0`; release note names `thread/shellCommand` and app-server realtime transcript.

### `EVID-118-no-change`

- Диапазон: `rust-v0.117.0^{}..rust-v0.118.0^{}`.
- Included: none.
- Result: no direct `thread/*` contract change selected.

### `EVID-119-thread-start-trust`

- Диапазон: `rust-v0.118.0^{}..rust-v0.119.0^{}`.
- Included: `95e809c135344747008ae84c7b328399d29f1946`.
- Verification: first stable containment is `rust-v0.119.0`.

### `EVID-120-no-change`

- Диапазон: `rust-v0.119.0^{}..rust-v0.120.0^{}`.
- Included: none.
- Result: no direct `thread/*` contract change selected.

### `EVID-121-threadstore-foundation`

- Диапазон: `rust-v0.120.0^{}..rust-v0.121.0^{}`.
- Included: `e9e7ef3d366fdb95c24d24fea734b0fa47d79fc4`, `d25a9822a75a04b9c5552530381cf93f3f768fa5`, `2f6fc7c137bfa1b607b97bd932370022bea198eb`, `dae56994da917ae7bff84dae6f633ad17e5e1293`, `cdfcd2ca9268adbd4bd3ba5af527c97fa8a82805`.
- Verification: all included commits first contain in `rust-v0.121.0`; release note includes realtime APIs while ThreadStore commits are path-filtered storage evidence.

### `EVID-122-threadstore-history`

- Диапазон: `rust-v0.121.0^{}..rust-v0.122.0^{}`.
- Included: `50d3128269ebfa6221d02ecfbbc46b0007643bea`, `6e72f0dbfd5e6b875e1ce00e8079e2bc15e28fe4`, `ec8d4bfc77353ac328a7b4897ab47b296e90f19e`, `9d6f4f2e2e15880c754d8292fa0306f197e55a23`, `fad3d0f1d0179253dcab41e05bafb1bb936330d0`, `eaf78e43f2e95b978622b97c1f656236c7cd8927`.
- Verification: all included commits first contain in `rust-v0.122.0`; path-filtered diffs cover ThreadStore-backed archive/read/history and `thread/turns/list`.

### `EVID-123-unloaded-store`

- Диапазон: `rust-v0.122.0^{}..rust-v0.123.0^{}`.
- Included: `ac7c9a685f618b2a08dbeb6c42cfbc44a06f16d1`, `660153b6def82341a9c4cb21164ef467e27d3004`.
- Verification: first stable containment is `rust-v0.123.0`.

### `EVID-124-cwd-guardian`

- Диапазон: `rust-v0.123.0^{}..rust-v0.124.0^{}`.
- Included: `4f8c58f73744c6b1cebacb520cde695698745cb8`, `11e5af53c4f8595f968f1b8992e64dab799ae95e`.
- Verification: both included commits first contain in `rust-v0.124.0`; for `11e5af53c4`, `git describe --contains` returned `rust-v0.123.0-alpha.9~7`, and `git merge-base --is-ancestor` is false for `rust-v0.123.0^{}` and true for `rust-v0.124.0^{}`.
- Containment paths: multiple cwd filters touch `thread/list`; Guardian approval touches `codex-rs/app-server-protocol/src/protocol/common.rs`, `codex-rs/app-server-protocol/src/protocol/v2.rs`, generated JSON/TypeScript schema files and `codex-rs/app-server/src/codex_message_processor.rs`.

### `EVID-125-exclude-turns-env`

- Диапазон: `rust-v0.124.0^{}..rust-v0.125.0^{}`.
- Included: `3d3028a5a95122e65e2ee66bfb1e215e001c003b`, `49fb25997f3c09c25684c2a729cb933939a7f830`.
- Verification: both included commits first contain in `rust-v0.125.0`.

### `EVID-126-goal-store`

- Диапазон: `rust-v0.125.0^{}..rust-v0.126.0^{}`.
- Included: `0a9b559c0bb2ab50cf32035199e348d6284ae1ec`, `6c874f9b341a8fcb01bdc35999aad1ca093afea2`.
- Verification: both included commits first contain in `rust-v0.126.0`; release note names persistent goal workflows.

### `EVID-127-no-change`

- Диапазон: `rust-v0.126.0^{}..rust-v0.127.0^{}`.
- Included: none.
- Result: release repeats/extends goal and plugin themes, but no new direct `thread/*` entry selected beyond `rust-v0.126.0`.

### `EVID-128-no-change`

- Диапазон: `rust-v0.127.0^{}..rust-v0.128.0^{}`.
- Included: none.
- Result: no new direct `thread/*` entry selected.

### `EVID-129-history-sessionid`

- Диапазон: `rust-v0.128.0^{}..rust-v0.129.0^{}`.
- Included: `5affb7f9d57eea9523076088b74a0f1b9e9c3527`, `127be0612cf386a772aad29fb09dea8c81f89570`, `e4d66756328a616361a79eb792d86ad55f0bf5ae`, `541e99cf090c441c7bd4a5ac13b7f12da14697a2`, `707e51bd8bfd0f03c0f0c5e8ab975040d8e7ca12`, `33d24b0df5831b9e0413959fe6b849e11d6bb99b`, `9e0c191c133deaab89a17ef714e916bafe4dbc23`, `2c1a361a2e7232b8177ea0ea651dfcc616be895d`, `06e5dfa4dd9fc6fb30d65553757060428c9102c2`, `5ecff051962e7299c743e4ce9c1545d71b756924`.
- Verification: all included commits first contain in `rust-v0.129.0`; path-filtered diffs cover ThreadStore migrations, thread naming and `Thread.sessionId`.

### `EVID-130-pagination-contract`

- Диапазон: `rust-v0.129.0^{}..rust-v0.130.0^{}`.
- Included: `0d0835dd537b57913627e877f29031151b49429e`, `56823ec46bd1bf8d73646d21b256ca83455ab994`, `4242bba2ebc618253bb43032309daa42cfa39ac3`.
- Verification: first stable containment is `rust-v0.130.0`; release note names thread pagination APIs.

### `EVID-131-redaction-permissions`

- Диапазон: `rust-v0.130.0^{}..rust-v0.131.0^{}`.
- Included: `7bddb3083d677bf36a1cd65ecff9fe81c5f0bdf1`, `8a5306ff88b868685ca76cfd5cdda42dca637d10`, `83bbb4f32660c9246e90da92e664e2e69401c07b`.
- Verification: all included commits first contain in `rust-v0.131.0`; diffs cover app-server thread history redaction and permission/runtime-root projections.

### `EVID-132-no-change`

- Диапазон: `rust-v0.131.0^{}..rust-v0.132.0^{}`.
- Included: none.
- Result: no direct `thread/*` contract entry selected.

### `EVID-133-settings-update`

- Диапазон: `rust-v0.132.0^{}..rust-v0.133.0^{}`.
- Included: `771a4e74ac319c3d8379c62c7caa3aec1ad53382`.
- Verification: first stable containment is `rust-v0.133.0`; subject names `thread/settings/update`.

### `EVID-134-thread-search`

- Диапазон: `rust-v0.133.0^{}..rust-v0.134.0^{}`.
- Included: `ac0bff27e714da0243f656a577dc64679c807af2`, `05cf2fc4ce82b4f894031522a7c42698e6e6addd`.
- Verification: first stable containment is `rust-v0.134.0`; release note names local conversation history search.

### `EVID-135-no-change`

- Диапазон: `rust-v0.134.0^{}..rust-v0.135.0^{}`.
- Included: none.
- Result: no direct app-server `thread/*` change selected.

### `EVID-136-initial-turns-page`

- Диапазон: `rust-v0.135.0^{}..rust-v0.136.0^{}`.
- Included: `2a1158b8e2f941afed79db95731b16c8a8db5774`.
- Verification: first stable containment is `rust-v0.136.0`; subject names `thread resume`.

### `EVID-137-parent-search`

- Диапазон: `rust-v0.136.0^{}..rust-v0.137.0^{}`.
- Included: `cf0911076f234e0219bd8d61dd3bc2f80a2df287`, `11e0f3d3aecf631715b18581095d16c8b8f101e7`, `45912a6dc69ed04f1e7ef8ec3ecb4c9dc0860b6`.
- Verification: first stable containment is `rust-v0.137.0`; diffs cover `Thread.parent_thread_id`, history persistence flag removal and compressed search snippets.

### `EVID-138-paths-names`

- Диапазон: `rust-v0.137.0^{}..rust-v0.138.0^{}`.
- Included: `d8121f93c8df2e9066e929128886cae2fc81782b`, `40c8f1a0072e04f2df049c2233a28543604fc448`, `76c0a5379c79cc6eafc3a041791201e849ad4c7f`.
- Verification: first stable containment is `rust-v0.138.0`; direct subjects mention forked thread name, thread settings cwd and runtime workspace roots.

### `EVID-139-environments`

- Диапазон: `rust-v0.138.0^{}..rust-v0.139.0^{}`.
- Included: `f3c1283411edadcc0522bea376d0adc6961d5ca3`.
- Verification: first stable containment is `rust-v0.139.0`.

### `EVID-140-delete-realtime`

- Диапазон: `rust-v0.139.0^{}..rust-v0.140.0^{}`.
- Included: `6a9a49b334e8081756934b0ce7d909234b53aac7`, `4a3eac214494a5cefebe53308dc48ebabf03201c`, `a1a8807e9d67fad4b95f2730a9669eca5a9d27d0`, `a19d43a40aee5f3308ce57eda604fa085fd9f356`, `216dee1189fd589ea6c0741a5f92f578a3ca4640`.
- Verification: first stable containment is `rust-v0.140.0`; direct subjects cover cold resume, realtime overrides, background terminal process APIs, `thread/delete` and realtime append-text roles.

### `EVID-141-parent-dynamic-realtime`

- Диапазон: `rust-v0.140.0^{}..rust-v0.141.0^{}`.
- Included: `dfd03ea01bbec2613013b477fb82abc67534a7d7`, `11faf9af94fa345ba090701f071c8d3107fbca45`, `1d8ff89aa3808e364918bdec56d3cdfd74bf4d7b`.
- Verification: first stable containment is `rust-v0.141.0`; direct subjects cover parent thread filtering, thread start dynamic tool namespaces and realtime speech append.
- Excluded near-misses: selected executor plugin MCP activation and response-item metadata are relevant to thread-scoped runtime/tool context but do not directly change `thread/*` method contract.

### `EVID-142-*`

- Диапазон: `rust-v0.141.0^{}..rust-v0.142.0^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s%n%B' rust-v0.142.0^{}`, `git rev-parse --verify --short rust-v0.141.0^{}`, `git rev-parse --verify --short rust-v0.142.0^{}`, `git rev-list --count rust-v0.141.0^{}..rust-v0.142.0^{}`.
- Triage commands: Spark background thread plus local keyword/path-filtered scans over `codex-rs/app-server`, `codex-rs/app-server-protocol`, `codex-rs/thread-store`, `codex-rs/rollout`, `codex-rs/state`, `thread/list`, `thread/read`, `thread/resume`, `thread/fork`, `thread/search`, `thread/turns`, `thread/realtime`, `recencyAt`, `session id`, `goal-first` and `multi-agent`.
- Deep/local verification commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --name-only --oneline <sha>`, selected-path `git show --no-color --patch <sha> -- <paths>`, `git merge-base --is-ancestor <sha> rust-v0.142.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.141.0^{}`, `git describe --contains <sha>`.
- Included `EVID-142-thread-recency`: `fac3158c2a783095768076489815f361fa9b0db4`, `cb15c647608ea848713837e4b49975e5a1481051`, `7dc7096ae116df18c917d5d2f7c75243d98a0419`. Verified stable containment includes `rust-v0.142.0`; diffs introduce, revert and restore `recencyAt`/thread recency with compatible migration history across app-server protocol, ThreadStore, rollout/state and tests.
- Included `EVID-142-thread-history`: `1e6970542e3362d751c4561782c15d4b532db376`, `01a2df29479e8e982a5b6a9097d21814ad24000f`. Verified stable containment includes `rust-v0.142.0`; diffs add incremental thread-history changes and optional ThreadStore turn filters.
- Included `EVID-142-thread-resume-goal`: `e8dd1b45cbfe241abf85c6696f7ab07e85a0c5d5`, `6d15bb3d17f2b2a7d2a11ddd0689dc9244bf744c`; supporting metadata evidence: `8f8de7844f9a95ba0f07b9ae584b5c08b3173dd6`. Verified stable containment includes `rust-v0.142.0`; diffs fix goal-first live thread list/search visibility, persist session ids across resume and restore `thread_source` in turn metadata.
- Included `EVID-142-thread-realtime`: `683bd170dc36812f73867aa6f421a4a8cf4bd133`, `e922f46a0f863a9630ddf7dc60dcbadb36a6c28a`. Verified stable containment includes `rust-v0.142.0`; diffs cover app-server realtime handoff delivery controls, realtime append-text protocol/docs/tests and realtime websocket methods.
- Included `EVID-142-thread-multi-agent-mode`: `fc8c6b73841e279f95f53b08771a7969e953bdf4`, `7abfcf220bbb57029e2ff5d9914124aef7ef3d0f`, `c03742ca0a78a8e54cd881032a2327363678b5aa`. Verified stable containment includes `rust-v0.142.0`; diffs project `MultiAgentMode` through `thread/start`, `thread/fork`, `thread/resume`, `thread/settings/updated` and `turn/start`. Detailed runtime analysis is recorded in `docs/fork/research/multi-agents` as `EVID-142-multi-agent-mode`.
- Excluded near-misses: `21d36296f137c0954df24ea86abe9619318915e6` adds workspace messages app-server API outside `thread/*`; `d667082322` allows resume/settings slash commands during tasks/MCP startup but is a TUI command path rather than app-server thread protocol; `765309d5a611ea02be842ead0ab1a2828196fae9` is MCP app identity on tool-call items and belongs in MCP research; selected plugin/cache/skills warmup commits are outside direct `thread/*` method semantics.

### `EVID-142.1-no-change`

- Диапазон: `rust-v0.142.0^{}..rust-v0.142.1^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s' rust-v0.142.1^{}`.
- Included: none.
- Result: no direct app-server `thread/*`, ThreadStore/history/resume/realtime/session lifecycle contract entry selected.

### `EVID-142.2-turn-metadata-inject-items`

- Диапазон: `rust-v0.142.1^{}..rust-v0.142.2^{}`; broader patch verification also used `rust-v0.142.0^{}..rust-v0.142.5^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s' rust-v0.142.2^{}`, `git rev-list --count rust-v0.142.0^{}..rust-v0.142.5^{}`.
- Included commits: `4a82ecc3c9fcb8b9f21d5144c40a0dfdab54a02a`, `3b32d861c5ecc1e705812bc21177d0038e3c05ee`, `b294638bb557afe3e8bd1d82aa3ea9f459b2747e`.
- Commands: `git show --stat --name-only --oneline 4a82ecc3c9 3b32d861c5 b294638bb5`, selected-path inspection under `codex-rs/app-server-protocol`, `codex-rs/app-server/src/request_processors`, `codex-rs/app-server/tests/suite/v2/thread_inject_items.rs`, `codex-rs/core/src/session/*`, `codex-rs/core/src/context/world_state/*`, `codex-rs/core/src/context_manager/history.rs` and `codex-rs/protocol/src/models.rs`.
- Verification: first `rust-v0.142.*` containment is `rust-v0.142.2` for all included commits; `git merge-base --is-ancestor <sha> rust-v0.142.5^{}` succeeds and the same check against `rust-v0.142.0^{}` fails.
- Evidence notes: `#28360` preserves durable response-item `turn_id` metadata through history/resume/fork/compaction/websocket reuse; `#29249` seeds live world-state baseline from the latest `TurnContextItem` on resume while keeping world state unserialized; `#29419` rejects remote HTTP(S) image URLs at app-server ingress for injected raw history items while preserving legacy resume/history compatibility.
- Excluded near-misses: `7c22d376e5` adds turn-scoped safety-buffering metadata and notifications outside `thread/*`; `c53b1dae09` belongs in MCP/tool exposure research; `3b8b60a583` belongs in multi-agent V1 guidance; plugin/logo/catalog, proxy resolver, model catalog and formatter commits do not change selected thread contracts.

### `EVID-142.3-no-change`

- Диапазон: `rust-v0.142.2^{}..rust-v0.142.3^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s' rust-v0.142.3^{}`.
- Included: none.
- Result: no direct app-server `thread/*`, ThreadStore/history/resume/realtime/session lifecycle contract entry selected.

### `EVID-142.4-no-change`

- Диапазон: `rust-v0.142.3^{}..rust-v0.142.4^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s' rust-v0.142.4^{}`.
- Included: none.
- Result: no direct app-server `thread/*`, ThreadStore/history/resume/realtime/session lifecycle contract entry selected.

### `EVID-142.5-no-change`

- Диапазон: `rust-v0.142.4^{}..rust-v0.142.5^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s' rust-v0.142.5^{}`.
- Included: none.
- Result: no direct app-server `thread/*`, ThreadStore/history/resume/realtime/session lifecycle contract entry selected. `e019402a9e848c770e4d5a36accbef6d60c8b6e4` removes full Responses WebSocket request payload trace logging in `codex-rs/codex-api`, but this pass found no app-server/core/thread-store `thread/*` contract change from that backport.

### `EVID-143-thread-wire`

- Диапазон: `rust-v0.142.5^{}..rust-v0.143.0^{}`; release anchor: `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda`, `2026-07-07T17:43:00-07:00`; размер right-only множества: 258 commits.
- Включены: `1882719b30b01b50145fbca3772fb1537ed4bb2d`, `8d80b0176a3564e4061f3b29d8883f73d51e96d4`, `8057603d0c70930fe096302227f582b5b94496d8`, `268328001f899ec02cebab238fdeafda052525b6`, `6d9dbacf1af69fc9b633132e34d63be202556a24`, `5267e805fb830891c0b23376bcd9cbd382c3473c`, `812cd2bb57ddb3a04a10d59495ac0212d2eb52a3`, `f72976a5f1e5bba3e1af18cc2ddffedfb2687376`, `c9767411249b50279785b438646140f285536663`.
- Подтверждение контракта: experimental `thread/turns/items/list` заменён на `thread/items/list`, а `turnId` стал optional; `thread/list.ancestorThreadId` добавляет transitive strict-descendant filtering и взаимоисключается с `parentThreadId`; thread/turn ids документированы как UUID7; `thread/start.allowProviderModelFallback` включается явно и ограничен authoritative static catalogs; `Thread.historyMode`/`thread/start.historyMode` добавляют immutable `legacy|paginated` storage metadata с fail-closed handling; stable `thread/fork.lastTurnId` копирует историю до terminal turn включительно; `thread/rollback` помечен deprecated; opt-in `thread/realtime/start.flushTranscriptTailOnSessionEnd` сбрасывает последний недублирующийся transcript tail перед закрытием.
- Финальная форма на `rust-v0.144.1` проверена командой `git grep -n -E 'ThreadItemsList|ancestor_thread_id|history_mode|last_turn_id|allow_provider_model_fallback|flush_transcript_tail' rust-v0.144.1^{} -- codex-rs/app-server-protocol codex-rs/app-server codex-rs/thread-store codex-rs/protocol` и просмотром registry в `codex-rs/app-server-protocol/src/protocol/common.rs`.

### `EVID-143-resume-persistence`

- Диапазон: `rust-v0.142.5^{}..rust-v0.143.0^{}`.
- Включены: `1acb722e8afeba0cd54c9019f140830d05fdf5e7`, `330ae6a516009e127db01843f5049497d3c46805`, `806a4b66c9ad1f9e901e3e2cfeefdb403a0d6a07`, `3e51b46eba036364a735f450c2bb4d7c35d48d2b`, `fa036d39aadb160f819198d512efd1a2151a761b`, `a74771340db6eb81db39e81445a706346f14c139`, `723b23efd0c06289e878b294b29f6d7ed98a66e8`, `c55ce3b51bc3b8bb2805cbee0afc1a8c6e1e7ee2`, `f5f812389ee49ab4c9ef1237781ea1013e733fdc`, `a107b84967eb9a3444fd2d4de03f200337acd52b`, `b9b934e99b57aa1908a7cd83adee9faf6d0b8f7d`.
- Подтверждение lifecycle/persistence: effective originator сохраняется в `SessionMeta` и восстанавливается/наследуется при resume/fork; resumed rollout vectors совместно используются через `Arc` без изменения serialization; fork-only response items получают stable ids до reconstruction и persistence; world-state snapshots/merge patches становятся durable rollout items и replay при resume/fork/rollback/compaction, а отсутствующие retained fragments безопасно reinject; submission-channel teardown закрывает live writer; terminal turn completion/interruption явно вызывает flush ThreadStore; canonical command/dynamic-tool/collaboration/sub-agent формы `TurnItem` и app-server projections образуют основу paginated storage.
- Причина включения: эти entries меняют durable resume/fork/lifecycle path или app-server history projection даже без нового JSON-RPC method. `330ae6a516` является implementation-only и serialization-neutral, но входит в нормализованную cold-resume data-flow map.
- Исключённые near-misses: `02c326f646` только инструментирует persistence bytes; `39ee1b96b0`, `b539e3e72b` и revert `d69828e18b` относятся к persistence metrics; `841f30598c` — analytics attribution, а не wire/durable originator ownership; `bdd282f3bb` меняет timeout `currentTime/read` вне `thread/*`; `4f1b5a4b73` структурирует shutdown logs; `db887d03e1`, `042e61726d`, `cfead68e5d` и `b35d4b6b9d` меняют Responses/Rendezvous WebSocket logging, liveness или incremental comparison, а не app-server thread realtime RPC; TUI managed defaults и rollback warning suppression являются UI/runtime near-misses.

### `EVID-144-canonical-history-resume`

- Диапазон: `rust-v0.143.0^{}..rust-v0.144.0^{}`; release anchor: `767822446c7a594caa19609ca435281a9ec67e0d`, `2026-07-09T08:59:13-07:00`; размер right-only множества: 80 commits.
- Включённый canonical lifecycle/projection stack: `cca16a10878202cb2f6e9666b6b4330329ea7e65`, `f659eb12bc8cecb976d92db192d9b2983c8053ff`, `1bd9d841ca9ec8c99e4843f5be3d1d71cb5d8169`, `058d97c5dcf572ef7084e488bbb108d24aa19e54`, `6b4882528eaf2d927904bba37c48e40d925c35d1`, `23aac925e7b3fd02b9d6a47147b7b8310a124b49`, `ba5dd1fd3acd8802325945b88219e69e8b744c40`.
- Включённые paginated/resume entries: `2342b2c2a66b5ac2a07278afb4f5f38700be452e`, `0746e8a34574b4bf4721672c97fc6a94fd8bfad8`.
- Подтверждение: canonical lifecycle `ItemStarted`/`ItemCompleted` становится live app-server source для command execution, dynamic tools, MAv1/MAv2 collaboration/sub-agent activity, review markers и hook prompts, а централизованные mappings сохраняют legacy raw events. Paginated threads сохраняют completed canonical `TurnItem` snapshots, требуют stable ids, наследуют history mode при fork и отбрасывают redundant legacy projections; legacy threads сохраняют старый event format. Resume сохраняет effective approval reviewer в turn context/settings events и восстанавливает последнее значение, если request не задаёт явный override.
- Исключённая adjacent item work: `f1affbac5e` и `a219b6fdb4` переносят ownership image-generation/web-search в extension items, сохраняя публичную app-server JSON shape; это полезно для projection architecture, но не выбрано как thread contract delta. `7affe3e3e4` является behavior-neutral extraction legacy fanout. Imported-session title sanitization, test fixture/builder migrations, auth, proxy-aware Responses WebSockets и model updates не меняют выбранный thread contract.

### `EVID-144.1-no-change`

- Диапазон: `rust-v0.144.0^{}..rust-v0.144.1^{}`; release anchor: `44918ea10c0f99151c6710411b4322c2f5c96bea`, `2026-07-09T15:10:27-07:00`; размер right-only множества: 4 commits, включая release commit.
- Включены: нет.
- Результат: commits `7ef1728763f5b6e346158a74e365ab6fbb48332c`, `9d47eb221ddd275e3936a76fb2f79efa21f4911e` и `77c42a202aa7e89189c1126021dcc054b1795e1d` являются installer/Darwin code-mode/embedded-V8 reliability backports и не меняют app-server `thread/*`, ThreadStore/history/resume, rollout/session persistence или realtime lifecycle contracts.

### Команды прохода `rust-v0.142.5` -> `rust-v0.144.1`

```bash
git rev-parse 'rust-v0.142.5^{}' 'rust-v0.143.0^{}' 'rust-v0.144.0^{}' 'rust-v0.144.1^{}'
git rev-list --left-right --count 'rust-v0.142.5^{}...rust-v0.143.0^{}'
git rev-list --left-right --count 'rust-v0.143.0^{}...rust-v0.144.0^{}'
git rev-list --left-right --count 'rust-v0.144.0^{}...rust-v0.144.1^{}'
git rev-list --count 'rust-v0.142.5^{}..rust-v0.143.0^{}'
git rev-list --count 'rust-v0.143.0^{}..rust-v0.144.0^{}'
git rev-list --count 'rust-v0.144.0^{}..rust-v0.144.1^{}'
git log --format='%H%x09%aI%x09%s' 'rust-v0.142.5^{}..rust-v0.144.1^{}' -- codex-rs/app-server codex-rs/app-server-protocol codex-rs/thread-store codex-rs/core/src/thread_manager.rs codex-rs/core/src/session codex-rs/rollout codex-rs/protocol/src
git log --format='%H%x09%aI%x09%s' 'rust-v0.142.5^{}..rust-v0.144.1^{}' | rg -i 'thread|session|resume|rollout|history|realtime|websocket|app.server|archive|fork|turn'
git show -s --format='%H%n%cI%n%s%n%B' rust-v0.143.0^{} rust-v0.144.0^{} rust-v0.144.1^{}
git show --stat --name-only --oneline <included-sha>
git show --no-color --patch <included-sha> -- <selected-paths>
git merge-base --is-ancestor <included-sha> <target-release>^{}
git diff --check -- docs/fork/research/thread/architecture.md docs/fork/research/thread/evidence.md docs/fork/research/thread/timeline.md
```
