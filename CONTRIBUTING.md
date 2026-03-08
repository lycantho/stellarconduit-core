# Contributing to StellarConduit Core

Thank you for your interest in contributing to the mesh networking engine of the StellarConduit protocol! 

This repository contains the most complex and critical code in the organization. We rely on community contributions to make it robust, performant, and secure.

## Getting Started

1. **Find an Issue**: Browse the [Issues](https://github.com/StellarConduit/stellarconduit-core/issues) tab. Look for the `good first issue` label if you're new, or `help wanted` for larger tasks.
2. **Claim the Issue**: Comment on the issue asking to be assigned. Wait for a maintainer to assign it to you before starting work to avoid duplicated effort.
3. **Fork & Branch**: Fork the repo and create a branch for your feature/fix. Name it `feat/your-feature`, `fix/issue-description`, or `chore/task`.

## Development Workflow

Before opening a Pull Request, you **must** ensure the following commands pass locally:

```bash
# 1. Format code
cargo fmt --all

# 2. Check for warnings/clippy rules
cargo clippy --all-targets --all-features -- -D warnings

# 3. Run all tests
cargo test --workspace
```

### Writing Tests
- All new features must include unit tests. Aim for >85% coverage.
- If you are modifying the gossip protocol, router, or topology, you must update or add an integration/simulation test in `tests/integration/`.

### Writing Commit Messages
We follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat(discovery): add BLE support`
- `fix(gossip): resolve bloom filter false positives`
- `docs: update transport protocol sequence diagram`
- `test(router): add path finding benchmark`

## Pull Request Process

1. Provide a clear, detailed PR description.
2. Link the PR to the issue it resolves (e.g., "Closes #42").
3. Ensure CI passes.
4. Two maintainer approvals are required before merging.

## Protocol Architecture & Design 
Please read the module-level READMEs in the `docs/` folder before making architectural changes. If you are proposing a significant change to the mesh protocol, please open an Issue for discussion first.
