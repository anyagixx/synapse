# Contributing

## Getting Started

```bash
git clone https://github.com/synapse-ai/synapse
cd synapse
make setup
make build
make test
```

## Development Rules

1. No `unsafe` code — `-D unsafe_code` in CI
2. No dynamic command execution — `Command::new` with variables forbidden
3. Every feature must have tests
4. All docs ≤500 lines
5. Conventional Commits for PR titles

## PR Process

1. Fork the repo
2. Create feature branch
3. `make check` must pass
4. Open PR to `develop` branch
5. CI must pass
6. Requires 1 maintainer review

## Code Style

- Rustfmt with `hard_tabs = true`
- Clippy clean (no warnings)
- EditorConfig respected
