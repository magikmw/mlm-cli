# Changelog

All notable changes to this project are documented in this file.
## [0.1.2] - 2026-09-13

### 💼 Other Changes

- (chore) trigger another release
## [0.1.1] - 2026-09-13

### 🐛 Bug Fixes

- *(ci)* Install minisign from upstream binary, not apt
- *(ci)* Pin sha256 of downloaded minisign binary

### 💼 Other Changes

- Release v0.1.1
## [0.1.0] - 2026-09-13

### 🚀 Features

- *(date)* DATE and WEEK_ID parsing, formatting and week iteration
- *(db)* Real schema and embedded migrations
- *(storage)* Implement Milestone 4 — punch and note storage

### 🐛 Bug Fixes

- Cargo fmt on the previous commit's cfg_attr line

### 💼 Other Changes

- Scaffold mlm CLI time tracker project
- Add spec section 1 (overview/goals/terminology)
- Complete mlm MVP spec (SPEC.md) through section 8
- Add implementation plan, confirm two accounting inferences
- Rework plan for parallel worktree execution
- Detailed per-milestone implementation plans + cross-plan reconciliation
- Sync detailed plans to reconciled contracts; adopt anyhow + verbose logging; add docs milestone; add pre-commit quality gate
- Milestone 1: real TIME/DURATION parsing and canonical duration formatting
- Milestone 6: week accounting (target, carry, fulfillment)
- Milestone 5: stint pairing (nearest-match LIFO) in src/stint.rs
- Mark M5's same-instant boundary defect visibly in SPEC.md
- Implement milestone 8: week target command
- Fix punch read tiebreak to (at_utc, kind, id)
- Swap Milestone 5/6 fixture stand-ins for real types
- Implement Milestone 7: start, stop, note commands
- Implement Milestone 9: shared rendering helpers
- Implement Milestone 11: week command and rendering
- Implement Milestone 10: status command and rendering
- Cross-cutting verification pass (wave 5)
- Milestone 12: real documentation for README/AGENTS.md, cli.rs doc polish
- Add test-data seeding script; restructure crate as lib+bin
- Add EUPL-1.2 license, prep for crates.io release
- Set up full release pipeline: git-cliff, cargo-dist, minisign signing
- Clean up CHANGELOG.md: filter merge/review noise, hand-write v0.1.0
- (docs) create a dedicated docs folder
- Add Features/Planned sections, drop Stretch goals
- Add CI workflow: build/test/lint + e2e smoke test, all 4 targets
- Add Linux ARM64 as a fifth release/CI target, fix stale README claims
- Add From-source (dist build) and clarify local dev build
- Fix Windows CI failure: ignore the TZ-env-var DST e2e test there
- Add cargo fmt --check as a pre-commit gate; activate the hook for real
- Fix insecure temp file in pre-commit hook's fmt gate
