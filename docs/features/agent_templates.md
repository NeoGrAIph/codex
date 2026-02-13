# AgentTemplates (SA)

## 1) Feature Passport
- **Code name:** AgentTemplates (`SA`)
- **Status:** implemented
- **Goal:** расширить `spawn_agent` так, чтобы `templates/agents/<agent_type>.md` мог задавать:
  - дефолтные `model`/`reasoning_effort` (если override не передан при спавне),
  - личности `agent_names` с персональными промптами,
  - description для `agent_type` и/или `agent_name`, видимую в tool schema `spawn_agent` (для выбора агента).
- **Scope in:** `codex-rs/core` (tool spec + collab handler + template parsing), `codex-rs/protocol` (metadata), `codex-rs/tui` (SAW display)
- **Scope out:** `.codex/agents` registry (project/user/system), отдельный `list_agents` tool, изменения публичного API app-server
- **API impact:** внутренние параметры `spawn_agent` расширены (`agent_name`, `model`, `reasoning_effort`)
- **Security/policy impact:** none (только выбор профиля/инструкций; tool allow/deny остаются как в базовой политике)
- **Upstream baseline:** 0.99 (fork work)

## 2) User Contract

### 2.1) `spawn_agent` inputs
Поддерживаются новые опциональные поля:
- `agent_type`: как раньше (built-in или template-backed).
- `agent_name`: опциональная личность внутри `agent_type` (если template определяет блоки).
- `model`: опциональный override модели для spawned agent.
- `reasoning_effort`: опциональный override (`none|minimal|low|medium|high|xhigh`).

Приоритеты:
1. explicit overrides из `spawn_agent` (`model`, `reasoning_effort`)
2. дефолты из YAML-frontmatter template
3. наследование от текущего turn/config (как было раньше)

### 2.2) Template format (`codex-rs/core/templates/agents/<agent_type>.md`)

Опциональный YAML frontmatter:
```yaml
---
description: |
  Use for ...
model: gpt-5.1-codex-mini
reasoning_effort: medium
allow_list:
  - shell
  - read_file
deny_list:
  - apply_patch
agent_names:
  - name: strict
    description: Strict mode
---
```

Опциональные blocks личностей:
- дефолтный промпт: текст до первого `<!-- agent_name: ... -->`
- персональный промпт: блок, начинающийся с `<!-- agent_name: <name> -->`

Default selection:
- если `agent_name` не передан:
  - берётся дефолтный блок, если он непустой
  - иначе, если есть ровно один `agent_name` block, берётся он
  - иначе `spawn_agent` возвращает ошибку “requires agent_name selection”

Validation:
- если YAML содержит `agent_names`, то blocks должны быть 1:1 (без лишних/пропущенных).
- `allow_list` и `deny_list` нормализуются (trim/dedup) и применяются как runtime-фильтр tools для spawned agent данного `agent_type`.

### 2.3) SAW (`/ A G E N T S /`) отображение роли
Если source содержит `agent_type` и `agent_name`, роль отображается как:
- `agent_type/agent_name`
Иначе:
- `agent_type` или legacy `subagent`.

## 3) Implementation Map

Файлы:
- `codex-rs/core/src/agent/role_templates.rs`
  - compile-time templates (`include_dir`)
  - парсинг frontmatter + `agent_name` blocks
  - `list_summaries()` для tool hint
  - `get_parsed()` для выбора инструкций + дефолтов
- `codex-rs/core/src/tools/spec.rs`
  - расширение schema `spawn_agent` параметрами `agent_name/model/reasoning_effort`
  - подсказка по template-backed ролям + descriptions
- `codex-rs/core/src/tools/handlers/collab.rs`
  - расширение args `spawn_agent` и применение приоритетов overrides/defaults
  - прокидывание `agent_name` в `SubAgentSource::ThreadSpawn`
- `codex-rs/protocol/src/protocol.rs`
  - `SubAgentSource::ThreadSpawn { agent_type: Option<String>, agent_name: Option<String> }` (backward compatible)
- `codex-rs/tui/src/app.rs`
  - SAW role formatting с учётом `agent_name`

Fork markers:
- template logic: `[SA] COMMIT OPEN/CLOSE`
- SAW metadata: `SAW COMMIT OPEN/CLOSE`

## 4) Verification Matrix
Минимум:
```bash
cd codex-rs
just fmt
just fix -p codex-core
cargo test -p codex-core --lib
cargo test -p codex-protocol
```
TUI (если нужно проверить SAW вручную):
```bash
cd codex-rs
cargo test -p codex-tui ctrl_t_overlay_action_cycles_transcript_and_agents_overlays
```

## 5) Doc Changelog
- `2026-02-12`: добавлен AgentTemplates (SA): YAML-frontmatter defaults + `agent_names` personalities + descriptions в `spawn_agent`.
