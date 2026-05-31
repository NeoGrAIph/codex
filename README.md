# Codex CLI — NeoGrAIph Fork

> **Status:** public upstream-compatible fork and maintainer workspace for Codex CLI.
> **Upstream:** `openai/codex`
> **Maintainer:** `NeoGrAIph`
> **Scope:** local development, bug reproduction, security review, regression testing, and upstream-oriented analysis.

This repository is a personal public fork of OpenAI's Codex CLI. It is not an official OpenAI distribution and does not claim to replace the upstream project.

The purpose of this fork is to maintain an isolated, reproducible workspace for studying Codex CLI behavior, testing local changes, validating security-sensitive workflows, and preparing high-quality feedback, bug reports, issue analyses, or invited upstream patches for the official `openai/codex` project.

---

## Why this fork exists

Codex CLI is a local coding agent that runs on a developer machine, interacts with repositories, executes tools, and assists with software engineering tasks. Because of that, even small changes can affect developer workflow, local execution behavior, repository access, security boundaries, and user trust.

This fork is used as a controlled workspace to:

* reproduce Codex CLI bugs in a local environment;
* test behavior changes before proposing them upstream;
* analyze regression risks in CLI, TUI, configuration, authentication, sandboxing, and tool-execution flows;
* prepare minimal, focused patches when an upstream contribution is invited;
* generate high-quality issue reports with reproduction steps, root-cause hypotheses, and implementation notes;
* validate security-sensitive changes before they are shared more broadly;
* keep experiments separate from the official upstream repository.

This repository is intentionally upstream-compatible. Changes should remain easy to review, compare, rebase, or discard.

---

## Relationship to upstream

This fork follows the upstream `openai/codex` repository as its source of truth.

The intended contribution path is:

1. reproduce and understand a bug or improvement area locally;
2. isolate the smallest meaningful change or behavioral observation;
3. validate the change with formatting, tests, and security review where applicable;
4. document the reasoning clearly;
5. share feedback upstream through issues, discussions, or an invited pull request if the upstream maintainers request one.

This fork does not represent an independent product roadmap. Its value is in careful analysis, reproducible experiments, and upstream-oriented maintenance work.

---

## Maintainer role

`NeoGrAIph` is the primary maintainer of this fork.

Maintainer responsibilities for this repository include:

* keeping the fork reviewable and aligned with upstream;
* maintaining local branches for focused experiments;
* avoiding unrelated or noisy changes;
* validating proposed changes before publishing them;
* documenting maintenance decisions;
* reviewing changes for regressions and security impact;
* ensuring that any security review is limited to authorized repositories, branches, and codebases.

---

## Maintenance workflow

The maintenance workflow for this fork prioritizes small, auditable changes.

Before any change is considered ready for wider review, it should have:

* a clear problem statement;
* reproduction steps or a concrete use case;
* a minimal implementation scope;
* formatting/lint checks where applicable;
* targeted tests or a documented reason why tests are not applicable;
* regression analysis;
* security review for changes touching execution, permissions, authentication, configuration, file access, networking, or repository operations.

Recommended local checks:

```bash
# Format
just fmt

# Apply automatic fixes to a touched crate
just fix -p <crate-you-touched>

# Run a targeted test
cargo test -p <crate-you-touched>

# Run a broader test sweep when appropriate
just test
```

For Rust-specific changes, use the relevant Cargo workspace commands from the `codex-rs` directory.

---

## Security review workflow

Security review is a first-class reason for maintaining this fork.

Codex CLI is a local agent, so security-sensitive areas include:

* tool execution;
* sandboxing and approval flows;
* file-system access;
* repository operations;
* authentication and token handling;
* configuration parsing;
* plugin behavior;
* network access;
* prompt/tool boundary handling;
* unintended disclosure of local or repository data.

The security workflow for this fork is:

1. scan only repositories, branches, and codebases that the maintainer owns or is authorized to review;
2. prefer read-only review before making changes;
3. validate findings manually before treating them as vulnerabilities;
4. avoid publishing exploit details in public issues;
5. prepare minimal fixes or responsible reports;
6. document the affected component, impact, reproduction path, and proposed mitigation;
7. route upstream-sensitive findings through the appropriate responsible disclosure process.

This repository should not be used to scan, probe, test, or review third-party repositories or systems without authorization.

---

## Codex Security use case

This fork is a suitable candidate for Codex Security because it is used to review a local coding agent with security-sensitive execution behavior.

Codex Security would be used for:

* repository-wide review of this fork;
* diff review before merging local branches;
* identifying security regressions in tool execution and sandboxing flows;
* reviewing changes that affect authentication, configuration, file access, or command execution;
* validating findings before acting on them;
* preparing minimal fixes or issue-quality reports.

The intended use is defensive and maintenance-oriented. The scope is limited to this fork, branches controlled by the maintainer, and other repositories where explicit authorization exists.

---

## API credits use case

If API credits are available through an open-source maintainer program, they would be used only for OSS maintainer workflows around this fork.

Planned uses include:

* automated review of local branch diffs;
* test generation for changed components;
* regression analysis;
* documentation review;
* release and packaging checks;
* reproducing reported issues;
* comparing behavior across Codex CLI surfaces;
* validating Codex-compatible IDE and API workflows;
* preparing upstream issue reports or invited patches.

API credits are not intended for unrelated personal workloads or proprietary product development.

---

## Current project status

This fork is currently an early-stage maintainer workspace.

The near-term focus is not to publish a separate distribution, but to create a transparent and reviewable environment for:

* local Codex CLI experiments;
* security analysis;
* reproducible bug reports;
* small upstream-compatible changes;
* documentation improvements;
* validated feedback for the official project.

No release from this fork should be treated as an official OpenAI release unless explicitly stated otherwise.

---

## Roadmap

Planned maintenance work for this fork:

* keep the fork synchronized with upstream;
* document local experiments and their rationale;
* add issue templates for bug reproduction and security-sensitive analysis;
* add a clear security policy for this fork;
* track candidate improvements in focused branches;
* validate local changes with tests and formatting checks;
* prepare upstream-oriented reports and patches where appropriate;
* document any divergence from upstream.

Potential areas of analysis:

* sandboxing and approval UX;
* configuration safety;
* local execution boundaries;
* model metadata handling;
* unsupported modality behavior;
* CLI/TUI reliability;
* logging and diagnostics;
* IDE/API-key workflows;
* security review ergonomics.

---

## Installation and usage

For normal Codex CLI usage, prefer the official OpenAI package and documentation.

### Installing Codex CLI

Install globally with your preferred package manager:

```shell
# Install using npm
npm install -g @openai/codex
```

```shell
# Install using Homebrew
brew install --cask codex
```

Then run:

```shell
codex
```

### Using Codex with your ChatGPT plan

Run `codex` and select **Sign in with ChatGPT**.

You can also use Codex with an API key, but this requires additional setup in accordance with the official Codex documentation.

---

## Working with this fork

To work directly with this fork:

```bash
git clone https://github.com/NeoGrAIph/codex.git
cd codex
```

If the Rust workspace is under `codex-rs`, enter it before building:

```bash
cd codex-rs
```

Install the Rust toolchain if necessary:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt
rustup component add clippy
```

Install helper tools:

```bash
cargo install just
cargo install cargo-nextest
```

Build Codex:

```bash
cargo build
```

Run the TUI with a sample prompt:

```bash
cargo run --bin codex -- "explain this codebase to me"
```

Run checks before publishing changes:

```bash
just fmt
just fix -p <crate-you-touched>
cargo test -p <crate-you-touched>
just test
```

---

## Contribution policy

This fork is primarily a maintainer workspace.

External contributions to this fork may be considered when they are:

* focused;
* reproducible;
* clearly explained;
* aligned with upstream behavior;
* covered by tests where practical;
* safe from a security and privacy perspective.

For upstream `openai/codex`, follow the official upstream contribution policy. This fork should not be used to bypass upstream review, contribution rules, or responsible disclosure expectations.

---

## Reporting bugs

When reporting a bug in this fork, include:

* operating system and version;
* Codex CLI version or commit SHA;
* shell and terminal environment;
* exact command used;
* expected behavior;
* actual behavior;
* logs or screenshots if useful;
* whether the issue reproduces on upstream `openai/codex`;
* whether the issue touches security-sensitive behavior.

For security-sensitive reports, do not include exploit details in a public issue.

---

## Documentation

Useful documents in this repository:

* `docs/contributing.md`
* `docs/install.md`
* `docs/open-source-fund.md`

Official OpenAI Codex resources should be treated as authoritative for product usage, authentication, supported workflows, and upstream contribution expectations.

---

## License

This repository is licensed under the Apache-2.0 License, following the upstream project license.

---

## Disclaimer

This is an independent public fork maintained by `NeoGrAIph`.

OpenAI, Codex, and related marks belong to their respective owners. This repository is not an official OpenAI release channel and does not imply endorsement, sponsorship, or approval by OpenAI.
