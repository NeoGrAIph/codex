# Quality gates и проверки

## 1) Базовые обязательные проверки

Из `justfile` и CI (`.github/workflows/rust-ci.yml`) следует минимальный набор:

1. Форматирование:

```bash
cd codex-rs
just fmt
```

2. Lint/fix (по измененному crate):

```bash
cd codex-rs
just fix -p <crate>
```

3. Тесты по затронутому crate:

```bash
cd codex-rs
cargo test -p <crate>
```

4. При необходимости полный прогон:

```bash
cd codex-rs
just test
# или
cargo test --all-features
```

## 2) Что реально проверяет CI

`rust-ci` включает:

1. `cargo fmt --check`
2. `cargo shear`
3. `cargo clippy --all-features --tests -D warnings` (матрица таргетов)
4. `cargo check` по отдельным crate (x86_64-unknown-linux-gnu dev)
5. `cargo nextest run --all-features --no-fail-fast` (матрица тестов)

Следствие: локально минимум нужно проходить эквивалентные проверки для измененной зоны, иначе риск CI-red высокий.

## 3) Спец-правила для изменений по зонам

### A. Изменяли `core/src/config*` или `ConfigToml`

1. Регенерация schema:

```bash
cd codex-rs
just write-config-schema
```

2. Проверить, что изменился только ожидаемый generated файл:

- `codex-rs/core/config.schema.json`

### B. Изменяли app-server API / protocol

1. Обновить API в `v2` (`common.rs` / `v2.rs`).
2. Регенерация схем:

```bash
cd codex-rs
just write-app-server-schema
```

3. Обязательные тесты:

```bash
cd codex-rs
cargo test -p codex-app-server-protocol
cargo test -p codex-app-server
```

4. Обновить документацию API:

- минимум `codex-rs/app-server/README.md`.

### C. Изменяли TUI

1. Тесты:

```bash
cd codex-rs
cargo test -p codex-tui
```

2. Если изменился рендер:

```bash
cd codex-rs
cargo insta pending-snapshots -p codex-tui
# точечный просмотр
cargo insta show -p codex-tui <path/to/file.snap.new>
# принимать только если изменение намеренное
cargo insta accept -p codex-tui
```

### D. Изменяли `common`, `core` или `protocol`

С точки зрения риска интеграции нужен расширенный прогон:

```bash
cd codex-rs
cargo test --all-features
```

## 4) Проверка чистоты diff перед merge

Минимальный pre-merge checklist:

1. `git diff --stat`
2. `git diff --name-only`
3. Нет незапланированных правок в:
   - `.github/workflows/*`
   - `vendor/*`
   - `MODULE.bazel.lock`
   - массовых generated-файлах вне ожидаемой зоны

## 5) Definition of Done для новой фичи

Фича готова, если:

1. Поведение реализовано и закрыто тестами на затронутом уровне.
2. Локальные проверки из раздела 1-3 прошли.
3. Generated файлы обновлены только через штатные команды.
4. Документация обновлена там, где изменился внешний контракт.
5. Diff минимален и не содержит несвязанных изменений.

## 6) Команды для воспроизводимого baseline-а

Перед стартом и перед merge полезно фиксировать:

```bash
git rev-parse HEAD
git status --short --branch
git diff --name-only rust-v0.99.0..HEAD
```

