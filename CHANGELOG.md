# Changelog

All notable changes to this project are documented in this file.
## [0.1.0] - 2026-09-12

### 🚀 Features

- *(date)* DATE and WEEK_ID parsing, formatting and week iteration
- *(db)* Real schema and embedded migrations
- *(storage)* Implement Milestone 4 — punch and note storage

### 📚 Documentation

- Independent adversarial review of Milestone 1
- Adversarial review of Milestone 6 week accounting
- *(review)* Independent adversarial review of Milestone 2
- *(review)* Independent adversarial review of Milestone 3
- *(review)* Independent adversarial review of Milestone 8

### 🔍 Reviews

- Milestone-4 punch/note storage adversarial review
- Independent adversarial review of integration swap (M5/M6)
- Milestone-7 write commands adversarial review
- Independent adversarial review of milestone-9 (shared rendering)
- Independent adversarial review of Milestone 11 (week command)
- Independent adversarial review of Milestone 10 (status command)
- Independent adversarial review of cross-cutting verification pass
- Independent adversarial review of Milestone 12 (documentation)

### 🔀 Merges

- Merge milestone-1: time-of-day and duration parsing/formatting
- Merge milestone-6: week accounting (target, carry, fulfillment)
- Merge milestone-5: stint pairing
- Merge milestone-2: calendar date and week-id parsing/formatting
- Merge milestone-3: schema and migrations
- Merge milestone-8: week target command
- Merge milestone-4: punch and note storage
- Merge integration-swap: fixture stand-ins -> real types
- Merge milestone-7: start, stop, note commands
- Merge milestone-9: shared rendering helpers
- Merge milestone-11: week command and rendering
- Merge milestone-10: status command and rendering
- Merge cross-cutting verification pass (wave 5)
- Merge milestone-12: documentation

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
- Add independent adversarial review of Milestone 5
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
