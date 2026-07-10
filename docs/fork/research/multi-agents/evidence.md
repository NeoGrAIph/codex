# Evidence: multi-agent/collab/subagents

Файл фиксирует проверочные команды и решения triage. Основной язык - русский; commit subjects, PR ids, API/type/tool names сохраняются как в upstream.

## Общий scope и refs

- Upstream refs проверены командой: `git ls-remote --tags upstream 'refs/tags/rust-v*'`.
- Локальные tags были обновлены командой: `git fetch upstream --tags --prune`, потому что upstream stable scope включал `rust-v0.137.0`, `rust-v0.138.0`, `rust-v0.139.0`, которых изначально не было локально.
- Stable scope после обновления: `rust-v0.81.0`-`rust-v0.144.1`. Alpha tags не входят в основной timeline.
- Для расходящихся stable tags запись `A^{}..B^{}` используется в буквальном Git-смысле: множество коммитов, достижимых из `B`, но не из `A`; это не утверждение о последовательном родстве tags. Топология дополнительно фиксируется через `git rev-list --left-right --count A^{}...B^{}` и `git merge-base A^{} B^{}`, а коммиты только с левой стороны разбираются отдельно и не смешиваются с правосторонней дельтой `B`.

## Базовые команды проверки entry

Для каждого functional entry применяются команды такого типа:

```bash
git show --quiet --format='%H%x09%cI%x09%s' <sha>
git show --stat --name-only --oneline <sha>
git show --no-color --patch <sha> -- <selected-paths>
git tag --contains <sha> | rg '^rust-v0\.[0-9]+\.0$'
git describe --contains <sha>
git merge-base --is-ancestor <sha> <release-tag>^{}
git merge-base --is-ancestor <sha> <previous-release-tag>^{}
```

## Release triage evidence

### `EVID-081-*`

- Диапазон: `rust-v0.80.0..rust-v0.81.0`.
- Triage commands: `git cat-file -p rust-v0.81.0`, `git show --no-patch --pretty=fuller --date=iso-strict rust-v0.81.0`, `git log --oneline --date=iso-strict rust-v0.80.0..rust-v0.81.0`, `git log --oneline --grep='clb\|collab\|agent\|thread listeners\|wait tool\|close tool\|response.done\|threadId\|model client sessions' rust-v0.80.0..rust-v0.81.0`, `git show --stat --no-color <SHA>`, `git merge-base --is-ancestor <SHA> rust-v0.80.0`, `git merge-base --is-ancestor <SHA> rust-v0.81.0`.
- Included: `568b938c80`, `86f81ca010`, `623707ab58`, `9659583559`, `3b8d79ee11`, `97f1f20edb`, `acfd94f625c4f75d927c3debc533347bcd19da24`.
- Excluded near-misses: `6709ad8975` image labels, `bcd7858ced` thread listener refresh, `490c1c1fdd` model client sessions, `0c09dc3c03` MCP threadId, `2d56519ecd` response.done, `cbca43d57a`/`57ba758df5`/`17ab5f6a52`/`bdae0035ec` general TUI queue/output improvements. Они могут влиять на общий runtime experience, но diff не вводит/меняет multi-agent/collab/subagent contract напрямую.

### `EVID-082-*`

- Диапазон: `rust-v0.81.0^{}..rust-v0.82.0^{}`.
- Triage commands: `git rev-parse rust-v0.81.0^{} rust-v0.82.0^{}`, `git log --oneline --no-decorate rust-v0.81.0^{}..rust-v0.82.0^{}`, `git rev-list --count rust-v0.81.0^{}..rust-v0.82.0^{}`, `git show --stat --name-only --oneline <sha>`, `git show --no-color --stat --patch <sha>`, `git merge-base --is-ancestor <sha> rust-v0.82.0`, `git merge-base --is-ancestor <sha> rust-v0.81.0`.
- Included: `6a939ed7a46a0cba28b505fd404f1fc17fdfad24`, `3d322fa9d80cee853df81f3d43c7175210c990ec`, `e6d2ef432d6214ceef9ae12c512f3940284f6819`.
- Excluded near-misses: `8e937fbba949281f45f18ececd3b76c91a6883ab` startup input queue, `5e426ac27030ec0c9e57334c6885ef9802236356` WebSearchMode, `577e1fd1b2e5ad03f92b8c65b16547476395c35c` PTY fallback, `02f67bace8f7dc3e264b27b4c884b0c187cb166f` SSE completed handling. Это общие runtime/API improvements без direct multi-agent surface change.

### `EVID-083-no-change`

- Диапазон: `rust-v0.82.0^{}..rust-v0.83.0^{}`.
- Triage commands: `git log --oneline --no-merges rust-v0.82.0^{}..rust-v0.83.0^{}`, `git show --name-only --stat <sha>`, `git show --patch <sha>`, `git tag --contains <sha>`.
- Excluded: `3728db11b87cb8490bcf6bf2cdf0e13dcfb0c28b` (`fix: eliminate unnecessary clone() for each SSE event (#9238)`) because it changes generic SSE tracing/performance, not multi-agent/collab/subagent behavior.

### `EVID-084-no-change`

- Диапазон: `rust-v0.83.0^{}..rust-v0.84.0^{}`.
- Triage commands: `git rev-list --oneline --no-merges --cherry-pick rust-v0.83.0^{}..rust-v0.84.0^{}`, `git show --stat --oneline <sha>`, `git show <sha> -- <paths>`, `git merge-base --is-ancestor <sha> rust-v0.84.0^{}`, `git log --oneline --grep='sub-agent\|subagent\|collab\|multi-agent\|multiagent\|old collab\|collaboration' rust-v0.83.0^{}..rust-v0.84.0^{}`.
- Excluded: `4a9c2bcc5a565cbbc1d4535041d9d68bd4899dc2` (`Add text element metadata to types (#9235)`) because it is broad protocol/text metadata work and does not alter agent coordination semantics.

### `EVID-085-*`

- Диапазон: `rust-v0.84.0^{}..rust-v0.85.0^{}`.
- Triage commands: `git show-ref -s --verify refs/tags/rust-v0.84.0`, `git show-ref -s --verify refs/tags/rust-v0.85.0`, `git show --no-patch --pretty=format:'%H|%ad|%s|%D' --date=short rust-v0.84.0^{}`, `git show --no-patch --pretty=format:'%H|%ad|%s|%D' --date=short rust-v0.85.0^{}`, `git log --oneline rust-v0.84.0^{}..rust-v0.85.0^{}`, `git show --stat/--patch <sha>`.
- Included: `bad4c12b9da59f3d73e9c73e6602004412e0bbc6`, `05b960671dcd3ab062a1214b93447851ea432636`, `faeb08c1e139590a1e01929d2bf289c2a6c47e2e`, `3fc487e0e0788045460df1abe21dc9de47f3b7f9`.
- Excluded: release commit `4607330eff53e3ae39126afef76882042e14a03c` is used as release anchor, not functional entry; `/models` metadata `#9219` is release-relevant but not multi-agent/collab.

### `EVID-086-*`

- Диапазон: `rust-v0.85.0^{}..rust-v0.86.0^{}`.
- Triage commands: `git rev-parse rust-v0.85.0^{} rust-v0.86.0^{}`, `git log --oneline --decorate --date=iso-strict --reverse rust-v0.85.0^{}..rust-v0.86.0^{}`, `git show --no-patch --pretty=... rust-v0.86.0^{}`, `git show --stat --name-only --oneline <sha>`, `git show --stat --patch --unified=3 <sha> -- <selected-paths>`.
- Included: `393a5a0311b5d19d1c4b0065cc4f952c059f14e4`.
- Excluded near-misses: `42fa4c237f2b70f07f8759a731455f9453daf363` SKILL.toml metadata, `169201b1b50c5b6b86f4e26613042575aba01034` web search eligibility, `004a74940a5226fd10921572d5f05f6350be8e1d` MCP elicitation content. Они не меняют multi-agent/subagent contract напрямую.

### `EVID-087-*`

- Диапазон: `rust-v0.86.0^{}..rust-v0.87.0^{}`.
- Triage commands: `git rev-parse "rust-v0.86.0^{}" "rust-v0.87.0^{}"`, `git cat-file -p rust-v0.87.0^{}`, `git log --no-merges --oneline --date=iso --reverse rust-v0.86.0^{}..rust-v0.87.0^{}`, `git show --patch --no-color <commit>`.
- Included: `c576756c81ee84036093e7abcb0f0660dd3c2914`, `f5b3e738fbaa742a38071e367c18b1626837c25e`, `7905e99d03027f17dff5df50cef0fb601fc10f93`.
- Excluded near-misses: `1fa8350ae79c37da62cd2f752a27cd67bc15cf4e` text metadata, `99f47d6e9a3546c14c43af99c7a58fa6bd130548` MCP `threadId` output, `c1ac5223e125b042205466a70cbd509166ebec57` user shell snapshot. They are adjacent thread/runtime improvements, not direct multi-agent/collab contract changes.

### `EVID-088-*`

- Диапазон: `rust-v0.87.0^{}..rust-v0.88.0^{}`.
- Triage commands: `git rev-parse rust-v0.87.0^{} rust-v0.88.0^{}`, `git log --no-merges --oneline --pretty=format:'%h %ad %s' --date=short <range>`, `git show --name-only --pretty='format:%h %s (%ad)' <commit>`, `git show --unified=0 <commit> -- <key files>`.
- Included: `246f50655111fda3d4fa4f2032285d03464fe249`, `146d54cede3c4ea38fbcacc89ab15a7e30d73b20`, `1478a88eb0e655ede6efe86764a5bf9255ca7a13`, `8f0e0300d2ebdb8e743342d73b30a7ba615e9747`, `f72f87fbeeea3663c99bf29478cf5765d4c806f6`, `31415ebfcf21e91b85a06effdc702a0b76ba3e46`, `d544adf71a4bc4368059a84df3e56d8481279a1e`, `675f165c56e0600d3bf4ccc6e798853c0d5fee60`, `483239d861596a1917fe6d0c82487a92929870aa`, `bf430ad9fee5583afab2bd6641407d67383be297`, `5ae6e70801110da6245ba900a8f07bee3e3d53f1`, `57ec3a82778b1cc88a000016b79bd4732ff18a43`, `0523a259c848c583b3960066160066312e43278c`, `5f55ed666b7551e030d0d0bf7576dd69909d512f`, `f1c961d5f7a00033c5c668a8a6ddbe96d2004762`.
- Excluded near-misses: typo-only `46a4a03083`, generic feature-framework beta/experimental changes, general shell/env/metric/UI preview changes, and release summary `149625a4...`.

### `EVID-089-*`

- Диапазон: `rust-v0.88.0^{}..rust-v0.89.0^{}`.
- Triage commands: `git rev-parse --verify rust-v0.88.0^{}`, `git rev-parse --verify rust-v0.89.0^{}`, `git log --oneline --no-decorate --date=iso --pretty='%H|%ad|%s' rust-v0.88.0^{}..rust-v0.89.0^{}`, `git show --name-only --pretty=format: <commit>`, `git show --unified=0 <commit>`.
- Included: `fe641f759f1725fc32920b228a4b9a86f2d13966`, `3fcb40245efbee1d9992686f1bdff243d6f9122a`, `4210fb9e6cb3f50bb93d8fdfcb4494af27a36352`.
- Excluded near-misses: `836f0343a37d119aee124a66832348d7a033a35c` (`Add tui.experimental_mode setting (#9656)`) because it is a broader TUI experimental setting, not a direct Plan/collab behavior change in this release; generic app-server `thread/read`, archived filtering, layered config, permissions/skills UI, personality/template and `end_turn` changes. They are useful platform work but not direct multi-agent/collaboration-mode evolution for this timeline.

### `EVID-090-*`

- Диапазон: `rust-v0.89.0^{}..rust-v0.90.0^{}`.
- Triage commands: `git rev-list --count rust-v0.89.0^{}..rust-v0.90.0^{}`, `git cat-file -p rust-v0.90.0`, `git log --oneline --date=short --pretty=format:'%h %ad %s' rust-v0.89.0^{}..rust-v0.90.0^{}`, `git show --unified=0 --name-only <commit>`.
- Included: `515ac2cd19a22151388d16c2d6c73e604cdbd3a0`, `83775f4df117b991e99db962171552fc056395e8`, `69cfc73dc6d9cfd7cf0ad4c47942dfcaeebee2ac`, `58450ba2a1d7c0d2d5405ae5f1f99fcf09e1cfe4`, `b3127e2eebcdc872cf43966649561b6d10aa8c56`, `f30f39b28b13e7048bfca7f7e8d4022e7840f443`, `0e79d239ed8d2802cacea364c023a72570afb415`, `d86bd20411522b7ecfd466aea7e384daac3e7fac`, `f353d3d695260543aa85af508303345bb06ed159`, `b332482eb1a183166d13bc8b58fbb2137fd3f446`, `73b5274443cd3ef70ee8d30d707f8fdf805b7ad2`.
- Excluded near-misses: network proxy, connectors, generic personality API, session-source telemetry, resume-cwd UX, text-element persistence. Some are adjacent to thread/state infrastructure but not direct multi-agent/collaboration-mode contract changes.

### `EVID-091-*`

- Диапазон: `rust-v0.90.0^{}..rust-v0.91.0^{}`.
- Triage commands: `git log --oneline --decorate rust-v0.90.0^{}..rust-v0.91.0^{}`, `git show --patch 8fea8f73d6c0dc77ffaf5e24d739ca348f30d5c0`, `git rev-list --left-right --pretty=oneline rust-v0.90.0^{}...rust-v0.91.0^{}`.
- Included: `8fea8f73d6c0dc77ffaf5e24d739ca348f30d5c0`.
- Excluded: release summary `3684bc646e0f1123e76a90f03cfb1111358f1aa1`.

### `EVID-092-*`

- Диапазон: `rust-v0.91.0^{}..rust-v0.92.0^{}`.
- Triage commands: `git tag --list 'rust-v0.91.*' 'rust-v0.92.*' --sort=version:refname`, `git rev-list --count <range>`, `git log --format='%H %ad %s' <range>`, `git show --name-only --pretty=... <commit>`, `git show --unified=... <commit>`.
- Included: `375a5ef05116a3d289dc548402387f86e28851cc`, `3ba702c5b6eb53523d3ff644789e900fffa3533c`, `c66662c61bda8480e747cde951b15186e5aa3134`, `3f338e4a6a70413dea864441f06f72becd49615c`, `70d59593988d3af390527a6aadd6e801da5d6a33`, `47aa1f3b6affa3b29e7c94ba56bdecb299a1c095`, `01d7f8095b54dc3b194def8a4d90de02c93f2fbf`, `d27f2533a9b44f1c42437cf8e05fc5da84bcf779`, `159ff062813653996b507d51dd451566440d2ec0`, `b7bba3614ef7a051f8e510092954367dfc85b9a0`, `b655a092bad1f1e96031d75ae9230dde7e796b64`, `28bd7db14aa600ed7b8b58bb7531932b9133b6f8`, `cabb2085cc05655a96d6e66d21a70c1ee37d06dd`, `509ff1c643e0b45366fc5d77c78279ca29b71448`.
- Excluded near-misses: `247fb2de647e60ebdfba41e4f10e317193e39406` (`[app-server] feat: add filtering on thread list (#9897)`) because source filtering is useful for discovery but too broad for the main multi-agent behavior timeline; dynamic tools injection `#9539`, `thread/unarchive #9843`, generic request-user-input overlay UX outside collab modes, model instructions template move. They remain platform-relevant but are not direct multi-agent/collab evolution entries here.

### `EVID-093-*`

- Диапазон: `rust-v0.92.0^{}..rust-v0.93.0^{}`.
- Triage commands: `git log --oneline/--pretty --reverse rust-v0.92.0^{}..rust-v0.93.0^{}`, `git log -1 --format=... rust-v0.92.0^{}`, `git log -1 --format=... rust-v0.93.0^{}`, `git show --stat --name-only <commit>`.
- Deep analysis commands: `git log -1 --format='%H%x09%cI%x09%s' <tag>^{}`, `git tag --contains <sha>`, `git describe --contains --tags <sha>`, `git show --stat --oneline --find-renames <sha>`, targeted `git show --no-color --unified=N <sha> -- <files>`.
- Included: `ec4a2d07e411d11f4de13b4e09aa7386c914df6d`, `11958221a3354fe7941671cc2c4a24ffd73c29a7`, `a0ccef9d5c44b7517243163ceb64be8223d5d8d5`, `1ce722ed2efe69165b16b0af3501c613dbddef0c`, `9b29a48a09893cfdea7d7ea056918a9a011ccc01`, `b7351f7f53d07584bf292c1b61bb58c8aa839bb5`, `83317ed4bff93b5cf98d3d3ec82dad66dd5b2207`.
- Excluded near-misses: `fc0fd853...` initial context rollout timing, `715138747...` dynamic tools in rollout, `dabafe204...` exec auto-subscribe to new threads, `894923ed...` SDK config overrides. They are useful architecture evidence for threads/tools/config but not main functional multi-agent timeline entries.

### `EVID-094-*`

- Диапазон: `rust-v0.93.0^{}..rust-v0.94.0^{}`.
- Triage commands: `git log --no-merges --reverse --format='%h|%ad|%s' rust-v0.93.0^{}..rust-v0.94.0^{}`, `git rev-list --no-merges --count rust-v0.93.0^{}..rust-v0.94.0^{}`, `git show --stat --oneline <commit_sha>`.
- Deep analysis commands: `git log -1 --format='%H%x09%cI%x09%s' <tag>^{}`, `git tag --contains <sha>`, `git describe --contains --tags <sha>`, `git show --stat --oneline --find-renames <sha>`, targeted `git show --no-color --unified=N <sha> -- <files>`.
- Included: `30ed29a7b37e183f151ea04362bb22b496dc456e`, `2d6757430a05c4c619384b6f1d2f9c8847d4416e`, `3dd9a37e0bb7af065eed668c2c6a4c96cec85320`, `03fcd12e77fedf4fa327af27e2e476e1ebc5f651`, `974355cfddb2402803d06076e60d12a2bd46bf37`.
- Excluded near-misses: personality commits (`8a461...`, `a33fa...`, `11c912...`, `0f985...`, `ed9e...`), model prompt template commits and `d3514...` thread unarchive ordering. They are not direct multi-agent/Plan evolution entries.

### `EVID-095-*`

- Диапазон: `rust-v0.94.0^{}..rust-v0.95.0^{}`.
- Triage commands: `git tag --list 'rust-v0.94.0' 'rust-v0.95.0'`, `git rev-parse rust-v0.94.0 rust-v0.95.0`, `git log --reverse --oneline --date=short rust-v0.94.0^..rust-v0.95.0^`, `git log --no-merges --oneline --date=short --grep='collab\|plan\|experimental\|app-server\|protocol\|persistence\|state\|TUI\|request_user_input' <range>`, `git show --stat --oneline <commit>`.
- Deep analysis commands: `git log -1 --format='%H%x09%cI%x09%s' <tag>^{}`, `git tag --contains <sha>`, `git describe --contains --tags <sha>`, `git show --stat --oneline --find-renames <sha>`, targeted `git show --no-color --unified=N <sha> -- <files>`.
- Included: `9513f18bfe9edd9795f86c9645bdd023189a567c`, `3cc9122ee2596d937def5ed6fcd221dc446813b5`, `3392c5af243821c85c3b816cbb0623a795a91385`, `d509df676b61d837d0c1dfb287ae53f1cce52578`, `998eb8f32be5ea0f29271d07d2fb812a69c6a3da`, `1096d6453c096a1860ba8e2595729cc69e2e312b`, `8f5edddf7121fdb676feac511c80da1b6d4dfb16`, `d9ad5c3c4959d2cabd57f9aaed75e96bb2c07c91`.
- Excluded near-misses: `74327fa...` Space in experimental popup, `fb2df99...` DB persistence for dynamic tools, skills/API/loading commits and parallel shell work. `fb2df99...` remains useful architecture evidence for dynamic tools/resume, but not a main Plan/collab timeline entry.

### `EVID-096-*`

- Диапазон: `rust-v0.95.0^{}..rust-v0.96.0^{}`.
- Triage commands: `git tag --list 'rust-v0.95.0' 'rust-v0.96.0'`, `git rev-parse rust-v0.95.0^{} rust-v0.96.0^{}`, `git log --oneline --no-decorate rust-v0.95.0^{}..rust-v0.96.0^{}`, `git rev-list --oneline --no-merges rust-v0.95.0^{}..rust-v0.96.0^{}`, `git show --name-only --stat <hash>`.
- Deep analysis commands: `git log -1 --format='%H%x09%cI%x09%s' <tag>^{}`, `git tag --contains <sha>`, `git describe --contains --tags <sha>`, `git show --stat --oneline --find-renames <sha>`, targeted `git show --no-color --unified=N <sha> -- <files>`.
- Included: `a9eb766f33953d651a0f01670d893a4cf5da3763`.
- Excluded near-misses: `38a47700b526a7ae3954d2dd8a233d166bc0a7b8` (`Add thread/compact v2 (#10445)`), `1eb21e279e9282b5b894bcfcf890d390cae7d643` (`Requirements: add source to constrained requirement values (#10568)`), `100eb6e6f0067f75b2e964c84769c5b9fae680a6` (`Prefer state DB thread listings before filesystem (#10544)`), `583e5d4f413dc10457e685d26672529ba7d2f70e` (`Migrate state DB path helpers to versioned filename (#10623)`). They are useful architecture background but not direct multi-agent/Plan/collab functional entries.

### `EVID-097-*`

- Диапазон: `rust-v0.96.0^{}..rust-v0.97.0^{}`.
- Triage commands: `git rev-parse rust-v0.96.0^{} rust-v0.97.0^{}`, `git log --oneline --reverse --no-decorate rust-v0.96.0^{}..rust-v0.97.0^{}`, `git log --pretty=format:'%H|%cI|%s' --no-decorate rust-v0.96.0^{}..rust-v0.97.0^{}`, `git show --stat --oneline <sha>`, targeted `git show <sha> -- <file>`.
- Deep analysis commands: `git show --quiet --format='%H%x09%cI%x09%s' <sha>`, `git merge-base --is-ancestor <sha> rust-v0.97.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.96.0^{}`, `git tag --contains 7f203576116de4002fcda5942a1e4248e18692ff`, `git describe --contains --tags 7f203576116de4002fcda5942a1e4248e18692ff`, `git show --stat --oneline --find-renames 7f203576116de4002fcda5942a1e4248e18692ff`, `git show --no-color --unified=80 7f203576116de4002fcda5942a1e4248e18692ff -- codex-rs/core/src/tools/handlers/collab.rs codex-rs/core/src/codex.rs codex-rs/core/src/client.rs`.
- Included: `7f203576116de4002fcda5942a1e4248e18692ff`.
- Excluded near-misses: `282f42c0ce3c067a1a00c7590f76ef7bec47e207` session-scoped MCP/Apps approvals, `7a253076fef8058adb7964fdd521df80663cf39d` pending input persistence, `7bcc552325b95b1f17042c0e4f928ef971f26339` live skills watcher, `5ea107a08857e44001fe156a67b7d8536d5db66b` dynamic tool text/image content items, `0e8d359da900512129750cd6cce0a7799ddea28e` session-level model client, `e9335374b9ef43710e76a411caca5e3a0bdf139b`/`4922b3e571d2da548f7e592707b4a1fac8fda09f` memory phase 1, `d452bb3ae5b5e0f715bba3a44d7d30a51b5f28ae` `/debug-config`, and TUI polish commits `d589ee05...`, `4ed8d74...`, `d876f3...`. They are platform or architecture evidence but not direct multi-agent/collab/Plan timeline entries.

### `EVID-098-*`

- Диапазон: `rust-v0.97.0^{}..rust-v0.98.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.97.0^{}..rust-v0.98.0^{} --grep='collab\|sub.agent\|subagent\|multi.agent\|agent\|plan mode\|/plan\|request_user_input\|collaborationMode'`, `git show --quiet --format='%H%x09%cI%x09%s' 41b4962b0a7f5d73bb23d329ad9bb742545f6a2c`, `git show --stat --oneline --find-renames 41b4962b0a7f5d73bb23d329ad9bb742545f6a2c`.
- Included: `41b4962b0a7f5d73bb23d329ad9bb742545f6a2c`.

### `EVID-099-*`

- Диапазон: `rust-v0.98.0^{}..rust-v0.99.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.98.0^{}..rust-v0.99.0^{} --grep='collab\|sub.agent\|subagent\|multi.agent\|agent\|plan mode\|/plan\|request_user_input\|collaborationMode\|experimentalApi'`, `git show --quiet --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`.
- Included: `62605fa47102d72bb901dcbd0b2be5ec68ace8c9`, `87ccc5bbae759629d8a51525c977c56c978e0b09`, `223fadc7605894638e01aa9f96c9cd1836c9ef85`, plus grouped UX/rollout evidence commits `040ecee7154556924e3b66b4b52253a51afc731f`, `b7ecd166a682de39ff1206e5c62afadbcbef72a2`, `1751116ec62aebf9c242ad7ed00515386629f56e`, `f3f35526a8056e28a99ef57136ef548301eee9d2`, `13de7442965c2b59df46e21287585ba57d8de71d`, `284c03ceabe0fc52fdff8e24657d1fa4ddf5fcdb`, `c2bfd1e473377e158cbdcb234dd62d2fc3ecf7ce`.
- Excluded near-misses: memory v2 commits, app loading/install flow, network constraints/proxy, search/app skill expansion, status-line and generic rollback/status fixes.

### `EVID-100-*`

- Диапазон: `rust-v0.99.0^{}..rust-v0.100.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.99.0^{}..rust-v0.100.0^{} --grep='sub-agent\|collab\|multi-agent\|agent'`, `git show --quiet --format='%H%x09%cI%x09%s' 2fac9cc8cd1afb02e3ac507b21786eaa3ab6bc52`, `git show --stat --oneline --find-renames 2fac9cc8cd1afb02e3ac507b21786eaa3ab6bc52`.
- Included: `2fac9cc8cd1afb02e3ac507b21786eaa3ab6bc52`.
- Excluded near-misses: memory agent consolidation, rate limits, `js_repl`, websocket transport and sandbox read-access work.

### `EVID-101-no-change`

- Диапазон: `rust-v0.100.0^{}..rust-v0.101.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.100.0^{}..rust-v0.101.0^{}`, path-filtered logs over multi-agent/collab/TUI/app-server surfaces.
- Excluded: memory cwd context `#11591`, because it changes memory behavior rather than multi-agent/Plan/collab contract.

### `EVID-102-*`

- Диапазон: `rust-v0.101.0^{}..rust-v0.102.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.101.0^{}..rust-v0.102.0^{} -- <multi-agent/collab/TUI/app-server paths>`, `git show --quiet --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames e41536944e e47045c806 beb5cb4f48 76283e6b4e 851fcc377b`.
- Included: `e41536944e7e89763c51d7780bd1c3f3b8957836`, `e47045c80686164df0d608ca652244646786c1d6`, `beb5cb4f4808c0b1c7717e47535868f1b9e2e050`, `76283e6b4e0fae570344b221b4bbb152d969e850`, `851fcc377bcfa8c5bf2453bc91e9717727b412f1`.
- Excluded near-misses: generic permissions/network approval, app listing/filtering, model reroute, remote skills and memory slash-command work.

### `EVID-103-no-change`

- Диапазон: `rust-v0.102.0^{}..rust-v0.103.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.102.0^{}..rust-v0.103.0^{}`, path-filtered logs over multi-agent/collab/TUI/app-server surfaces.
- Excluded: richer app listing and commit co-author attribution, because neither changes multi-agent/Plan/collab behavior.

### `EVID-104-no-change`

- Диапазон: `rust-v0.103.0^{}..rust-v0.104.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.103.0^{}..rust-v0.104.0^{}`, path-filtered logs over multi-agent/collab/TUI/app-server surfaces.
- Excluded near-misses: thread archive/unarchive notifications and distinct command approval IDs.

### `EVID-105-*`

- Диапазон: `rust-v0.104.0^{}..rust-v0.105.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.104.0^{}..rust-v0.105.0^{} --grep='collab\|sub.agent\|subagent\|multi.agent\|agent\|plan mode\|/plan\|request_user_input\|collaborationMode'`, path-filtered `git log` over `multi_agents`, `agent`, TUI and app-server protocol paths, `git show --quiet --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`.
- Included: `2daa3fd44fe5787aee79016a1019fe8cec369151`, `dcab40123f5e64ba8af962ae27abe6cbcc205344`, config/guardrail group (`7b65b05e87acc26de946eed7d47c4352fc28a93f`, `9f5b17de0d2969d825385091a7e83094627ad633`, `d87cf7794c0cbff93310021ab3a159ae87b0eca7`, `8758db5d5bf9f3d98bff595f38f6124d0174a2a8`, `6d6570d89d13d45fe5cea4e653dbe9a4f6c25a84`, `01f25a7b9646bf71672cb3363132ff8f97556c27`), TUI/model group (`486e60bb5515da92ae4a64ef31004a194c200b1a`, `0f9eed3a6f479395520fcd7aceb9fc5b2f3f3754`, `4d60c803ba9c703b978e89a8f6f8a4ad1866582c`, `5a30cd3f92f28ba9a32b38c6d277b377493a253b`, `829d1080f641197c3ce8351c3e9951dccc3636c6`, `cf0210bf22c5b2f0dbf5c2610add895f6ee8e96b`, `0679e70bfce35d589363df24c49abd75ff98b10a`, `bcd6e68054ef5ac7507733fdb46cb15ec5156773`), Plan/rollout group (`c3cb38eafbbbe9ccd9fd18c10e76417941c5d59d`, `6e60f724bcd18e722086e646f9d7e4f19214a9f1`, `4c1744afb22383ae61bebf7feb19a24eedc970df`, `2119532a812831578a8ffeb2b5ac014037518106`).
- Excluded near-misses: zsh bridge, syntax highlighting/theme picker, voice transcription, app-server thread search/status/resume improvements, generic approval/network permissions, apps cache/config plumbing and js_repl fixes.

### `EVID-106-*`

- Диапазон: `rust-v0.105.0^{}..rust-v0.106.0^{}`.
- Commands: `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.105.0^{}..rust-v0.106.0^{} --grep='collab\|sub.agent\|subagent\|multi.agent\|agent\|plan mode\|/plan\|/agent\|request_user_input'`, path-filtered logs over multi-agent/TUI/app-server/protocol paths, `git show --quiet --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`.
- Included: `2f4d6ded1dd7a6d6e3c9ed6cded8f2bf41328f05`; polish group `51cf3977d496825a6d89d8a2326aa7aecde8af9e`, `79cbca324ab0f58d9f41b1e0e90bde5adb0aee55`, `f0a85ded1867072d5b4963b8be3be1ef4f216549`.

### `EVID-107-*`

- Диапазон: `rust-v0.106.0^{}..rust-v0.107.0^{}`.
- Commands: same grep/path pattern, plus `git show --stat --oneline --find-renames d3603ae5d38ab3addbf995ee8c51a22ceb068872`, `git tag --contains d3603ae5d38ab3addbf995ee8c51a22ceb068872`.
- Included: `d3603ae5d38ab3addbf995ee8c51a22ceb068872`, `3404ecff153241c688fd34307a4049acf92ad561`.
- Excluded near-misses: local date/timezone, permission profile file roots, pending item replay and model availability metadata.

### `EVID-108-*`

- Диапазон: `rust-v0.107.0^{}..rust-v0.108.0^{}`.
- Commands: same grep/path pattern, plus `git show --stat --oneline --find-renames f8838fd6f3a22f228a1ed551df67789b25c82b12`, `git tag --contains f8838fd6f3a22f228a1ed551df67789b25c82b12`.
- Included: `/agent` enablement group (`2b38b4e03bcb33fb5e04bb0771714dfd9b759d6d`, `eec3b1e235ff59582acd1bb37fe3ad7df2adbe3b`, `9a42a56d8f0a14298c0e54b1ac6d7dda48ec347b`, `f8838fd6f3a22f228a1ed551df67789b25c82b12`, `2e154a35bc6df9239dff23b6552076fc2a8a49b1`, `932ff2818320e7c5181a28316db75e53620aeabc`, `bda3c49dc4a4ea052ed55faf9c4310e6875ca41e`), runtime-fix group (`c2e126f92ad560cfbda3a542db4e669680af9c25`, `3166a5ba82...`, `cacefb5228...`, `e4a202ea52...`).
- Excluded near-misses: plugin loading, artifacts, fast mode, enterprise feature requirements, general app-server tracing and sandbox/network policy changes.

### `EVID-109-no-change`

- Диапазон: `rust-v0.108.0^{}..rust-v0.109.0^{}`.
- Commands: `git rev-list --count rust-v0.108.0^{}..rust-v0.109.0^{}`, `git log --no-merges --reverse --pretty=format:'%h%x09%cI%x09%s' rust-v0.108.0^{}..rust-v0.109.0^{}`.
- Result: only release summary commit in the stable interval.

### `EVID-110-*`

- Диапазон: `rust-v0.109.0^{}..rust-v0.110.0^{}`.
- Commands: grep/path pattern, `git show --quiet --format='%H%x09%cI%x09%s' f80e5d979d7a1872da11520507ae881860fa4268`, `git show --stat --oneline --find-renames f80e5d979d7a1872da11520507ae881860fa4268`.
- Included: `f80e5d979d7a1872da11520507ae881860fa4268`.

### `EVID-111-no-change`

- Диапазон: `rust-v0.110.0^{}..rust-v0.111.0^{}`.
- Commands: release log and path-filtered logs over multi-agent/collab/TUI/app-server surfaces.
- Result: no direct multi-agent/collab/subagent/Plan timeline entries.

### `EVID-112-no-change`

- Диапазон: `rust-v0.111.0^{}..rust-v0.112.0^{}`.
- Commands: release log and path-filtered logs over multi-agent/collab/TUI/app-server surfaces.
- Result: no direct multi-agent/collab/subagent/Plan timeline entries.

### `EVID-113-*`

- Диапазон: `rust-v0.112.0^{}..rust-v0.113.0^{}`.
- Commands: grep/path pattern, `git show --quiet --format='%H%x09%cI%x09%s' e84ee33cc02e693a3cf66204c72cb37e8dda3ed6`, `git show --stat --oneline --find-renames e84ee33cc02e693a3cf66204c72cb37e8dda3ed6`.
- Included: `e84ee33cc02e693a3cf66204c72cb37e8dda3ed6`; guardian follow-up/test stabilization commits kept as evidence, not separate functional entries.

### `EVID-114-no-change`

- Диапазон: `rust-v0.113.0^{}..rust-v0.114.0^{}`.
- Commands: release log and path-filtered logs.
- Excluded near-miss: `c6343e0649` thread-level elicitation counter; relevant to app-server stopwatch/elicitation, not direct multi-agent.

### `EVID-115-*`

- Диапазон: `rust-v0.114.0^{}..rust-v0.115.0^{}`.
- Commands: grep/path pattern, `git show --stat --oneline --find-renames bc24017d64829d0b97b8bc6ed529a389e1e8bc1b`, `git show --stat --oneline --find-renames 91ca20c7c3 793bf32585 7626f61274 cfd97b36da bc24017d64`, `git show --format=fuller --stat a67660da2d274282c9c8dee7101787bf023e6f94 -- codex-rs/core/src/config/agent_roles.rs codex-rs/core/src/config/config_tests.rs codex-rs/core/config.schema.json`, `git show --no-color --patch --unified=80 a67660da2d274282c9c8dee7101787bf023e6f94 -- codex-rs/core/src/config/agent_roles.rs`, `git show --format=fuller --stat 4fa7d6f444b919afb6ccec25e49c036aa0180971`, `git show --no-color --patch --unified=60 4fa7d6f444b919afb6ccec25e49c036aa0180971 -- codex-rs/core/src/config/agent_roles.rs codex-rs/core/src/config/config_tests.rs`, `git tag --contains bc24017d64829d0b97b8bc6ed529a389e1e8bc1b`, `git tag --contains 4fa7d6f444b919afb6ccec25e49c036aa0180971`, `git merge-base --is-ancestor 4fa7d6f444b919afb6ccec25e49c036aa0180971 rust-v0.115.0^{}`.
- Included: guardian/old-MA hardening group (`bc24017d64829d0b97b8bc6ed529a389e1e8bc1b`, `6fdeb1d602842b80088641b941dea174435c01b7`, `91ca20c7c39e326aa995c350cb68547d57a9bf54`, `a67660da2d274282c9c8dee7101787bf023e6f94`, `285b3a51435d3ff1da7e4e78b613d2f451f04915`, `793bf32585c31e5c3a33a538bc816c8023074da7`, `7626f612748515d6d79e149c2ae37d7d783cf989`, `8e89e9ededc64253c228749521fc9d8049f8947b`, `cfd97b36da76a17db407b2d9653ed993636e0a30`, `36dfb844277e79793766f96305c9633f90bc043e`, `7f571396c8819d7f4c4486ed1e967e40a2c9ffae`, `3f266bcd68c78ac043969f8a7a916c7ee30df112`), agent-role-files detail (`a67660da2d274282c9c8dee7101787bf023e6f94`), agent-role-nonfatal detail (`4fa7d6f444b919afb6ccec25e49c036aa0180971`), fan-out/navigation group (`a4d884c767622e694899a8ddc2de6e4c165aae1c`, `180a5820fc1fa3ca398f088f8906cfe74f7c22a0`, `ce1d9abf117651965ffb312d94929267190a3149`, `bf5e997b318935732c110f3f8366394d8e1370f3`).
- Excluded near-misses: image inspection, js_repl, realtime transcription, filesystem APIs, app integrations and plugin workflow work.

### `EVID-116-*`

- Диапазон: `rust-v0.115.0^{}..rust-v0.116.0^{}`.
- Commands: grep/path pattern, `git show --quiet --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`.
- Included: `4ed19b07664d28ef67592ab5d77aa30d13d3aba0`, `e8add54e5dda2fc6f49757aa939378a21b8515e9`, `84f4e7b39d17fea6d28c98bc748652ea4b279a14`, `a265d6043edc8b41e42ae508291f4cfb9ed46805`.
- Excluded near-misses: plugin setup, hooks, realtime and auth work.

### `EVID-117-*`

- Диапазон: `rust-v0.116.0^{}..rust-v0.117.0^{}`.
- Commands: grep/path pattern, `git show --stat --oneline --find-renames 79ad7b247bb6805853b00f55d2e992810ce949ea`, `git show --stat --oneline --find-renames 37ac0c093c 18f1a08bc9 773fbf56a4 4a5635b5a0`, `git tag --contains 79ad7b247bb6805853b00f55d2e992810ce949ea`, `git tag --contains 773fbf56a43a38ef65e9f64da279db22264aa3d5`.
- Included: `79ad7b247bb6805853b00f55d2e992810ce949ea`, MAv2 messaging group (`37ac0c093cb4be42f7812737366cab181b9d0417`, `450dc289c3305bf9d94d862d6d30c4916aa2497a`, `18f1a08bc9c6e39331d9cf34ee240ea0124173cb`, `191fd9fd16e8f4ea43adb438122fb13f1b2ed674`, `4605c653085ac1ea3a4c48e4d1727022bd942f68`, `527244910fb851cea6147334dbc08f8fbce4cb9d`, `38c088ba8d03b7b211464166e04be0391e53266e`, `b51d5f18c7154498524b705ab2fbb81259d2bbed`, `773fbf56a43a38ef65e9f64da279db22264aa3d5`), bridge/guard group (`70cdb17703a4310b7173642e011f7534d2b2624f`, `4a5635b5a0336274b6ee196140bfe151b18a642d`, `352f37db03315dc215fbf23cb36b442554afb8c5`, `970386e8b2a776d47ef6ac6c2bd67a3c0ed86744`).
- Excluded near-misses: plugin workflow, app-server shell/filesystem, image workflow, terminal title and generic hook changes.

### `EVID-118-*`

- Диапазон: `rust-v0.117.0^{}..rust-v0.118.0^{}`.
- Triage commands: `git rev-parse rust-v0.117.0^{} rust-v0.118.0^{}`, `git rev-list --count <start>..<end>`, path-filtered `git log` over app-server/protocol/TUI/core agent/config paths, keyword grep over collab/multi-agent/plan, `git show --name-only --pretty='format:%h %s' <commit>`.
- Deep analysis commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included: MAv2 spawn/mailbox group (`6a0c4709ca2154e9f3ebb07e58fb156386630188`, `426f28ca99a809134bbc9d4879789582f80093c1`, `213756c9ab22b567d426fe1be9757705fb5862c9`, `c74190a622667158e53b6d10fbfc0387ec43b964`) and mode/agent-picker fixes (`37b057f0030ab34f66dbb6eb6f8e7e7a3443407c`, `ed977b42ac4f0b71ea218546153a751866f25b5b`, `bede1d9e23202e2fce23b2ad6d154255672a675b`, `38e648ca67802d5f2fb23f9b3bd3f200cdb067fa`).
- Excluded near-misses: auth/sandbox/network refactors, app-server transport/auth work, TUI voice/legacy split removal, custom prompt removal, codex-tools extraction, generic TUI copy/status changes and release rollup `b630ce9a4e`.

### `EVID-119-*`

- Диапазон: `rust-v0.118.0^{}..rust-v0.119.0^{}`.
- Triage commands: `git rev-parse rust-v0.118.0^{} rust-v0.119.0^{}`, `git rev-list --count rust-v0.118.0^{}..rust-v0.119.0^{}`, `git log --oneline --no-abbrev-commit <range>`, path-filtered logs over core/TUI/app-server/protocol/config/state, `git show --stat --oneline --no-abbrev-commit <commit>`.
- Deep analysis commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included: MAv2 contract group (`1fc8aa0e169c74b571960f529baafc17d686beda`, `285f4ea8176c74934e22fc9f78e216b9c7da429c`, `d0474f2bc1eb246c206c1e9c02338f52bf8d992e`, `23d638a573ab0775c8e8c7f54db8e19ddbb10332`, `0c776c433b02ae4e07efc2db9eac0e55455630a3`, `7fc36249b5f8661fd067018282e68c12a3ae0912`, `8a19dbb1776d5a99df3ddb6d79ace2a7b2b93620`, `2a8c3a2a527d1b3cb51365a72552bfa3aff32040`, `68e16baabe79a1f48c7d93c9a3d6e234a9b2c9b7`, `4cc6818996bbe47b4f490f9ca3ce3cd8504bf6c3`, `4c07dd4d25006a41d35872f520aa2e18bc3200e7`), mailbox/guardian group (`e4f1b3a65e06a74e2716b01e695a6c1d37a8fdbd`, `dcbc91fd39ba4f3d90fbfd96fa7ba6e8cf9a6159`).
- Excluded near-misses: `/resume` per ID/name, generic session indexing, `clientMetadata`, `ServerResponse`, remote-control, realtime/WebRTC, prompt-debug/config knobs, state DB maintenance and release rollup.

### `EVID-120-*`

- Диапазон: `rust-v0.119.0^{}..rust-v0.120.0^{}`.
- Triage commands: `git rev-parse --verify --short rust-v0.119.0^{}`, `git rev-list --count rust-v0.119.0^{}..rust-v0.120.0^{}`, `git log --oneline --no-merges <range>`, path-filtered logs over TUI/app-server/protocol/core/tools/rollout, `git show --stat --oneline --no-color <commit>`.
- Deep analysis commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included: background-agent group (`60236e8c920f59ddec4faa70f7a7d58a2984b2f9`, `1de0085418340b3e7f7136cfb5e56b4bebafc584`, `2e81eac004e280fdb447f0d8b74dfc76f4db2913`, `029fc63d13b943c1f10c8c66da6fa16488fc1ed5`) and guardian/MAv2 description group (`4e910bf151d5a037dc522939c837ec5a81b516ec`, `88165e179a9547e6ff56a6538462f6efdf04db46`, `147cb8411267062c5c2b93e95171ec13c943338c`, `a3be74143ab7a1d9641b234ffb11f826cee471e1`, `d39a722865f5e6af210f752a6603329b2566f676`).
- Excluded near-misses: `SessionStartSource`, hook rendering, status-line title, rollout recorder reliability, quota/rate-limit, MCP `outputSchema`, sandbox/windows safety changes.

### `EVID-121-*`

- Диапазон: `rust-v0.120.0^{}..rust-v0.121.0^{}`.
- Triage commands: `git tag --list 'rust-v0.12*'`, `git rev-parse rust-v0.120.0^{} rust-v0.121.0^{}`, `git rev-list --count <range>`, `git log --no-color --oneline --date=short --reverse <range>`, path-filtered logs over app-server/protocol/TUI/core/rollout/config, keyword grep over agent/plan/guardian.
- Deep analysis commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included: guardian/identity/Plan group (`37aac89a6db500b8647f8b0fc9251e629eecea51`, `39cc85310fbb1c4d04034e596cd7420090875799`, `8e784bba2fb795ec9248bbfd9a26fdad1b809dc6`, `3b948d9dd8d2e13a36e95a05efb6bb2288b801c4`, `ce5ad7b295afbaa763b595eef5501ce4c3eb84ab`) and fork/mailbox group (`776246c3f5f931a72ebe41f52ef719e3b413371d`, `05c582992359e47afaa298c045c62af42001a463`).
- Excluded near-misses: generic memory APIs, thread-store, filesystem metadata, slash history search, remote-control seq-id, marketplace/plugin flows, MCP apps/namespacing, memory endpoint controls and local thread-store listing.

### `EVID-122-*`

- Диапазон: `rust-v0.121.0^{}..rust-v0.122.0^{}`.
- Triage commands: `git for-each-ref --format='%(refname:short) %(objecttype) %(objectname)' refs/tags/rust-v0.121.0 refs/tags/rust-v0.122.0`, `git rev-parse <tag>^{}`, `git rev-list --count <start>..<end>`, `git log --no-merges <start>..<end> --oneline ...`, `git show --name-only --pretty=format:'%h %ad %s' --date=short <commit>`.
- Deep analysis commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included: agent identity/task/policy group (`55c3de75cba1a65088ff02b91527b94a1b60a69b`, `e5b52a3caa7a050c4df81570f5698d74793ef942`, `49403e3676f8f3685f0d4ba4a1292b8953d05b42`, `0500801123ada245ba752cf0577ca08cc7fc0065`, `b44d2851cf0e7b728d4a848d4a4f68da3483dfd6`, `fc758af9eb4ba82af629493d4ada6b0f0ac66973`) and Plan/side/mailbox group (`d3692b14c900560c3775e3cdaf2f0d9a9b37902c`, `241136b0e9cd5d787d74a905af315252930c886b`, `95dafbc7b5b5c6905db9015b3850dc5967523b2a`, `e3c2acb9cdd37277c334765a30ea2ac6020a2b9f`).
- Excluded near-misses: external config migration prompt, Fast mode copy, queued slash/shell prompts, marketplace/plugin work, thread-store/memory management, MCP/tool discovery/image detail, app-server marketplace removal and generic config aliases.

### `EVID-123-*`

- Диапазон: `rust-v0.122.0^{}..rust-v0.123.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.123.0^{}`.
- Triage commands: `git rev-list --count rust-v0.122.0^{}..rust-v0.123.0^{}`, `git log --pretty=format:'%h %ad %s' --date=short --reverse rust-v0.122.0^{}..rust-v0.123.0^{}`, path-filtered logs over `codex-rs/core`, `codex-rs/tui`, `codex-rs/app-server`, `codex-rs/app-server-protocol`, `codex-rs/codex-api` and config/schema paths.
- Deep analysis commands: `git show -s --format='@@ %H%nDATE %ad%nSUBJECT %s%nBODY %B' --date=iso-strict <sha>`, `git show --stat --oneline --no-color <sha>`, `git show --no-color --find-renames --unified=80 <sha> -- <relevant paths>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included `EVID-123-mav2-fork-model`: `15b8cde2a4f63bb510d46c88d3ddd7e286bee203`, `54bd07d28c8c8684217fd5ab83c0e7d943d06d91`. Verified stable containment includes `rust-v0.123.0`; `git describe --contains` maps them to `rust-v0.123.0-alpha.7~1` and `rust-v0.123.0-alpha.2~14` respectively as auxiliary alpha evidence.
- Included `EVID-123-realtime-handoff`: `126bd6e7a8839a861ee9bb40ec72c72ea1bf7b4d`, `ca3246f77a5eca14f3424d786ce8855fe8811dbc`, `1029742cf7747ad2e82fe10d931744372902e2d6`. Verified stable containment includes `rust-v0.123.0`; release notes explicitly mention realtime handoffs and `#18597`, `#18761`, `#18635`.
- Included `EVID-123-external-side`: `833212115e18e9eb75b0a75dcecf4034b8bab6ac`, `0dc503ba6e0f82a27fc676bcdc8bec642b20e6fe`. Verified stable containment includes `rust-v0.123.0`; diffs show external-agent config ownership move and direct TUI side-conversation parent-status behavior.
- Excluded triage candidates: `5fe767e8e1` (`ConfigManager`) is broad app-server config infrastructure, not direct multi-agent/collab behavior; `ab26554a3a` and `46e5814f77` are remote sandbox/thread-store config surfaces without direct subagent/collab diff in this release; `ac7c9a685f` and `a718b6fd47` are thread-store read/write path changes but not direct multi-agent behavior; `3d2f123895`, `dcec516313`, `34a3e85fcd`, `48f82ca7c5` are protocol/permission/device-key/patch-notification changes outside direct multi-agent/collab scope; `5a8700abcc`, `43a69c50eb`, `2af4f15479`, `7e5588699d` are MCP diagnostics, picker names, TUI refactor or review-prompt cleanup without direct multi-agent/collab functional change.

### `EVID-124-*`

- Диапазон: `rust-v0.123.0^{}..rust-v0.124.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.124.0^{}`.
- Triage commands: `git rev-parse rust-v0.123.0^{} rust-v0.124.0^{}`, `git rev-list --count rust-v0.123.0^{}..rust-v0.124.0^{}`, `git log --no-merges --format='%h %s %ad' --date=short <range>`, path-filtered logs over `app-server*`, `protocol`, `tui`, `core/src/{session,agent,tools/handlers/multi_agents}`, `rollout`, `config*`.
- Deep analysis commands: `git show -s --format='@@ %H%nDATE %ad%nSUBJECT %s%nBODY %B' --date=iso-strict <sha>`, `git show --stat --oneline --no-color <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included `EVID-124-wait-wrapper`: `639382609f8a96b4bf255fa2e735e8fb1aca4531`, `3cc3763e6c0473d6f6666524709528fdcdb7d9c1`. Verified stable containment includes `rust-v0.124.0`; `#18968` is also called out in release notes as the `wait_agent` mailbox fix.
- Included `EVID-124-agent-identity`: `be75785504ff152fa6333e380a2d50642f42fba0`, `69c8913e24f2f455c8f000fa7afe039a38bdd48d`. Verified stable containment includes `rust-v0.124.0`; commit bodies define revert/reintroduce stack and runtime-only identity state.
- Included `EVID-124-permission-profile`: `5eab9ff8ca5e994d68a8a9bf26f0700a937bafbc`, `18a26d7bbc9dfe7509dc0bac134af604d4f0d145`, `bc083e47137431a89d2aab1ff6f21c5b15062b49`, `6ca038bbd121b1a20d8d469ac3e57b3619a3c888`, `082fc4f632800f6aa5b0b3badcfbe3aa300f24a7`, `8bc667b07be22c005769e814bc529e30cff1ea27`, `08b5e96678331cb75a124d552d6e6c2bc5813a3a`. Verified stable containment includes `rust-v0.124.0`; diffs cover app-server schema/TS fixtures, core `TurnContext`, rollout persistence, protocol `SessionConfigured`, TUI session state and side-conversation permission preservation.
- Included `EVID-124-env-plan`: `ddbe2536be03af5e9b84f2890efba14543d6e7b5`, `1d4cc494c9c0ac94e8a1eda7acf420e5df85e24f`, `e502f0b52d490a0953579b269b1c080d62b8fb0e`. Verified stable containment includes `rust-v0.124.0`; release notes explicitly mention managed environments and per-turn environment/cwd selection.
- Excluded near-misses: `ffa6944587` (`ConfigManager`) is app-server config loading infrastructure, not direct multi-agent/collab behavior; `9d824cf4b4` is command-exec permission profile support rather than thread/fork/turn propagation; `79ea577156` keeps remote app-server events draining but is generic remote reliability; `993e3f407e` persists model default reasoning after upgrade and only touches side-thread tests incidentally; auto-review/guardian analytics commits are policy/review work without a direct multi-agent/subagent surface in this release.

### `EVID-125-*`

- Диапазон: `rust-v0.124.0^{}..rust-v0.125.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.125.0^{}`.
- Triage commands: `git rev-parse rust-v0.124.0^{} rust-v0.125.0^{}`, `git log --oneline --no-merges --date=short --reverse <range>`, `git show --stat --format=fuller <sha>` for candidates.
- Deep analysis commands: `git show -s --format='@@ %H%nDATE %ad%nSUBJECT %s%nBODY %B' --date=iso-strict <sha>`, `git show --stat --oneline --no-color <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`, plus `git show --name-status --find-renames a9c111da544c976d591343db5493a7da283b72e5` and `git show --raw a9c111da544c976d591343db5493a7da283b72e5` to confirm no local diff for `#18879`.
- Included `EVID-125-mav2-config-context`: `d3b044938d245b519c1a5baefe880ef89e3a30c1`, `a2f868c9d6a208572992da1ad7a3ea8ed00b84a1`, `d87d9187162e8db2bcc287e141afea36369a0df6`. Verified stable containment includes `rust-v0.125.0`; release notes call out MAv2 thread-limit conflict and relative agent-role config paths.
- Included `EVID-125-agent-role-layer-paths`: `d87d9187162e8db2bcc287e141afea36369a0df6`. Commands: `git show --format=fuller --stat d87d9187162e8db2bcc287e141afea36369a0df6 -- codex-rs/core/src/config/agent_roles.rs codex-rs/core/src/config/config_tests.rs`, `git show --no-color --patch --unified=80 d87d9187162e8db2bcc287e141afea36369a0df6 -- codex-rs/core/src/config/agent_roles.rs codex-rs/core/src/config/config_tests.rs`. Diff shows `layer.config_folder()` passed into `agents_toml_from_layer` and `AbsolutePathBufGuard` used while deserializing `[agents.*]`.
- Included `EVID-125-sticky-env`: `49fb25997f3c09c25684c2a729cb933939a7f830`. Verified stable containment includes `rust-v0.125.0`; diff touches app-server v2 thread/turn params, `EnvironmentSelection`, `ThreadManager`, old and v2 multi-agent spawn paths.
- Included `EVID-125-permission-profile`: `5c239ad7483ac58c475d5ff2c183f9e5b229094f`, `f90cc0ee648b1053431a92e46ec372b2a937ba44`, `ff22982d752fcf70e849c21d96b896b1b88d8988`, `9c0eced39196671646862e9c918d09535ff7bff6`, `4816b892044084de6ab5a55ea0b5854c330843fd`. Verified stable containment includes `rust-v0.125.0`; release notes mention permission profile round-trip across TUI sessions, user turns, MCP sandbox state, shell escalation and app-server APIs.
- Included `EVID-125-rollout-trace`: `6d09b6752d6a29994d97543189134c01c5c48a73`, `e3c8720a99114154929dbab950fac9fb1e1e0558`. Verified stable containment includes `rust-v0.125.0`; `a9c111da544c976d591343db5493a7da283b72e5` (`[rollout_trace] Trace sessions and multi-agent edges (#18879)`) is evidence-only because local `git show --stat`, `--name-status`, `--raw` and `--summary` show no changed files.
- Excluded near-misses: `a9c111da544c976d591343db5493a7da283b72e5` from functional entries due to empty local diff despite relevant subject/body; `e8d80808182311d62e9ae91a9a03ddbc09a4b742` model-provider discovery is broad provider architecture, not multi-agent behavior; `c10f95ddac7b35095d334dece2ebcf69bcde61fc` model fixture refresh only changes snapshots/fixtures; `6d09b6752d6a29994d97543189134c01c5c48a73` is included only for rollout-trace tool boundaries, not as proof of `#18879` multi-agent edge diff.

### `EVID-126-*`

- Диапазон: `rust-v0.125.0^{}..rust-v0.126.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.126.0^{}`.
- Triage commands: `git rev-list --count rust-v0.125.0^{}..rust-v0.126.0^{}`, path-filtered logs over app-server/protocol/TUI/core/config/tools/rollout, and `git show --name-only --pretty=format:'%h %s' <sha>` for triage candidates.
- Deep analysis commands: `git show -s --format='@@ %H%nDATE %ad%nSUBJECT %s%nBODY %B' --date=iso-strict <sha>`, `git show --stat --oneline --no-color <sha>`, `git tag --contains <sha>`, `git describe --contains <sha>`.
- Included `EVID-126-goals`: `0ee737cea69f0907effceefa5da49e5ea5d0f39f`, `6c874f9b341a8fcb01bdc35999aad1ca093afea2`, `32ace07ac57ef0c7774cbc6fe55ac089fa307868`, `4167628622a0af70374a7a6c44a547a99b5075eb`, `f1c963d77eabe02b0ebb26bea1d117ca14ffed9c`. Verified stable containment includes `rust-v0.126.0`; release notes explicitly list persistent goal workflows.
- Included `EVID-126-mav2-config`: `120aa07d81ea9f3838ddec31653d1237db11f09d`, `28742866c78cbbef0f38a652678c9a1908dfb84a`, `deb45093020f801b235cafc0ec9d30fffd49f3ff`, `1f304dd1f2c87f907aa56cbf076a846f4d013b9a`, `f8c527e5298f2cd047a12624133b24de1bf3829d`, `fd36838cf30f837a0ae66f540c49e0f85432905c`, `34d71d43eb87e16429a3945ec3de5799ea2153c0`, `857146b328009c259f65b871c1c3b1f6494c2cb2`, `70ac0f123c4b1869c9069d5b34e367b96c28bfad`. Verified stable containment includes `rust-v0.126.0`; release notes call out feature-scoped thread caps, context hints, configurable wait minimums and nested spawn behavior.
- Included `EVID-126-external-agent-migration`: `4c68bd728fe7c1c184f88ed611161e18921cf096`, `cb8b1bbcd64bc12338893f3591ab1c150105e9d4`. Verified stable containment includes `rust-v0.126.0`; diffs add external-agent session/import crates and app-server protocol migration item types for sessions/MCP/hooks/commands/subagents.
- Included `EVID-126-plan-nudge`: `c6bcd278329826c9e836099be9e24a0f84b8aa38`. Verified stable containment includes `rust-v0.126.0`; diff adds TUI composer/footer/chatwidget Plan-mode nudge tests and snapshots.
- Excluded near-misses: broad permission-profile hardening commits in this release continue the permission contract already captured in `EVID-124-*`, `EVID-125-*` and `architecture.md`; memory split, plugin marketplace, package update, provider capability, network-proxy and app-server handler streamlining are not direct multi-agent/collab/subagent functional entries.

### `EVID-127-*`

- Диапазон: `rust-v0.126.0^{}..rust-v0.127.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.127.0^{}`.
- Triage commands: `git rev-parse rust-v0.126.0^{} rust-v0.127.0^{}`, `git rev-list --count rust-v0.126.0^{}..rust-v0.127.0^{}`, `git log --reverse --pretty=format:'%h %s' rust-v0.126.0^{}..rust-v0.127.0^{}`, `git show --name-status --shortstat <sha>`.
- Deep analysis commands: `git show -s --format='@@ %H%nDATE %ad%nSUBJECT %s%nBODY %B' --date=iso-strict <sha>`, `git show --stat --oneline --no-color <sha>`, `git show --no-ext-diff --unified=60 <sha> -- <relevant paths>`, `git tag --contains <sha>`, `git describe --contains <sha>`, `git merge-base --is-ancestor <sha> rust-v0.127.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.126.0^{}`.
- Included `EVID-127-goal-resume`: `91ca551df80b77d9434587007bcc912b08915868`. Verified stable containment includes `rust-v0.127.0`; tracked as adjacent goal workflow evidence, not primary multi-agent runtime.
- Included `EVID-127-inter-agent-commentary`: `05fd90457263834535d85995023102e98f9a8be8`. Verified stable containment includes `rust-v0.127.0`; diff locks `InterAgentCommunication::to_response_input_item` commentary phase.
- Included `EVID-127-agent-graph-store`: `782191547cdc2a24a12a0bb9cef39b3e1dad1f62`. Verified stable containment includes `rust-v0.127.0`; diff adds `codex-agent-graph-store` and storage-neutral agent graph traits.
- Included `EVID-127-external-import`: `7bcd4626c4cc66620bf78744122ccb04fd63c446`, `c8abcbf9259c00a2dbc5f7d08295f12d5fad2a0b`. Verified stable containment includes `rust-v0.127.0`; diffs improve external session title/end metadata and move import completion to background notification.
- Excluded near-misses: `8356806fc9` sample ThreadManager crate and TUI dependency-removal refactors are infrastructure/sample work, not direct user-facing multi-agent behavior; `/goal resume` is retained only as adjacent long-running workflow context.

### `EVID-128-*`

- Диапазон: `rust-v0.127.0^{}..rust-v0.128.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.128.0^{}`.
- Triage commands: `git rev-list --count rust-v0.127.0^{}..rust-v0.128.0^{}`, keyword/path-filtered `git log`, `git show --stat --format=medium <sha>`, `git show <sha> -- <relevant paths>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.128.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.127.0^{}`.
- Included `EVID-128-collaboration-surface`: `fedcefe9dafcb4ee6cd74e5438a2b272f5160301`. Verified stable containment includes `rust-v0.128.0`; diff removes broad collaboration-mode coupling across thread/model/app-server/TUI/tool paths.
- Included `EVID-128-side-return`: `515aa9a4fb16f89544d065ac57e8ce01a2360929`. Verified stable containment includes `rust-v0.128.0`; diff changes TUI side-chat `Ctrl-D` behavior.
- Excluded near-misses: `8f3c06cc97` persisted hook enablement is extension/state infrastructure, not direct multi-agent behavior; `4e677d62da` bespoke event cleanup is broad app-server protocol cleanup; release rollup `e4310be51f` only bumps version/notes.

### `EVID-129-*`

- Диапазон: `rust-v0.128.0^{}..rust-v0.129.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.129.0^{}`.
- Triage commands: keyword/path-filtered logs over MAv2, realtime, app-server protocol, ThreadStore, Guardian and tool handler paths; `git show --name-only --pretty=format:'%H%n%B' <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.129.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.128.0^{}`.
- Included `EVID-129-mav2-gating-hints`: `c37f7434badbc50a4bee80148952a98f4dca36b4`, `f48b777717e09eb68ef34736d328e96f7a39e9ac`, `cc84e6bc6dca7d9f668028e71576457975d4d246`. Verified stable containment includes `rust-v0.129.0`; `#20973` interpolation is explicitly reverted by `#21337`.
- Included `EVID-129-tool-ownership`: `f593323ef18fcbbfd093668454ca277650d231b6`, `9417cf969682330ea5fcccbdf39818bc4b213167`. Verified stable containment includes `rust-v0.129.0`; diffs touch old and v2 multi-agent handlers/specs.
- Included `EVID-129-realtime`: `8a97f3cf0349a544d987e5239c2aeee6487dac3d`, `e7e6267ab3e084e2c3d9647acbd83952e4e687ad`. Verified stable containment includes `rust-v0.129.0`; diffs touch realtime websocket protocol, app-server protocol, core and TUI.
- Included `EVID-129-threadstore-guardian-graph`: `ee02cf26d684c3e00f20e19225f8e6e6c176cb62`, `7e310bc7f3c94af86c1d06d6e387054b3685ebaf`, `a8488fec5ef27216cae96e24eec2f18ef2374ed7`, `346070a424f8cebcbc42d05081ec5e3ba82965b2`. Verified stable containment includes `rust-v0.129.0`; `#20689` graph-store injection is reverted by `#21481`.
- Excluded near-misses: broad TUI resume/copy/Vim/keymap UI, plugin sharing/marketplace flows, remote-control transport, thread API identity, pagination-only items and plugin protocol metadata unless directly used by a listed multi-agent/realtime/Guardian/threadstore entry.

### `EVID-130-*`

- Диапазон: `rust-v0.129.0^{}..rust-v0.130.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.130.0^{}`.
- Triage commands: `git rev-list --count rust-v0.129.0^{}..rust-v0.130.0^{}`, path-filtered `git log`, `git show --stat --name-status <sha> -- <relevant paths>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.130.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.129.0^{}`.
- Included `EVID-130-tool-specs-handlers`: `566f2cb61220342871ea63afee2322490ecc5d0e`. Verified stable containment includes `rust-v0.130.0`; diff updates multi-agent, MAv2 and Plan handler-local specs.
- Included `EVID-130-thread-pagination-review-timing`: `0d0835dd537b57913627e877f29031151b49429e`, `99016ec732e4c231ddb5116e159a634a1436d88c`. Verified stable containment includes `rust-v0.130.0`; diffs add thread pagination contracts and protocol-native Guardian review timing.
- Excluded near-misses: plugin share metadata/hook detail, `remote-control` CLI entrypoint, remote thread-store removal and generic thread name/summary persistence are architecture background but not direct multi-agent timeline entries in this release.

### `EVID-131-*`

- Диапазон: `rust-v0.130.0^{}..rust-v0.131.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.131.0^{}`.
- Triage commands: keyword/path-filtered logs over MAv2, tool registry/spec, extension/session, TUI agent metadata and collaboration commands; `git show --name-only --stat -1 <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.131.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.130.0^{}`.
- Included `EVID-131-model-only-wait`: `fc26af377fc64cc42ff0709a9fcebefb4f5b6b80`, `7c57a59f51b605b82f49e71833aa6e675b9ec54c`. Verified stable containment includes `rust-v0.131.0`; diffs add `ToolExposure::DirectModelOnly`, `features.multi_agent_v2.non_code_mode_only` and configurable `wait_agent` timeout fields.
- Included `EVID-131-spawn-metadata-service-tier`: `90c0bec50c9439da7a9359b9eb0ec0e24add32ef`, `9e7cdbd0d28a4400a70339042e4643b4f462da6c`, `87de4e32903610b8ab66d1bda1d491936b8b0973`. Verified stable containment includes `rust-v0.131.0`; diffs touch TUI agent metadata hydration, old/v2 multi-agent handlers and spawn `service_tier`.
- Included `EVID-131-collab-cleanup`: `efdcbba053f4971132d944b918d3d81c8b786201`. Verified stable containment includes `rust-v0.131.0`; diff removes resurrected `/collab` TUI slash-command wiring.
- Excluded near-misses: broad extension registry/contributor plumbing, memory extension movement, thread metadata sync, permission profile changes and state DB recovery are architecture background but not direct user-facing MAv2/collab entries here.

### `EVID-132-*`

- Диапазон: `rust-v0.131.0^{}..rust-v0.132.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.132.0^{}`.
- Triage commands: keyword/path-filtered logs over `multiagent`, MAv2 config/spec, protocol, persistence and TUI; `git show --stat --name-only --no-color <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.132.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.131.0^{}`.
- Included `EVID-132-mav2-namespace`: `061a614d857e0f2289457770e90caf8a45b5e430`, `545ede569cff13d0aabc782ebd8cdaa6d5cb86ab`. Verified stable containment includes `rust-v0.132.0`; diffs touch `multi_agents_spec`, MAv2 config/schema, feature configs and spec-plan tests.
- Excluded near-misses: Python SDK/API changes, image detail preservation, plugin install API, goal extension skeleton/events, async extension lifecycle, permission profile cleanup, app-server settings writes and general TUI startup/resume polish are not direct multi-agent timeline entries in this release.

### `EVID-133-*`

- Диапазон: `rust-v0.132.0^{}..rust-v0.133.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.133.0^{}`.
- Triage commands: keyword/path-filtered logs over MAv2, old multi-agent tools, hooks/extensions, app-server protocol and TUI thread settings; `git show --name-only --pretty=format:'%H%n%B' <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.133.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.132.0^{}`.
- Included `EVID-133-fork-v1-service-tier`: `826b2182ed32b8534df1c44c2a82d8eeca094b58`, `b3ae3de4056f9417c968f8628cde55b428f309b8`, `05b8ce43541bbcd3ed14a21ea3f103c20cb9392b`, `c53da029bcb8afe07e4019fa13dfd46b37d7cd7d`. Verified stable containment includes `rust-v0.133.0`; diffs cover fork context baselines, deferred v1 tool exposure, namespaced v1 tools and role-defined service tier propagation.
- Included `EVID-133-subagent-hooks`: `d661ab70eda6bde758d7e48d6bf3638523cb014c`, `eee3e60db3d8ca25bb4e8dc9438e4f418430fc17`, `59507b849126f598ed7c624bbaf75d7ebc5588c2`. Verified stable containment includes `rust-v0.133.0`; release notes and diffs add `SubagentStart`, `SubagentStop` and richer turn metadata for extensions.
- Included `EVID-133-thread-settings`: `771a4e74ac319c3d8379c62c7caa3aec1ad53382`, `edc48e4612b457f2e6a32a007516fbde6fc5a832`. Verified stable containment includes `rust-v0.133.0`; diffs add app-server `thread/settings/update` and TUI synchronization of thread settings, including `collaborationMode`.
- Excluded near-misses: goals default enablement, permission profile list/inheritance, plugin marketplace/list output, remote-control foreground command and generic extension lifecycle items that do not alter subagent lifecycle or multi-agent surfaces directly.

### `EVID-134-*`

- Диапазон: `rust-v0.133.0^{}..rust-v0.134.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.134.0^{}`.
- Triage commands: keyword/path-filtered logs over hooks/extensions, MAv2 traversal, session input, thread manager and external session reset plumbing; `git show --stat --name-only <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.134.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.133.0^{}`.
- Included `EVID-134-hook-identity-history`: `16d85e270817de21e3a6afe6162b44c13df1d1c0`, `7e802b22f13e2714efd2fb2a6e396319958d6506`. Verified stable containment includes `rust-v0.134.0`; release notes and diffs expose subagent identity in hook inputs and conversation history to extension tools.
- Included `EVID-134-agent-traversal-turninput`: `5865ec45e596e67c0a1279c82d3a02e50dcaef1b`, `fbd4efa9ed6b9fe13dacd56247cc714903df72b7`, `6ad3a8350902f5825fe4172ef06cb5b506d10f3d`. Verified stable containment includes `rust-v0.134.0`; diffs keep live subtree traversal away from stale config snapshots, route session task input through `TurnInput`, and remove external client session reset plumbing.
- Excluded near-misses: local history search, profile CLI migration, MCP setup/OAuth, connector schema compaction and read-only MCP tool concurrency; these are adjacent platform improvements without direct multi-agent behavior changes.

### `EVID-135-*`

- Диапазон: `rust-v0.134.0^{}..rust-v0.135.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.135.0^{}`.
- Triage commands: keyword/path-filtered logs over forked thread metadata, rollout truncation/compaction, Guardian review and TUI/user input test paths; `git show --stat --find-renames <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.135.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.134.0^{}`.
- Included `EVID-135-fork-lineage-compaction`: `1911021c0eae972e622c8445d136ae04545a530f`, `61cbf3574eca870df6fa7f49648ec7e001901b5a`, `bee78806a9b8fad5f250f7822da08008b2c1dd01`. Verified stable containment includes `rust-v0.135.0`; diffs add `forked_from_thread_id` turn metadata and preserve compaction/truncation context for forked rollouts.
- Included `EVID-135-guardian-stability`: `e88626621bad4bb2655fa8b5fc7d2f73d68e168d`, `7df8431bbdcdd6bdbac7d2dacd5b59aca0fb7732`. Verified stable containment includes `rust-v0.135.0`; diffs stabilize Guardian review behavior around legacy notifications and test user input.
- Excluded near-misses: `codex doctor`, remote status display, Vim keybindings, `/permissions` display, packaged zsh helper and Python SDK sandbox presets.

### `EVID-136-*`

- Диапазон: `rust-v0.135.0^{}..rust-v0.136.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.136.0^{}`.
- Triage commands: keyword/path-filtered logs over Guardian cache/review flow, subagent lineage, slot naming and MAv2 tool aliases; `git show --stat --find-renames <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.136.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.135.0^{}`.
- Included `EVID-136-guardian-cache`: `c95eb3d07bd7af4977d70bddfcaac925755da4c8`, `bf4978a01f0241fc4bddecf4da68d45e867de72b`, `4ce563a87389e14072b9f6d068acb2f3e1c888a8`, `4b9eda6ff646938344bdfc718f294c6d0b2c1302`, `3307240195ece4ef772b45e5323c683f4181f1e5`, `3abf96739b543a53d2a28870eaf2a05f61fa0c15`. Verified stable containment includes `rust-v0.136.0`; diffs/cache-key subjects tighten Guardian review caching and repeated auto-review behavior around subagent-style review flows.
- Included `EVID-136-lineage-slots-rename`: `e2551a5e362eb1538e21a47c8f7babf124f6d029`, `fc9cf62efb8098fce40b5198d3e7cfc45b4ce441`, `8acaec73b6a227ae9375064e910ecf86f2620ec0`. Verified stable containment includes `rust-v0.136.0`; diffs add thread lineage handling, relax slot limits and temporarily rename `followup_task` to `assign_task`.
- Excluded near-misses: generic release/package maintenance and non-agent UI polish in the release interval.

### `EVID-137-*`

- Диапазон: `rust-v0.136.0^{}..rust-v0.137.0^{}`.
- Release verification: `git show -s --format='%H%n%ad%n%s%n%B' --date=iso-strict rust-v0.137.0^{}`.
- Triage commands: keyword/path-filtered logs over MAv2 defaults, runtime metadata, thread metadata, tool aliases, TUI spawn metadata and close/interrupt guards; `git show --stat --find-renames <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cs%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.137.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.136.0^{}`.
- Included `EVID-137-parent-thread-dogfood`: `cf0911076f234e0219bd8d61dd3bc2f80a2df287`, `8d49394febc57518479ece9e3dc5eb9060ac6968`. Verified stable containment includes `rust-v0.137.0`; release notes and diffs store/expose `parent_thread_id` and enable MAv2 dogfood defaults.
- Included `EVID-137-runtime-metadata`: `6ddb747e7687e9e6e3a2482631028c07ddc89cb6`, `3f1fb7ed8b641542add19bb841e4e4be5651693e`, `0c5ccd18abda96efaed9e94e26ffe22def5e28ed`, `bf9fd885b2546e91fe3bab271aa481b070f47518`, `3cf6f08da562ee1bf6866fbc2db45a3c3af620ff`. Verified stable containment includes `rust-v0.137.0`; release notes and diffs rename `assign_task` back to `followup_task`, add runtime metadata types, persist runtime metadata, resolve per-thread runtime and align prewarming.
- Included `EVID-137-spawn-metadata-close-guard`: `668703c23f8a6cde07c317f687d9b95606293752`, `51493157cd1fda08343e521ef47ee002009b8a40`. Verified stable containment includes `rust-v0.137.0`; diffs hide spawn metadata by default and reject MAv2 `close_agent` self-targets.
- Excluded near-misses: keybinding and paste UX, enterprise/cloud-managed config, remote-control pairing, plugin JSON/catalog output, hosted web/image tooling and generic code-mode web-search parallelism.

### `EVID-138-*`

- Диапазон: `rust-v0.137.0^{}..rust-v0.138.0^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s%n%B' rust-v0.138.0^{}`.
- Triage commands: `git rev-list --count rust-v0.137.0^{}..rust-v0.138.0^{}`, keyword/path-filtered `git log` over MAv2/Plan/subagent/tool namespace paths, `git show --stat --oneline --find-renames <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.138.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.137.0^{}`, `git describe --contains <sha>`.
- Included `EVID-138-config-namespace-prompts`: `99c9be1d30ea78fad24d9eca8daa3526592a77b5`, `11bceb8f8bfdd8723da10bdcaa767a19d92a2a57`, `8b1238856b0839cfdb345ee2af9f02e4ee2959f9`. Verified stable containment includes `rust-v0.138.0`; diffs touch MAv2 prompt/spec text, config catalog loading, tool namespace exclusion config/schema and spec-plan tests.
- Included `EVID-138-plan-idle`: `d297616d3e6a27865fb327e7b4ea3d548f7fdb45`. Verified stable containment includes `rust-v0.138.0`; release notes and diff gate automatic idle turns in Plan mode.
- Included `EVID-138-payload-delivery`: `5f4d06ef186b896d316620556e561d59206c3ebf`, `66232220e21712604b0066f496e966c9a2c1bda8`, `0b1512c2c83935af4eaa096ac7e434992f15b066`, `d5e4f01af4f7d3937ee4cd54ac27507fa421d755`. Verified stable containment includes `rust-v0.138.0`; diffs cover encrypted MAv2 payloads, v1 metadata visibility, agent-control module ownership and reload-on-delivery.
- Excluded near-misses: `9e41f8ddbe087ae6b9bdfb8d5821ab2f09f8c5fe` emits forked thread id analytics but is telemetry-only for this timeline; app handoff, image paths, reasoning effort ordering, auth/plugin JSON output, workspace instruction loading and plugin hook startup changes are not direct multi-agent functional entries.

### `EVID-139-*`

- Диапазон: `rust-v0.138.0^{}..rust-v0.139.0^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s%n%B' rust-v0.139.0^{}`.
- Triage commands: `git rev-list --count rust-v0.138.0^{}..rust-v0.139.0^{}`, keyword/path-filtered `git log` over MAv2 lifecycle, thread routing, auto-review delegation and app-server/TUI projections, `git show --stat --oneline --find-renames <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.139.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.138.0^{}`, `git describe --contains <sha>`.
- Included `EVID-139-residency-interrupt`: `4e803a017c958dd37eb251372ba810232d3e84ba`, `743f5aad38accd52da34bf4dcbdd1215a8c3ab9a`, `8d415050fce4b4ebc6da1ba247379844235fa453`, `0526cb56ac3501a02968010d03873993c319e290`. Verified stable containment includes `rust-v0.139.0`; diffs add v2 residency LRU, active-execution concurrency accounting, `interrupt_agent` rename and resume behavior for descendants.
- Included `EVID-139-thread-status-prompts`: `5a440c03f2f3393169c5df517d1fd8eee969e45e`, `9e0d7f02c9416c46dde6e571068a0fb03a4facdf`, `f9a680b9075562093ff78e45ff4fcb2e9a0348f9`. Verified stable containment includes `rust-v0.139.0`; diffs scope MCP startup status by thread, preserve auto review across delegation and calm MAv2 usage prompts.
- Excluded near-misses: web search direct calls, connector/tool schema preservation, doctor environment reports, plugin marketplace caching, HTTP window ID metadata and complete skill-read enforcement; these are platform/tooling changes without direct MAv2 lifecycle or subagent behavior.

### `EVID-140-*`

- Диапазон: `rust-v0.139.0^{}..rust-v0.140.0^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s%n%B' rust-v0.140.0^{}`.
- Triage commands: `git rev-list --count rust-v0.139.0^{}..rust-v0.140.0^{}`, keyword/path-filtered `git log` over app-server protocol, MAv2 handlers, TUI agent navigation/status feed, rollout-trace and MCP contribution paths, `git show --stat --oneline --find-renames <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --oneline --find-renames <sha>`, `git merge-base --is-ancestor <sha> rust-v0.140.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.139.0^{}`, `git describe --contains <sha>`.
- Included `EVID-140-activity-appserver`: `fae270932065355b5d7f197b3f1c72912588369b`, `1547785657607043309fbe19d826aea8ee6e40e0`, `1026e9de1be292ebb01579dfcce5a34ab224917a`. Verified stable containment includes `rust-v0.140.0`; diffs add path-based v2 activity tracking and app-server interruption/direct-input guards.
- Included `EVID-140-mcp-metrics-threadscope`: `0ffcefaf3ddb3a61d8683ca0703f7d8b39ad6c1e`, `ced1b8aa883f25b992f9ebe7d218f3709926f912`, `693082f3c4ca3b17d18989a8c5136f6258f54ac0`. Verified stable containment includes `rust-v0.140.0`; diffs isolate child MCP warnings, tag spawn metrics with multi-agent version and make MCP server contributions thread-scoped.
- Included `EVID-140-usage-plaintext`: `087035224123977defc7fda6e684088395b1d0a0`, `84520225b9928a9eee3ed2c9072fb5d26d6fc6e6`, `8f2d6416ce41be54551185c640f83e22f061eccd`. Verified stable containment includes `rust-v0.140.0`; diffs move concurrency guidance into v2 usage hints, update MAv2 prompt/spec text and add plaintext agent message support across protocol/schema/session/rollout/trace consumers.
- Excluded near-misses: remote plugin auth requirements, selected executor skills through extensions, extensible analytics feature thread sources, global instruction lifecycle characterization, hosted plugin runtime, thread delete/session delete APIs, turn diff caching, realtime AVAS/roles work and encrypted local MCP OAuth.

### `EVID-141-*`

- Диапазон: `rust-v0.140.0^{}..rust-v0.141.0^{}`.
- Release verification: `git ls-remote --tags upstream 'refs/tags/rust-v*'`, `git show -s --format='%H%n%cI%n%s%n%B' rust-v0.141.0^{}`, `git rev-list --count rust-v0.140.0^{}..rust-v0.141.0^{}`.
- Triage commands: keyword/path-filtered `git log` over agent/subagent/MAv2, Guardian/review, thread listing, MCP/executor plugins, external-agent import, response-item metadata and app-server protocol paths; `git show --stat --name-only --oneline --find-renames <sha>`.
- Deep/local verification commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --name-only --oneline --find-renames <sha>`, selected-path `git show --no-color --patch <sha> -- <paths>`, `git merge-base --is-ancestor <sha> rust-v0.141.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.140.0^{}`, `git describe --contains <sha>`.
- Included `EVID-141-prompts-wait`: `127224cacce1bc08922f21a5ff3e3ddd33246474`, `ee40dddbf6a50a6f0641180ca299be7a3a03fd22`. Verified stable containment includes `rust-v0.141.0` and excludes `rust-v0.140.0`; diffs update MAv2 default root/subagent usage hints, `spawn_agent` `fork_turns` description and `wait_agent` early return on steered user input.
- Included `EVID-141-parent-filter`: `dfd03ea01bbec2613013b477fb82abc67534a7d7`. Verified stable containment includes `rust-v0.141.0` and excludes `rust-v0.140.0`; diffs add experimental `ThreadListParams.parentThreadId`, app-server validation, thread-store `parent_thread_id` listing and focused thread-list tests.
- Included `EVID-141-selected-plugin-mcp`: `b3c423e475a9fa2bb1ad493e09408d13732d4b98`, `c8c78b63a7cbc7101884613d0301db5cc58d730d`. Verified stable containment includes `rust-v0.141.0` and excludes `rust-v0.140.0`; diffs add executor-plugin MCP discovery/provider state and app-server activation with thread-scoped selected-root tests.
- Included `EVID-141-guardian-external-import`: `a18de1f3b6482cf162a473d173db7bf24206333e`, `fc1fb682a7b94b8146ee1ce6b96f278669374988`. Verified stable containment includes `rust-v0.141.0` and excludes `rust-v0.140.0`; diffs isolate Guardian review sessions from skill/plugin/memory injection and add external-agent import IDs plus type-level success/failure accounting.
- Included `EVID-141-response-metadata`: `040dafa32d5312f7803786d208ed6a0a11dfdabb`. Verified stable containment includes `rust-v0.141.0` and excludes `rust-v0.140.0`; diffs add optional `ResponseItemMetadata`, preserve metadata through response/history/rollout paths and attach optional metadata to `InterAgentCommunication`.
- Excluded near-misses: `a292faae5a4b1bc532d371e9cb0e1b343f3e4ba0` and `11faf9af94fa345ba090701f071c8d3107fbca45` change generic dynamic-tool namespace representation and app-server `thread/start.dynamicTools` wire shape; they are relevant to tool registry compatibility but do not directly change multi-agent/subagent behavior. `08901fc8e127f1f2ed08fbda1bcc9870046735eb` adds a generic interruptible sleep tool that wakes on user/mailbox input, but it is a separate feature-gated tool rather than MAv2 `wait_agent` or subagent lifecycle behavior.

### `EVID-142-*`

- Диапазон: `rust-v0.141.0^{}..rust-v0.142.0^{}`.
- Release verification: `git show -s --format='%H%n%cI%n%s%n%B' rust-v0.142.0^{}`, `git rev-parse rust-v0.141.0^{} rust-v0.142.0^{}`, `git rev-list --count rust-v0.141.0^{}..rust-v0.142.0^{}`.
- Triage commands: Spark background threads plus local keyword/path-filtered logs over `agent`, `subagent`, `multi_agent`, `MAv2`, `mailbox`, `Guardian`, rollout budget, external-agent import, app-server protocol and thread/turn multi-agent mode paths; local synthesis narrowed MCP/plugin noise out of this package.
- Deep/local verification commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --name-only --oneline <sha>`, selected-path `git show --no-color --patch <sha> -- <paths>`, `git merge-base --is-ancestor <sha> rust-v0.142.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.141.0^{}`, `git describe --contains <sha>`.
- Included `EVID-142-mav2-messages`: `5b22a8e5b13bd4bc3b331e7a1392569107b7bccf`, `45f603302c45269737db97443612bb4876365798`, `1b24ba912ac4c56ef936364deb1c3e294b0ef9fa`. Verified stable containment includes `rust-v0.142.0`; diffs cover typed MAv2 message envelopes, `ResponseItemMetadata` join keys and terminal child-agent errors surfaced to parent agents.
- Included `EVID-142-multi-agent-mode`: `fc8c6b73841e279f95f53b08771a7969e953bdf4`, `7abfcf220bbb57029e2ff5d9914124aef7ef3d0f`, `c03742ca0a78a8e54cd881032a2327363678b5aa`. Verified stable containment includes `rust-v0.142.0`; diffs add per-turn and thread-level `MultiAgentMode`, app-server projections and simplified runtime/config controls.
- Included `EVID-142-rollout-budget`: `ecc4c30e281a9dff77ab45e0365c73a2a526a520`, `32a696dbacaa1383745455ea2a77d5477891ed0b`, `dac588f41398e8b628d71838d5745dad430477f1`, `bd5bd953fb2a5d610a30d112eafe20b644924085`. Verified stable containment includes `rust-v0.142.0`; diffs add rollout-budget config/schema, runtime accounting, exhausted-budget aborts and reminder thresholds.
- Included `EVID-142-guardian-import`: `15f448d8b06c25c9ad04b2abeb3e4e2f07e4c327`, `314fa3d25b1f8a2542ddefa72a8cdb706e9ee3c6`. Verified stable containment includes `rust-v0.142.0`; diffs start the Guardian child session with the parent session and add external-agent import progress/completion/type-level result accounting.
- Excluded near-misses: `5b95745eae1b13be75d6c395a6baec8e7b7ad99b` renames response metadata passthrough and touches MAv2 tests, but the selected `EVID-142-mav2-messages` entries already cover the functional MAv2 message contract; `8f8de7844f9a95ba0f07b9ae584b5c08b3173dd6` restores `thread_source` in turn metadata and is better tracked in `thread/*` evidence as resume/client-metadata support; `6bfc58a688a73c287da51797d99c3345caecbae8` updates Plan-mode prompt text without changing multi-agent runtime; `21d36296f137c0954df24ea86abe9619318915e6` adds workspace messages app-server API outside subagent/MAv2 delivery semantics.

### `EVID-142.5-*`

- Диапазон: `rust-v0.142.0^{}..rust-v0.142.5^{}`.
- Release verification: `git rev-parse --short rust-v0.142.0^{}`, `git rev-parse --short rust-v0.142.5^{}`, `git rev-list --count rust-v0.142.0^{}..rust-v0.142.5^{}` returned `28`.
- Triage commands: Spark research thread plus local keyword/path-filtered scans over `agent`, `subagent`, `multi-agent`, `multi_agent`, `MAv2`, `Guardian`, `delegation`, `world state`, `turn_id`, `tool_search`, app-server thread/turn protocol and core session/agent paths.
- Deep/local verification commands: `git show -s --format='%H%x09%cI%x09%s' <sha>`, `git show --stat --name-only --oneline <sha>`, selected-path `git show --no-color --patch <sha> -- <paths>`, `git merge-base --is-ancestor <sha> rust-v0.142.5^{}`, `git merge-base --is-ancestor <sha> rust-v0.142.0^{}`, `git describe --contains <sha>`.
- Included `EVID-142.5-world-state-subagents`: `3b32d861c5ecc1e705812bc21177d0038e3c05ee`; supporting stabilization `97dc6abb11a1e5e15acb5b8a499e853418ea4902`. Verified first `rust-v0.142.*` containment is `rust-v0.142.2`; diffs add `WorldState`/`EnvironmentsState`, render `<subagents>` through environment world state and move the diff baseline into `ContextManager`.
- Included `EVID-142.5-turn-id-inter-agent`: `4a82ecc3c9fcb8b9f21d5144c40a0dfdab54a02a`. Verified first `rust-v0.142.*` containment is `rust-v0.142.2`; diffs stamp durable response/inter-agent items with `internal_chat_message_metadata_passthrough.turn_id` and preserve metadata across persistence, resume/fork, compaction and websocket reuse.
- Included `EVID-142.5-tool-search-v1-agents`: `c53b1dae09db40902c59f6a0d57d0dcc334926db`. Verified first `rust-v0.142.*` containment is `rust-v0.142.2`; diffs make search-tool exposure the default supported path and route V1 collaboration tools through the simplified deferred exposure condition.
- Included `EVID-142.5-ultra-mav2-mode`: `aedb8f345a221b22105daad9fdf080988026fbca`. Verified first `rust-v0.142.*` containment is `rust-v0.142.2`; diffs add `ReasoningEffort::Ultra`, derive effective MAv2 mode from reasoning effort, remove selected multi-agent mode from core session/thread/spawn plumbing and deprecate app-server `multiAgentMode` request fields as accepted-but-ignored compatibility wire.
- Included `EVID-142.5-v1-delegation-guidance`: `3b8b60a58399ff79a41f8457710ae5a5fae5c39f`. Verified stable containment includes `rust-v0.142.5` and excludes `rust-v0.142.0`; diffs restore V1 `spawn_agent` model-visible guidance for authorization, sidecar delegation and critical-path local work.
- Excluded near-misses: `f1945de3b7` wraps token-budget context but does not change rollout-budget agent accounting; `7153affa0f` and `b294638bb5` handle remote image errors/rejection outside agent-specific behavior; `07f7032383` reverts auto-review prompt behavior without Guardian runtime changes; `7c22d376e5` safety-buffering metadata is turn-notification platform work; plugin catalog/sharing, PAC/system proxy, OpenSSL/esbuild, Bedrock model catalog and WebSocket trace backport do not directly change multi-agent/subagent/Guardian/external-agent contracts selected here.

### `EVID-143-*`

- Правосторонняя дельта: `rust-v0.142.5^{}..rust-v0.143.0^{}` содержит `258` коммитов. Топология symmetric difference: left/right `6/258`; merge-base `27f22b54aef4d7e5eb6c564e969c961c74605461`.
- Разбор left-only: `aedb8f345a221b22105daad9fdf080988026fbca` и `3b8b60a58399ff79a41f8457710ae5a5fae5c39f` уже учтены в `EVID-142.5-*`; `a2325b50ff6003e18d549038eee1b21e9a645df1`, `07f70323839c8ecb4312fafd4401fc3021307bb2`, `e019402a9e848c770e4d5a36accbef6d60c8b6e4` и release anchor `26de83050b20f7e0ee211b9739e52ae00ce8032a` не являются частью правосторонней дельты `rust-v0.143.0` и не создают новых записей этого релиза.
- Проверка релиза: `git rev-parse 'rust-v0.142.5^{}' 'rust-v0.143.0^{}'`; `git show -s --format='%H%n%cI%n%s%n%B' 'rust-v0.143.0^{}'` вернул release commit `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda` с датой `2026-07-07T17:43:00-07:00`.
- Команды triage: `git log --reverse --format='%H%x09%cI%x09%s' 'rust-v0.142.5^{}..rust-v0.143.0^{}'`; поиск по subject через `rg -i 'agent|subagent|multi-agent|multi agent|collab|guardian|reviewer|thread|world state|world_state|AGENTS.md|delegation|rollout'`; `git log` с фильтром по путям MAv2 handlers/specs, agent control/graph store, session/world-state, protocol/items, rollout/thread-store/state, app-server thread protocol/processors и TUI projections.
- Команды углублённой локальной проверки: `git show -s --format='%H%n%cI%n%s%n%b' <sha>`, `git show --stat --name-only --oneline --find-renames <sha>`, `git show --no-color --patch <sha> -- <paths>` по выбранным путям, `git merge-base --is-ancestor <sha> rust-v0.143.0^{}`, `git merge-base --is-ancestor <sha> rust-v0.142.5^{}`, `git describe --contains <sha>`, а также `git show 'rust-v0.144.1^{}:<path>'` и `git blame 'rust-v0.144.1^{}' -L <range> -- <path>` для проверки утверждений текущей архитектурной карты.
- Включено в `EVID-143-namespace-policy`: `49614a0391d83eec442ffeca1d4aa0fdeb119818`, `79a8ffdbf7ca08820e0b44cab0261eb61ff72465`, `da4c8ca57d40b074bdc1b5b1218851100150c56b`. Containment включает `rust-v0.143.0` и исключает `rust-v0.142.5`. Diffs задают resolved default MAv2 namespace `collaboration`, обновляют текст разрешения делегирования через AGENTS.md/skill и добавляют persisted custom multi-agent hint. Source `rust-v0.144.1` сохраняет `MultiAgentV2ConfigToml.tool_namespace`, поэтому корректное утверждение — default namespace с explicit override, а не безусловно фиксированный namespace.
- Включено в `EVID-143-agent-message-persistence`: `97dce078c57895d9cc184be923dc26b102e19493`, `b4f0f3eff1303ea445a218afb039e0079df0afce`. Containment включает `rust-v0.143.0`; diffs назначают и сохраняют IDs `amsg_`, напрямую сохраняют подготовленный `ResponseItem::AgentMessage` и оставляют legacy inter-agent rollout compatibility.
- Включено в `EVID-143-agent-graph-descendants`: `8057603d0c70930fe096302227f582b5b94496d8`, `ece1dfece07650458f84f3d2b35ffbde29ae48b7`. Containment включает `rust-v0.143.0`; diffs добавляют `ancestorThreadId`, recursive state-DB traversal по `thread_spawn_edges`, explicit injection `AgentGraphStore` и graph-backed операции spawn/close/resume/subtree.
- Включено в `EVID-143-world-state-agents-md`: `3e51b46eba036364a735f450c2bb4d7c35d48d2b`, `fa036d39aadb160f819198d512efd1a2151a761b`, `a74771340db6eb81db39e81445a706346f14c139`, `f2f80ef442ff84612004bad064991283eb17cdb5`. Containment включает `rust-v0.143.0`; diffs определяют stable serializable world-state sections/snapshots, persist/replay full и merge-patch records и добавляют environment-reactive AGENTS.md как bounded world-state section. Snapshot/render `EnvironmentsState` включает cwd, permission-derived filesystem text и `<subagents>`.
- Включено в `EVID-143-mav2-communication`: `a98a21798c3301cfbeb6c323d6c0f6a804e08a57`, `129ea2aaf5fb426d8ba683ee53f290742f41dd31`. Containment включает `rust-v0.143.0`; diffs направляют все виды MAv2 communication через `submit_inter_agent_communication` и эмитят correlated send/receive records `codex_otel.agent_communication`.
- Включено в `EVID-143-canonical-item-foundation`: `a107b84967eb9a3444fd2d4de03f200337acd52b`, `b9b934e99b57aa1908a7cd83adee9faf6d0b8f7d`. Containment включает `rust-v0.143.0`; diffs добавляют canonical collab/sub-agent types `TurnItem`, поддержку app-server conversion/history и centralized legacy-event fanout. Producer migration намеренно отнесена к `EVID-144-canonical-agent-items`.
- Включено в `EVID-143-guardian-approvals`: `50eee505a3bbd7a66a73360ae28ea4968f79dd36`, `aa94ea139744f0a9dd76343805cc96649d9f2658`. Containment включает `rust-v0.143.0`; diffs ограничивают connected-account email trusted built-in metadata app review и переносят shell/apply-patch review routing за generic approval abstraction.
- Исключённые near-misses: `330ae6a516009e127db01843f5049497d3c46805` разделяет resumed history через `Arc`, но сохраняет прежнее serialized behavior; `d1d11cac0556ce678750b791e49e45162ab4fb05` меняет общий ownership world state при inline compaction; `4cc6a4bab5c39021ff28d73ab2f1631721f1d847` использует environments текущего step для tools; `31b99f65cf4b3eaf9180f625b48b5614806cd121` переименовывает CLI flag permission profile; `c65cfeab1478c4c8356b42aca3d06c92b2730334` передаёт permission profile shell tools; `39aab9fc45cc639d1caa5a2a999ce7f7260d7c34` объединяет bounded probes AGENTS.md/Git в pipeline; `723b23efd0c06289e878b294b29f6d7ed98a66e8` выполняет общий reinjection WorldState при resume; `dbf67f34a0a37be77d12e2801575f33946f7d629` эмитит общие warnings thread config. Это supporting-изменения runtime/environment/permissions без отдельной прямой дельты multi-agent/subagent contract сверх уже включённых записей.

### `EVID-144-*`

- Правосторонняя дельта: `rust-v0.143.0^{}..rust-v0.144.0^{}` содержит `80` коммитов. Топология symmetric difference: left/right `1/80`; merge-base `6afcf26d5d76c2f88b9096caa758931ffa673745`.
- Разбор left-only: release anchor `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda` остаётся только на стороне `rust-v0.143.0`; это якорь предыдущего релиза, а не функциональный commit для повторного включения в `rust-v0.144.0`.
- Проверка релиза: `git rev-parse 'rust-v0.143.0^{}' 'rust-v0.144.0^{}'`; `git show -s --format='%H%n%cI%n%s%n%B' 'rust-v0.144.0^{}'` вернул release commit `767822446c7a594caa19609ca435281a9ec67e0d` с датой `2026-07-09T08:59:13-07:00`.
- Команды triage и углублённой проверки: те же reverse log, поиск по subject/paths, `git show`, проверки containment и `git describe --contains`, что и для `EVID-143-*`, с фокусом на producers/persistence canonical items, TUI warning для Ultra и paths Guardian review/resume.
- Включено в `EVID-144-canonical-agent-items`: `1bd9d841ca9ec8c99e4843f5be3d1d71cb5d8169`, `058d97c5dcf572ef7084e488bbb108d24aa19e54`, `6b4882528eaf2d927904bba37c48e40d925c35d1`. Containment включает `rust-v0.144.0` и исключает `rust-v0.143.0`; diffs мигрируют MAv2 activity, V1 non-wait lifecycle и V1/V2 wait на canonical production `TurnItem`, а app-server использует canonical completion для watcher effects и игнорирует duplicate legacy fanout.
- Включено в `EVID-144-paginated-turn-items`: `2342b2c2a66b5ac2a07278afb4f5f38700be452e`. Containment включает `rust-v0.144.0`; diff делает rollout persistence history-mode-aware, сохраняет completed canonical items для paginated threads, сохраняет legacy policy для legacy threads и передаёт forks source mode.
- Включено в `EVID-144-ultra-concurrency-warning`: `927004c06dc55565af17f0bc8eeb5e35fb990351`. Containment включает `rust-v0.144.0`; release notes и diff добавляют explicit Ultra warning при configured concurrency threshold `8`, включая thread и вычисленный subagent counts на picker/shortcut surfaces.
- Включено в `EVID-144-guardian-reviewer`: `3eb56537eb598e157af8b415f6236c27b56b378f`, `0746e8a34574b4bf4721672c97fc6a94fd8bfad8`. Containment включает `rust-v0.144.0`; diffs сужают Guardian prompt/tools и persist/restore effective reviewer, если resume не задаёт explicit override.
- Исключённые near-misses: `0bbea86a6aae37b1f243676db4248000f04ad111` и `8dfd3975f5a7c3847dde13f80a949e8fab0d3bae` стабилизируют только rollout-budget и encrypted MAv2 tests; `4e270ddec40939dcdfb0e8f3e1c824ec122ea9da` меняет пути app-server fixtures; `20e5edfa74d44b2e767fa52a1d3769a4f65d49fa` меняет только CODEOWNERS; `ba5dd1fd3acd8802325945b88219e69e8b744c40` мигрирует hook prompt items, а не agent lifecycle; `23aac925e7b3fd02b9d6a47147b7b8310a124b49` добавляет canonical review-mode items, но само по себе не меняет Guardian subagent behavior; `bdaad6820cd884ea11787477f4c495e4de0a8be5` повторно использует MCP snapshot в общем sampling path. Эти коммиты не создают отдельной прямой multi-agent записи.

### `EVID-144.1-no-change`

- Правосторонняя дельта: `rust-v0.144.0^{}..rust-v0.144.1^{}` содержит `4` коммита, включая release anchor. Топология symmetric difference: left/right `1/4`; merge-base `3380969a29134630d56feb6218e8e8dcc5e8196d`.
- Разбор left-only: release anchor `767822446c7a594caa19609ca435281a9ec67e0d` остаётся только на стороне `rust-v0.144.0` и не является частью правосторонней дельты patch-релиза.
- Проверка релиза: `git show -s --format='%H%n%cI%n%s%n%B' 'rust-v0.144.1^{}'` вернул `44918ea10c0f99151c6710411b4322c2f5c96bea` с датой `2026-07-09T15:10:27-07:00`.
- Команда полной правосторонней дельты: `git log --reverse --format='%H%x09%cI%x09%s' 'rust-v0.144.0^{}..rust-v0.144.1^{}'` вернула `7ef1728763f5b6e346158a74e365ab6fbb48332c` (installer metadata parsing), `9d47eb221ddd275e3936a76fb2f79efa21f4911e` (Darwin code-mode installation), `77c42a202aa7e89189c1126021dcc054b1795e1d` (embedded V8 fallback) и release commit.
- Результат: прямых изменений multi-agent/collab/subagent/Guardian contract, lifecycle, permissions, environment, persistence, listing или UI projections нет.

## Evidence gaps

- Primary spark triage for `rust-v0.133.0`-`rust-v0.140.0` was attempted in release-sized batches but blocked by the `GPT-5.3-Codex-Spark` usage limit during that run; the recorded entries were locally verified with `git show`, `git show --stat`, `git merge-base --is-ancestor`, `git describe --contains` and release-note/changelog subjects instead.
- `#18879` in `rust-v0.125.0` has a relevant subject/body about multi-agent rollout-trace edges but no local changed files; it remains evidence-only unless GitHub PR metadata is reviewed later.
- Для `why` вне commit/release note/diff внешняя PR metadata не использовалась; где локального evidence недостаточно, timeline пишет `не установлено локально`.
