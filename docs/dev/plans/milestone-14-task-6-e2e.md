# Milestone 14 — Task 6 low-level plan: e2e smoke test

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


**Scope:** `.github/workflows/ci.yml` only. Extends the existing `e2e
smoke test (seed + status + week)` step's shell script. No new file,
no Rust test file, no changes to any other file. This plan proposes
shell text only — it does not edit the workflow.

**Source of truth for wording:** Task 6 of
`docs/dev/plans/milestone-14-delete-punches-notes.md` (already
rewritten to target this script) and §5/§8 of
`docs/dev/specs/2026-09-13-delete-punches-notes.md` for the exact
output strings.

---

## 1. Exact current state (quoted)

The step, as it exists today (`.github/workflows/ci.yml` lines 57–114):

```yaml
      - name: e2e smoke test (seed + status + week)
        shell: bash
        run: |
          set -euo pipefail
          db="${RUNNER_TEMP}/mlm-ci-smoke.db"
          bin_ext=""
          if [ "${{ runner.os }}" = "Windows" ]; then
            bin_ext=".exe"
          fi
          mlm_bin="target/${{ matrix.target }}/debug/mlm${bin_ext}"

          cargo run --target ${{ matrix.target }} --example seed_test_data -- \
            --db "$db" --seed 20260913

          echo "--- status ---"
          MLM_DB_PATH="$db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status.out"
          grep -q "Day total:" "${RUNNER_TEMP}/status.out"

          echo "--- week ---"
          MLM_DB_PATH="$db" "$mlm_bin" week | tee "${RUNNER_TEMP}/week.out"
          grep -q "Target:" "${RUNNER_TEMP}/week.out"

          # --- backdated punches (milestone 12): a fresh, separate db,
          # kept independent of the seeded one above for determinism. ---
          backdate_db="${RUNNER_TEMP}/mlm-ci-smoke-backdate.db"

          echo "--- backdate: start/stop/note on a fixed absolute date ---"
          MLM_DB_PATH="$backdate_db" "$mlm_bin" start -d 2020-01-01 09:00
          MLM_DB_PATH="$backdate_db" "$mlm_bin" stop -d 2020-01-01 17:00
          MLM_DB_PATH="$backdate_db" "$mlm_bin" note -d 2020-01-01 "ci backdating smoke test"

          echo "--- backdate: status on that absolute date ---"
          MLM_DB_PATH="$backdate_db" "$mlm_bin" status 2020-01-01 | tee "${RUNNER_TEMP}/status-backdate.out"
          grep -qE "Day total: *08h 00m" "${RUNNER_TEMP}/status-backdate.out"
          grep -q "ci backdating smoke test" "${RUNNER_TEMP}/status-backdate.out"

          # -N shorthand round-trip: let mlm itself resolve "3 days ago"
          # on both the write and read side, so there's no bash-side
          # relative-date arithmetic to get cross-platform-wrong. Each
          # invocation resolves "today" fresh from the wall clock, so a
          # local-midnight rollover between the write and read calls
          # here could in principle make -3 resolve to different days
          # on each side; accepted as negligible residual risk given
          # these run sequentially, well under a second apart.
          echo "--- backdate: -N shorthand round-trip ---"
          MLM_DB_PATH="$backdate_db" "$mlm_bin" start -d -3 09:00
          MLM_DB_PATH="$backdate_db" "$mlm_bin" stop -d -3 17:00
          MLM_DB_PATH="$backdate_db" "$mlm_bin" status -3 | tee "${RUNNER_TEMP}/status-shorthand.out"
          grep -qE "Day total: *08h 00m" "${RUNNER_TEMP}/status-shorthand.out"

          # Future dates must be rejected outright on start/stop/note.
          echo "--- backdate: future date is a hard error ---"
          if MLM_DB_PATH="$backdate_db" "$mlm_bin" start -d 2099-01-01 09:00 2>"${RUNNER_TEMP}/future.err"; then
            echo "expected start -d 2099-01-01 to fail, but it succeeded" >&2
            exit 1
          fi
          grep -q "date is in the future" "${RUNNER_TEMP}/future.err"
```

Style patterns this plan matches:
- Variable naming: `<feature>_db` for a scoped db (`backdate_db`
  precedent), `${RUNNER_TEMP}/mlm-ci-smoke-<feature>.db` naming.
- Section banners: `echo "--- <label> ---"`.
- Every read command's output is `tee`d to a `${RUNNER_TEMP}/<label>.out`
  (or `.err` for stderr) file, then asserted against with
  `grep -q`/`grep -qE`.
- Failure-expected commands use:
  ```sh
  if MLM_DB_PATH="$x" "$mlm_bin" <cmd that must fail> 2>"${RUNNER_TEMP}/<label>.err"; then
    echo "expected <cmd> to fail, but it succeeded" >&2
    exit 1
  fi
  grep -q "<expected message fragment>" "${RUNNER_TEMP}/<label>.err"
  ```
- Every invocation of `$mlm_bin` is prefixed with `MLM_DB_PATH="$<db>"`
  inline (no `export`).
- Comments above a section explain *why* a design choice was made
  (isolation, determinism), not just what the code does — matches this
  plan's own comments below.

## 2. Scratch DB isolation — decision

Follow the existing pattern exactly: one fresh, separate db per
logically-independent scenario, never reusing the seeded `$db` (whose
content/schema-adjacent shape is randomized and not meant to be
asserted against precisely) or `$backdate_db` (a different milestone's
scenario; mixing concerns there would make failures ambiguous about
which milestone regressed).

Four new scratch dbs, one per scenario, mirroring `backdate_db`'s
"fresh, separate db, kept independent ... for determinism" rationale:

- `delete_db="${RUNNER_TEMP}/mlm-ci-smoke-delete.db"` — main round-trip
  scenario (list → delete → confirm gone → recreate → confirm
  restored), for both punches (both the `start` at position 1 and the
  `stop`/End punch at position 2 — the latter added specifically to
  exercise `PunchKind::as_str()`'s "stop", not "end" fix end-to-end,
  not just at Task 3's unit-test level) and a note.
- Reused for the "empty list" check (a date on `delete_db` that was
  never written to — `-d -1`, yesterday relative to the punches/note
  seeded on today's date — needs no separate db since nothing has ever
  touched that date on `delete_db`).
- `delete_oob_db="${RUNNER_TEMP}/mlm-ci-smoke-delete-oob.db"` — a
  single-punch db, isolated so the out-of-range-id assertion ("exactly
  one entry exists, id 2 is out of range") can't be perturbed by
  anything `delete_db`'s round-trip section does to its own count.
- `delete_flag_db="${RUNNER_TEMP}/mlm-ci-smoke-delete-flag.db"` — the
  leading-flag-lookalike note, isolated so a stray earlier note on the
  same date can't shift its ephemeral listing position and make the
  test fragile/order-dependent.
- Future-date rejection reuses `delete_db` (no write happens on a
  rejected command, so there's nothing to isolate from — same reasoning
  the existing future-date check already applies by reusing
  `backdate_db`).

This costs 3 new db files total (`delete_db`, `delete_oob_db`,
`delete_flag_db`) plus reuse of `delete_db` for the empty-list case —
proportionate to the existing script's one new db per milestone
section (`backdate_db`), not a proliferation.

## 3. Exact new shell block

**Placement:** insert immediately after the current final line of the
`run: |` block —

```
          grep -q "date is in the future" "${RUNNER_TEMP}/future.err"
```

(line 113 of the current file) — i.e. appended as the new tail of the
same `run: |` block, no changes to any earlier line. Nothing currently
follows that line in the block, so this is a pure append.

**New content to insert** (10-space base indent, matching the existing
block's indent level exactly):

```sh
          # --- delete note/punch (milestone 14): a fresh, separate db,
          # kept independent of the seeded/backdated ones above so the
          # ephemeral listing positions this section asserts on can't be
          # perturbed by unrelated data. ---
          delete_db="${RUNNER_TEMP}/mlm-ci-smoke-delete.db"

          echo "--- delete: seed a punch and a note for today ---"
          MLM_DB_PATH="$delete_db" "$mlm_bin" start 09:00
          MLM_DB_PATH="$delete_db" "$mlm_bin" stop 17:00
          MLM_DB_PATH="$delete_db" "$mlm_bin" note "ci delete smoke test"

          echo "--- delete: bare 'delete punch' lists both punches numbered ---"
          MLM_DB_PATH="$delete_db" "$mlm_bin" delete punch | tee "${RUNNER_TEMP}/delete-punch-list.out"
          grep -qE "^1 +start 09:00" "${RUNNER_TEMP}/delete-punch-list.out"
          grep -qE "^2 +stop 17:00" "${RUNNER_TEMP}/delete-punch-list.out"

          echo "--- delete: bare 'delete note' lists the note numbered ---"
          MLM_DB_PATH="$delete_db" "$mlm_bin" delete note | tee "${RUNNER_TEMP}/delete-note-list.out"
          grep -qE "^1 +ci delete smoke test" "${RUNNER_TEMP}/delete-note-list.out"

          echo "--- delete: delete punch 1 (the start), confirm gone, capture recreate line ---"
          MLM_DB_PATH="$delete_db" "$mlm_bin" delete punch 1 | tee "${RUNNER_TEMP}/delete-punch-do.out"
          grep -q "deleted. to recreate: mlm start 09:00" "${RUNNER_TEMP}/delete-punch-do.out"
          MLM_DB_PATH="$delete_db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status-after-punch-delete.out"
          if grep -q "09:00" "${RUNNER_TEMP}/status-after-punch-delete.out"; then
            echo "expected 09:00 start punch to be gone from status after delete, but it's still there" >&2
            exit 1
          fi

          echo "--- delete: run the punch recreate line for real, confirm restored ---"
          recreate_punch="$(grep -o 'mlm start .*' "${RUNNER_TEMP}/delete-punch-do.out")"
          MLM_DB_PATH="$delete_db" eval "\"\$mlm_bin\" ${recreate_punch#mlm }"
          MLM_DB_PATH="$delete_db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status-after-punch-restore.out"
          grep -q "09:00" "${RUNNER_TEMP}/status-after-punch-restore.out"

          echo "--- delete: delete punch 2 (the stop/End punch), confirm the recreate line says 'stop' not 'end' ---"
          # Cross-consistency review finding (minor): Task 3 unit-tests
          # PunchKind::as_str()'s "stop", not "end" bug directly, but no
          # e2e path had previously exercised deleting/recreating the
          # End-kind punch through the real dispatch/binary. This closes
          # that gap -- if `recreate_punch_line` regresses to using
          # `PunchKind::as_str()` verbatim (which yields "end", not the
          # real `mlm end` command that doesn't exist), this grep catches
          # it end-to-end, not just at the unit level.
          MLM_DB_PATH="$delete_db" "$mlm_bin" delete punch 2 | tee "${RUNNER_TEMP}/delete-punch2-do.out"
          grep -q "deleted. to recreate: mlm stop 17:00" "${RUNNER_TEMP}/delete-punch2-do.out"
          if grep -q "mlm end " "${RUNNER_TEMP}/delete-punch2-do.out"; then
            echo "recreate line used 'end' instead of 'stop' -- mlm has no 'end' subcommand" >&2
            exit 1
          fi
          recreate_punch2="$(grep -o 'mlm stop .*' "${RUNNER_TEMP}/delete-punch2-do.out")"
          MLM_DB_PATH="$delete_db" eval "\"\$mlm_bin\" ${recreate_punch2#mlm }"
          MLM_DB_PATH="$delete_db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status-after-punch2-restore.out"
          grep -q "17:00" "${RUNNER_TEMP}/status-after-punch2-restore.out"

          echo "--- delete: delete note 1, confirm gone, capture recreate line ---"
          MLM_DB_PATH="$delete_db" "$mlm_bin" delete note 1 | tee "${RUNNER_TEMP}/delete-note-do.out"
          grep -q "deleted. to recreate: mlm note 'ci delete smoke test'" "${RUNNER_TEMP}/delete-note-do.out"
          MLM_DB_PATH="$delete_db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status-after-note-delete.out"
          if grep -q "ci delete smoke test" "${RUNNER_TEMP}/status-after-note-delete.out"; then
            echo "expected note to be gone from status after delete, but it's still there" >&2
            exit 1
          fi

          echo "--- delete: run the note recreate line for real, confirm restored ---"
          recreate_note="$(grep -o "mlm note .*" "${RUNNER_TEMP}/delete-note-do.out")"
          MLM_DB_PATH="$delete_db" bash -c "\"\$mlm_bin\" ${recreate_note#mlm }" _ "$mlm_bin"
          MLM_DB_PATH="$delete_db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status-after-note-restore.out"
          grep -q "ci delete smoke test" "${RUNNER_TEMP}/status-after-note-restore.out"

          echo "--- delete: empty date list mode prints 'nothing to delete' and exits 0 ---"
          MLM_DB_PATH="$delete_db" "$mlm_bin" delete note --date -1 | tee "${RUNNER_TEMP}/delete-empty.out"
          grep -q "nothing to delete for" "${RUNNER_TEMP}/delete-empty.out"

          # --- delete: out-of-range id (milestone 14), isolated single-entry db. ---
          delete_oob_db="${RUNNER_TEMP}/mlm-ci-smoke-delete-oob.db"

          echo "--- delete: seed a single punch for the out-of-range check ---"
          MLM_DB_PATH="$delete_oob_db" "$mlm_bin" start 09:00

          echo "--- delete: id 2 against a 1-entry date is rejected, nothing deleted ---"
          if MLM_DB_PATH="$delete_oob_db" "$mlm_bin" delete punch 2 2>"${RUNNER_TEMP}/delete-oob.err"; then
            echo "expected delete punch 2 (out of range) to fail, but it succeeded" >&2
            exit 1
          fi
          MLM_DB_PATH="$delete_oob_db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status-after-oob.out"
          grep -q "09:00" "${RUNNER_TEMP}/status-after-oob.out"

          # --- delete: leading flag-lookalike note body (milestone 14),
          # isolated so no other note on the same date can shift its
          # ephemeral listing position. ---
          delete_flag_db="${RUNNER_TEMP}/mlm-ci-smoke-delete-flag.db"

          echo "--- delete: seed a note whose body starts with a flag-lookalike token ---"
          MLM_DB_PATH="$delete_flag_db" "$mlm_bin" note "--verbose logging bug"

          echo "--- delete: delete it, capture the quoted recreate line ---"
          MLM_DB_PATH="$delete_flag_db" "$mlm_bin" delete note 1 | tee "${RUNNER_TEMP}/delete-flag-do.out"
          grep -q "deleted. to recreate: mlm note '--verbose logging bug'" "${RUNNER_TEMP}/delete-flag-do.out"

          echo "--- delete: run the flag-lookalike recreate line through a real shell, confirm exact body restored ---"
          recreate_flag="$(grep -o "mlm note .*" "${RUNNER_TEMP}/delete-flag-do.out")"
          MLM_DB_PATH="$delete_flag_db" bash -c "\"\$mlm_bin\" ${recreate_flag#mlm }" _ "$mlm_bin"
          MLM_DB_PATH="$delete_flag_db" "$mlm_bin" status | tee "${RUNNER_TEMP}/status-after-flag-restore.out"
          grep -q -- "--verbose logging bug" "${RUNNER_TEMP}/status-after-flag-restore.out"

          # Future dates must be rejected on delete too, mirroring the
          # start/stop/note future-date check above.
          echo "--- delete: future --date is a hard error ---"
          if MLM_DB_PATH="$delete_db" "$mlm_bin" delete note --date 2099-01-01 2>"${RUNNER_TEMP}/delete-future.err"; then
            echo "expected delete note --date 2099-01-01 to fail, but it succeeded" >&2
            exit 1
          fi
          grep -q "date is in the future" "${RUNNER_TEMP}/delete-future.err"
```

### Notes on the block above

- **Re-running the recreate line "for real, through the actual
  shell"**: the recreate line is `mlm <verb> ... '<quoted body>'` (the
  literal binary name `mlm`, not `$mlm_bin`'s actual path/extension).
  Piping that string into `eval`/`bash -c` verbatim would try to
  execute a program literally named `mlm` on `PATH`, which isn't
  guaranteed on a CI runner. The block strips the literal leading
  `mlm ` prefix (`${recreate_punch#mlm }` / `${recreate_note#mlm }` /
  `${recreate_flag#mlm }`) and substitutes the real `$mlm_bin` path,
  while still routing the **rest of the line — including the
  single-quoted body — through a real shell** (`bash -c "..."`, or
  `eval` for the punch line, which has no metacharacter-bearing
  payload to worry about) so the quoting itself is genuinely exercised
  by shell parsing, not just string-inspected. This satisfies the
  acceptance criterion's "run for real, through the actual shell"
  literally, while staying runnable on a matrix runner where the
  binary isn't installed as `mlm` on `PATH`.
  - `bash -c "<cmd>" _ "$mlm_bin"` passes `$mlm_bin`'s value in as
    `bash -c`'s implicit `$0`... **this needs one more step**: the
    inner command string must actually reference that positional for
    the substitution to work portably without variable-expansion
    order surprises. Simplify instead: build the inner command with
    `$mlm_bin`'s value already substituted by the outer (non-quoted)
    shell before it reaches `bash -c`, i.e.:
    ```sh
    recreate_note="$(grep -o "mlm note .*" "${RUNNER_TEMP}/delete-note-do.out")"
    inner="${mlm_bin} ${recreate_note#mlm }"
    MLM_DB_PATH="$delete_db" bash -c "$inner"
    ```
    This is the form to actually use in place of the `bash -c
    "\"\$mlm_bin\" ..." _ "$mlm_bin"` lines drafted above — outer bash
    substitutes `$mlm_bin` and `$recreate_note` into a plain string
    first (`$inner`), then that whole string — single-quoted body and
    all — is handed to a **fresh** `bash -c` to be tokenized for real.
    Apply this same `inner="..."` pattern to the punch line too
    (replacing the `eval` line) and to the flag-lookalike line, for
    consistency (three call sites, one pattern):
    ```sh
    recreate_punch="$(grep -o 'mlm start .*' "${RUNNER_TEMP}/delete-punch-do.out")"
    inner="${mlm_bin} ${recreate_punch#mlm }"
    MLM_DB_PATH="$delete_db" bash -c "$inner"
    ```
    ```sh
    recreate_flag="$(grep -o "mlm note .*" "${RUNNER_TEMP}/delete-flag-do.out")"
    inner="${mlm_bin} ${recreate_flag#mlm }"
    MLM_DB_PATH="$delete_flag_db" bash -c "$inner"
    ```
    **Use this corrected three-line `inner="..."` + `bash -c "$inner"`
    form at all three recreate call sites** (punch, note, flag-body
    note) instead of the `eval`/`bash -c ... _ "$mlm_bin"` variants
    shown in the main block above — those were left in the main block
    only to show the placement/ordering; the implementer should use
    the corrected form when actually adding these lines.
  - Windows note: `bin_ext` may be `.exe`; `$mlm_bin` already carries
    that suffix (set once near the top of the script), so `$inner`
    naturally becomes e.g. `target/.../mlm.exe start 09:00`, which
    `bash -c` (the step's shell is pinned to `shell: bash`, per the
    existing `shell: bash` step key — this holds on the Windows runner
    too, since GitHub Actions' `shell: bash` uses the Git-for-Windows
    bash there) runs the same way as on Linux/macOS.
- **Punch-listing grep pattern**: §4 of the spec gives the format as
  `<n>  <kind> <HH:MM>` (two spaces). `grep -qE "^1 +start 09:00"` uses
  `+` (one-or-more spaces) rather than hardcoding two literal spaces,
  matching the existing script's own tolerance style
  (`grep -qE "Day total: *08h 00m"` already uses `*`/flexible
  whitespace rather than an exact byte match) — deliberately not
  over-fitting to the exact column width, which is presentation detail
  the spec doesn't freeze.
- **Recreate-line grep for the note case**: matches
  `docs/dev/specs/2026-09-13-delete-punches-notes.md` §5's literal
  example format (`deleted. to recreate: mlm note 'fixed migration
  runner bug'`) — single-quoted body, no `--date` (today's date is
  omitted per §5: "`--date …` is only included when the deleted
  entry's date isn't today"). Since every write in this section omits
  `-d`, every recreate line in the main round-trip section is expected
  to omit `--date` too — consistent with that rule, not asserted
  against a stray literal date.
- **Why `grep -q -- "--verbose logging bug"`**: the leading `--`
  prevents `grep` itself from interpreting the search pattern as an
  option string — the same category of flag-lookalike hazard this
  whole scenario exists to catch, just one level down the tool chain
  here in the test script's own `grep` invocation. Match the same
  care that motivated the original delete-echo quoting fix.
- **Why the empty-list check reuses `delete_db` with `--date -1`
  instead of a fresh db**: `delete_db` never receives any write for
  "yesterday relative to today" anywhere else in this section (every
  other write in the section omits `-d`, defaulting to today), so `-1`
  is guaranteed empty on that db without needing a new scratch file —
  narrower diff, same isolation guarantee the other sections rely on
  scratch dbs for.

## 4. Local verification before pushing

This repo has no documented way to run *this exact CI step* locally
(no `act`/self-hosted-runner tooling referenced in `AGENTS.md` or
`README.md` — both only document `cargo build`/`cargo test`/`cargo
clippy` plus manual `cargo run` smoke commands under "Verifying
changes" in `AGENTS.md`). Approximate the step by hand, in the same
order the CI script runs it:

```sh
cargo build
mlm_bin="target/debug/mlm"

delete_db="$(mktemp -u /tmp/mlm-ci-smoke-delete.XXXXXX.db)"
MLM_DB_PATH="$delete_db" "$mlm_bin" start 09:00
MLM_DB_PATH="$delete_db" "$mlm_bin" stop 17:00
MLM_DB_PATH="$delete_db" "$mlm_bin" note "ci delete smoke test"
MLM_DB_PATH="$delete_db" "$mlm_bin" delete punch
MLM_DB_PATH="$delete_db" "$mlm_bin" delete note
MLM_DB_PATH="$delete_db" "$mlm_bin" delete punch 1
MLM_DB_PATH="$delete_db" "$mlm_bin" status
# ...paste the printed "to recreate: mlm ..." line back in, with `mlm`
# replaced by "$mlm_bin", and re-run `status` to confirm it restored.
rm -f "$delete_db"
```

Repeat the same manual pattern for the out-of-range-id db, the
leading-flag-lookalike-note db, and the future-`--date` rejection
case, each as its own scratch `MLM_DB_PATH`, mirroring the sections
above. Once satisfied locally, the real signal is CI itself — push to
a branch/PR and let the matrix run the actual step on all five
targets (this section exists specifically to catch
platform-specific/cross-shell issues a Linux dev box won't surface,
per the step's own leading comment), then inspect the Action run's
"e2e smoke test (seed + status + week)" step log.

## 5. Verification checklist for the implementer

- [ ] New block appended after the existing future-date check, same
      `run: |` block, no reordering of existing lines.
- [ ] Every new `$mlm_bin` invocation is prefixed `MLM_DB_PATH="$<db>"`
      inline, no `export`.
- [ ] Every read/list command's stdout is `tee`d to a uniquely-named
      `${RUNNER_TEMP}/*.out` file before being grepped.
- [ ] Every expected-failure command uses the
      `if ...; then echo ...; exit 1; fi` + follow-up `grep` pattern,
      not `!`/`set +e` toggling.
- [ ] The three recreate-line replay sites use the corrected
      `inner="${mlm_bin} ${line#mlm }"` + `bash -c "$inner"` form from
      §3's notes, not the placeholder `eval`/`bash -c ... _` forms
      shown inline in the main block.
- [ ] `cargo fmt --check` / `cargo clippy` are unaffected (YAML-only
      change) — still run the full local verification in AGENTS.md's
      "Verifying changes" section plus both clippy gates (Task 3's
      cognitive-complexity gate) before opening the PR, since Task 6's
      own acceptance criteria require the full suite and both clippy
      gates green as the milestone's overall completion signal.
