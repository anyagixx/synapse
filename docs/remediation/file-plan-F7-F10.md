# F7-F10 — MCP, Packaging, Testing, Hygiene

# F7 — MCP/dashboard/security truth repair

## `/home/truffle/Загрузки/synapse/src/mcp/mod.rs`
- **Фаза:** F7
- **Зачем менять:** module docs and exports should reflect actual MCP modes and LSP support.
- **Что менять:** align exports/comments/contracts with final MCP policy.
- **Результат:** module truth aligned.
- **Зависимости:** `mcp/server.rs`, `mcp/lsp.rs`.

## `/home/truffle/Загрузки/synapse/src/dashboard.rs`
- **Фаза:** F7
- **Зачем менять:** local-only security posture needs explicit guardrails.
- **Что менять:** add bind checks, optional auth/token path if chosen, clearer warnings.
- **Результат:** safer HTTP surface.
- **Зависимости:** product security decision.

## `/home/truffle/Загрузки/synapse/src/grace/review.rs`
- **Фаза:** F7
- **Зачем менять:** secret scan feature must be downgraded or improved.
- **Что менять:**
  - rename report labels to heuristic/basic if keeping current approach;
  - or integrate stronger pattern logic.
- **Результат:** security review not oversold.
- **Зависимости:** CI security tool choice.

## `/home/truffle/Загрузки/synapse/docs/FAQ.md`
- **Фаза:** F7
- **Зачем менять:** security/runtime questions must reflect real protections.
- **Что менять:** document localhost-only expectations, heuristic secret scanning, HTTP mode support state.
- **Результат:** users warned correctly.
- **Зависимости:** runtime policy.

## `/home/truffle/Загрузки/synapse/README.md`
- **Фаза:** F7 follow-up
- **Зачем менять:** update security posture messaging after MCP/dashboard decisions.
- **Что менять:** revise marketed claims.
- **Результат:** no false safety implication.
- **Зависимости:** F7 decisions.

---

# F8 — Packaging/release/install

## `/home/truffle/Загрузки/synapse/Cargo.toml`
- **Фаза:** F8
- **Зачем менять:** package metadata/repo/homepage must match project truth and release story.
- **Что менять:**
  - verify `repository`, `homepage`, description;
  - possibly add features or metadata for release tooling;
  - keep dependency list honest if release matrix changes.
- **Результат:** cargo metadata aligned with public docs.
- **Зависимости:** canonical repo/homepage decision.

## `/home/truffle/Загрузки/synapse/install.sh`
- **Фаза:** F8
- **Зачем менять:** installer uses fragile temp paths and may mismatch release artifacts/URLs.
- **Что менять:**
  - use `mktemp`;
  - validate URLs and artifact names;
  - add checksum/signature verification if available;
  - improve cleanup and error handling;
  - align supported platforms with release outputs.
- **Результат:** safer, accurate installer.
- **Зависимости:** release workflow updates.

## `/home/truffle/Загрузки/synapse/Dockerfile`
- **Фаза:** F8 optional
- **Зачем менять:** if docs mention container usage, Docker build should align with release/runtime changes.
- **Что менять:**
  - verify build path after CLI/module changes;
  - optionally add smoke command.
- **Результат:** container path remains valid.
- **Зависимости:** build/release changes.

## `/home/truffle/Загрузки/synapse/.github/workflows/release.yml`
- **Фаза:** F8
- **Зачем менять:** current release builds Linux-only, while docs imply broader support.
- **Что менять:**
  - add matrix for supported platforms;
  - standardize artifact names;
  - generate checksums;
  - optionally sign artifacts;
  - upload all outputs consistently.
- **Результат:** release artifacts == documented support.
- **Зависимости:** platform support decision.

## `/home/truffle/Загрузки/synapse/.github/workflows/ci.yml`
- **Фаза:** F8, F9
- **Зачем менять:** CI lacks release-smoke parity and docs drift gates.
- **Что менять:**
  - in F8 add release build smoke if needed;
  - in F9 add docs parity, self-host verify, installer tests, security scan.
- **Результат:** CI catches drift earlier.
- **Зависимости:** test scripts ready.

## `/home/truffle/Загрузки/synapse/INSTALL.md`
- **Фаза:** F8
- **Зачем менять:** final install docs must match new release pipeline.
- **Что менять:** update URLs, platform matrix, checksum verification instructions.
- **Результат:** doc-install parity.
- **Зависимости:** release/install changes.

---

# F9 — Testing + parity gates

## `/home/truffle/Загрузки/synapse/tests/integration_test.rs`
- **Фаза:** F9
- **Зачем менять:** current integration coverage mostly happy-path; misses docs parity, corrupted index, command drift, install/runtime edge cases.
- **Что менять:**
  - expand with negative cases;
  - add verify/review/status consistency checks;
  - add hooks status regression test;
  - add config load/init behavior tests;
  - add command visibility tests.
- **Результат:** broader regression net.
- **Зависимости:** core fixes landed.

## `/home/truffle/Загрузки/synapse/tests/fixtures/`
- **Фаза:** F9 new dir
- **Зачем менять:** need stable sample repos and broken cases.
- **Что менять:** create fixture projects:
  - minimal valid rust repo
  - mixed-language repo
  - broken-contract repo
  - corrupted-index repo
  - large-ish repo sample
- **Результат:** reproducible integration coverage.
- **Зависимости:** test strategy.

## `/home/truffle/Загрузки/synapse/tests/cli_help_snapshot.rs`
- **Фаза:** F9 new file
- **Зачем менять:** protect CLI help and command surface from drift.
- **Что менять:** snapshot top-level and subcommand help outputs.
- **Результат:** docs and help parity easier to maintain.
- **Зависимости:** CLI cleanup stable.

## `/home/truffle/Загрузки/synapse/tests/docs_parity.rs`
- **Фаза:** F9 new file
- **Зачем менять:** automatic check for README/docs claims vs code.
- **Что менять:** parse command list/tool count/platform claims/metrics or compare against generated manifest.
- **Результат:** drift caught in CI.
- **Зависимости:** capability source ready.

## `/home/truffle/Загрузки/synapse/tests/storage_corruption.rs`
- **Фаза:** F9 new file
- **Зачем менять:** ensure malformed index no longer silently disappears.
- **Что менять:** simulate truncated/invalid index and assert explicit behavior.
- **Результат:** storage hardening tested.
- **Зависимости:** storage fixes complete.

## `/home/truffle/Загрузки/synapse/tests/config_behavior.rs`
- **Фаза:** F9 new file
- **Зачем менять:** lock in pure load vs init semantics.
- **Что менять:** test missing config, invalid config, readonly paths, init path.
- **Результат:** config contract protected.
- **Зависимости:** config redesign complete.

## `/home/truffle/Загрузки/synapse/tests/tracking_identity.rs`
- **Фаза:** F9 new file
- **Зачем менять:** prevent basename collision regressions.
- **Что менять:** test two same-named directories under different parents.
- **Результат:** tracking correctness protected.
- **Зависимости:** tracking schema update.

## `/home/truffle/Загрузки/synapse/tests/mcp_runtime.rs`
- **Фаза:** F9 new file
- **Зачем менять:** validate stdio mode, HTTP mode policy, tool exposure.
- **Что менять:** add smoke/runtime behavior checks.
- **Результат:** MCP truth enforced.
- **Зависимости:** MCP policy stabilized.

## `/home/truffle/Загрузки/synapse/tests/dashboard_bind.rs`
- **Фаза:** F9 new file
- **Зачем менять:** validate loopback/non-loopback warning behavior.
- **Что менять:** add server startup policy tests where feasible.
- **Результат:** dashboard security posture enforced.
- **Зависимости:** dashboard hardening complete.

## `/home/truffle/Загрузки/synapse/tests/release_install_smoke.rs`
- **Фаза:** F9 optional/new file or script-based
- **Зачем менять:** protect install/release flow.
- **Что менять:** smoke-test artifact naming/install logic, possibly via shell-driven CI.
- **Результат:** installer drift caught before release.
- **Зависимости:** release workflow stable.

## `/home/truffle/Загрузки/synapse/.github/workflows/ci.yml`
- **Фаза:** F9
- **Зачем менять:** wire all parity tests and self-host checks.
- **Что менять:**
  - add `cargo build --release`;
  - add self-host `syn verify` check;
  - add docs parity test job;
  - add installer smoke job;
  - add external secret scan if chosen;
  - optional `cargo deny`/coverage.
- **Результат:** CI becomes anti-drift gatekeeper.
- **Зависимости:** tests and scripts exist.

## `/home/truffle/Загрузки/synapse/scripts/`
- **Фаза:** F9
- **Зачем менять:** may need helper scripts for docs parity, release smoke, checksum verify.
- **Что менять:**
  - add deterministic helper scripts only if needed;
  - keep them minimal and CI-oriented.
- **Результат:** repeatable automation.
- **Зависимости:** exact CI jobs chosen.

---

# F10 — Repo hygiene + final seal

## `/home/truffle/Загрузки/synapse/.gitignore`
- **Фаза:** F10
- **Зачем менять:** repo contains noisy/generated assets that may pollute search/test/audit.
- **Что менять:**
  - review ignored generated outputs;
  - ensure local artifacts, temp indexes, release outputs, fixtures behave correctly.
- **Результат:** cleaner repo behavior.
- **Зависимости:** release/test artifact policy.

## `/home/truffle/Загрузки/synapse/.opencode/`
- **Фаза:** F10
- **Зачем менять:** vendored dependencies/config may create search noise and drift.
- **Что менять:**
  - decide what should be committed;
  - reduce node_modules/vendor noise if possible;
  - align plugin/rules with init-generated outputs.
- **Результат:** lower audit/search noise.
- **Зависимости:** hook/install policy.

## `/home/truffle/Загрузки/synapse/opencode-plugin/plugin.ts`
- **Фаза:** F10
- **Зачем менять:** plugin file currently outside contract system and may drift from hooks/init docs.
- **Что менять:**
  - align plugin behavior with generated `.opencode/plugins/synapse.ts`;
  - decide source-of-truth between plugin template and committed plugin.
- **Результат:** plugin story consistent.
- **Зависимости:** hook/init packaging decisions.

## `/home/truffle/Загрузки/synapse/hooks/opencode/synapse-rewrite.sh`
- **Фаза:** F10
- **Зачем менять:** hook script should align with plugin/proxy/runtime expectations.
- **Что менять:**
  - audit path assumptions;
  - update behavior or docs;
  - decide whether to bring under same governance/testing.
- **Результат:** hook runtime consistent.
- **Зависимости:** proxy/hook policy.

## `/home/truffle/Загрузки/synapse/hooks/adapters/cursor.sh`
- **Фаза:** F10 optional
- **Зачем менять:** shell adapters currently outside contract/testing coverage.
- **Что менять:**
  - audit install/usefulness;
  - decide keep/remove/experimental.
- **Результат:** reduced dead surface.
- **Зависимости:** product scope decision.

## `/home/truffle/Загрузки/synapse/hooks/adapters/claude-code.sh`
- **Фаза:** F10 optional
- **Зачем менять:** same as above.
- **Что менять:** audit and classify.
- **Результат:** same.
- **Зависимости:** scope decision.

## `/home/truffle/Загрузки/synapse/build.rs`
- **Фаза:** F10
- **Зачем менять:** build script should track correct generated/config sources after refactors and packaging updates.
- **Что менять:**
  - audit rerun-if-changed list;
  - include new capability/generated docs files if required;
  - ensure no stale dependencies.
- **Результат:** reproducible builds.
- **Зависимости:** final file layout stabilized.

## `/home/truffle/Загрузки/synapse/Cargo.toml`
- **Фаза:** F10 follow-up
- **Зачем менять:** after refactors/tests/scripts, update package metadata, features, dev-dependencies.
- **Что менять:**
  - add/remove deps from remediation work;
  - keep manifest lean.
- **Результат:** dependency graph honest and minimal.
- **Зависимости:** implementation complete.

## `/home/truffle/Загрузки/synapse/Makefile`
- **Фаза:** F10
- **Зачем менять:** expose new validation flows and release/test helpers.
- **Что менять:**
  - add targets for parity, verify-self, release-smoke, install-smoke if useful.
- **Результат:** repeatable maintainer workflows.
- **Зависимости:** CI/test scripts finalized.

---

