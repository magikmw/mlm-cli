# doc-drift fresh-eyes run

Sandbox: /tmp/cd-sandbox/doc-drift/ (`mlm 0.4.0` + `README.md`). Every run used `MLM_DB_PATH=/tmp/cd-sandbox/doc-drift/<name>.db`, with a fresh db for each scenario. Today was Sat 2026-10-31.

## Scenarios — what you ran, what it printed

### 1. Install / build / local trial
- Read Install, From source, Build (local dev), Data location. The README has no section called "trying it out locally". The nearest match is "Build (local dev)".
- `cargo install mlm`, `cargo install cargo-dist`, `dist build ...` and `cargo build` / `cargo run` could not be run. No cargo or dist is installed, and there is no source tree. The README never says how to get one (no `git clone` line).
- Relative links `docs/SIGNING.md` and `LICENSE` do not exist in what I was given. The external links (rustup.rs, GitHub releases, cargo-binstall) cannot be checked offline.
- I ran the dev-loop commands against the binary: `mlm start 9:00 "working on mlm"` gave rc=0, and `status` showed `09:00-now` plus the note. So `9:00` (H:MM) works, but the README lists TIME as `HH:MM`, `HHMM` or `HH`.

### 2. Notes with a backslash and an apostrophe
```
$ mlm note 'path C:\work\notes.txt'; mlm note "Bob's review"; mlm note "keep this one"
$ mlm delete note 1
deleted note (2026-10-31): path C:\work\notes.txt
$ mlm delete note 1
deleted. to recreate: mlm note 'Bob'"'"'s review'
```
- The apostrophe recreate command works in both bash and fish. Retyping `mlm note 'path C:\work\notes.txt'` by hand also works in both.
- A body with an apostrophe and a backslash (`it's C:\x`) also gets no command.
- `-v flag fix` gives `mlm note '-v flag fix'`, which replays correctly. `$HOME` and backticks are quoted correctly.
- A note typed with an embedded newline is stored and shown as `line1 line2`. Leading spaces (`"  indented"`) are trimmed.
- The recreated note goes to the end of the list (order changes).

### 3. Wrong punch yesterday
```
$ mlm start -d -1 09:00; mlm stop -d -1 12:00; mlm start -d -1 13:00 "afternoon"; mlm stop -d -1 17:30
$ mlm delete punch -d -1        -> 1 start 09:00 / 2 end 12:00 / 3 start 13:00 / 4 end 17:30
$ mlm delete punch 3 -d -1
deleted. to recreate: mlm start 13:00 --date 2026-10-30
$ mlm status -1   -> [!] orphaned end at 17:30 ... fix: mlm start HH:MM --date 2026-10-30  (a time before 17:30)
$ mlm status      -> no mention of yesterday's anomaly at all
$ mlm start 13:00 --date 2026-10-30   -> restored, 07h 30m, clean
```
- Separately, I followed the orphaned-end fix literally with `mlm start -d -1 10:00` (10:00 is "before 17:30"). It printed nothing. `status -1` then showed `[!] overlapping stints 09:00-17:30 and 10:00-12:00` and a 10h 30m day.
- A punch deleted from today gives `deleted. to recreate: mlm start 09:00`, with no `--date`.

### 4. `[!]` for an old unclosed start
```
$ mlm start -d -5 09:00              (no output)
$ mlm start -d -3 09:00; mlm stop -d -3 10:30; mlm start -d -3 08:00
$ mlm start 08:00                     -> warning blocks for 2026-10-26 and 2026-10-28 (window 08:00-09:00)
$ mlm status
Day total:     09h 28m (incl. ongoing), est. EOD 22:30 (tomorrow)
Week 2026-44:  ...
[!] unclosed start on Mon 2026-10-26 at 09:00 (5 days ago) ...
[!] unclosed start on Wed 2026-10-28 at 08:00 (3 days ago) ...

  08:00-now    (09h 28m, ongoing)
$ mlm status -5
Day total:     00h 00m (+ unclosed)
[!] unclosed start at 09:00, 5 days old (no stop recorded)
  09:00-?      (unclosed)
```
- Fresh db: `mlm start -d -20 09:00; mlm start -d -1 18:00; mlm status`:
```
Week 2026-44:  160h 00m left to 40h 00m target (fulfillment -120h 00m = worked 00h 00m + carry-in -120h 00m / target 40h 00m)
[i] start on Fri 2026-10-30 at 18:00 still open ...
[!] carry-in includes anomalies from 1 earlier week, 2026-41 (see mlm week 2026-41)

  18:00-now    (23h 28m so far, ...)
```
  After `mlm delete punch -d -20 1`, carry-in went back to 00h 00m.
- Read `mlm --help`, `mlm delete --help`, `mlm delete punch --help` and `mlm status --help` (the last one has the glossary).

### 5. `week --notes` vs `export notes`
- The README examples agree with each other: the same four notes for 2026-43, same dates, same order. The week example's arithmetic also checks out (rows sum to 48h 48m, and 48h 48m - 03h 32m = 45h 16m).
- With my own data, `mlm week --notes`, `mlm export notes`, `mlm ex n -w -1`, `mlm week -1 -n` and `mlm ex n -m 10` agreed line for line, backslash and apostrophe notes included.
- A future `-d 2027-01-01` gave empty output and rc 0. `-d` together with `-w` gave a clap error, rc 2.
- On a fresh db, a single `mlm note -d -8 "x"` (no punches anywhere) made this week show `Carry-in: -40h 00m` and `80h 00m left to 40h 00m target`.

## Findings

1. **README build/trial sections cannot be followed from the README** (S1). I expected the source-build sections to say where the source comes from. Neither "From source" nor "Build (local dev)" gives a clone/checkout step, and `cargo build` / `cargo run` only make sense inside a checkout. There is no "trying it out locally" section by that name. The relative links `docs/SIGNING.md` and `LICENSE` lead nowhere unless you are reading inside the repo. A new-machine user following the page top to bottom gets stuck at the first `cargo build`.

2. **The dev-loop example uses an undocumented time form** (S1). The README defines TIME as `HH:MM`, `HHMM` or `HH`, but its own example is `start 9:00`. The tool accepts it. Small, but page and tool disagree on what input is allowed.

3. **A backslash note gets no recreate path and no hint** (S2). I expected a ready-to-run command, as the Features list promises ("prints a ready-to-run command to recreate what was deleted"). I got `deleted note (2026-10-31): path C:\work\notes.txt`. The `deleted.` prefix is gone and nothing says "no recreate command, retype with `mlm note --date 2026-10-31 '...'`". The user has to work out the quoting alone. The README's reason ("no quoting replays a backslash the same way in every shell") does not fit the common case: `'C:\work\notes.txt'` replays the same in bash and fish. `delete note --help` says nothing about recreate commands at all. The Features bullet does not mention the exception.

4. **Recreate commands for today omit `--date`** (S2/S3). `deleted. to recreate: mlm start 09:00` and `mlm note 'working on mlm'` are only right if you run them the same day. Yesterday's deletes get an explicit `--date`. Someone who copies the line into a notes file and replays it after midnight writes to the wrong date, without any error.

5. **A recreated note changes the note order, and input is normalised without a word** (S2). After delete and recreate, the note goes to the end of `Notes:`, `week --notes` and `export`. Embedded newlines become spaces and leading whitespace is trimmed. None of this is in the README. The order change matters, because the README stresses "in the order you entered them".

6. **Deleting a punch silently creates an anomaly, and today's status does not show it** (S3). `delete punch 3 -d -1` (a start) left an orphaned end. The delete output did not warn, though `stop` warns in the same situation. Plain `mlm status` today showed nothing about yesterday's orphaned end. Only `mlm week` (`[!]` row) or `mlm status -1` reveal it. A user who deletes the wrong punch and checks `status` thinks all is well.

7. **The orphaned-end fix gives an open-ended time that can make things worse** (S3). The fix says `mlm start HH:MM --date 2026-10-30  (a time before 17:30)` while an `end 12:00` sits earlier on the date. Following it with 10:00 produced overlapping stints and a 10h 30m day, and the `start` printed no warning. The unclosed-start fix gives a window (`a time between 08:00 and 09:00`), so the asymmetry is surprising. The README documents the window only for starts.

8. **The help pages do not explain what the user sees** (S4). `mlm --help` and `mlm delete punch --help` never mention `[!]`, recreate commands, or that deleting one half of a stint leaves an anomaly. The delete description, "by its ephemeral, per-listing position", is jargon. `status --help` has a glossary, but it has no "unclosed start" entry. The README never mentions `(+ unclosed)` (only the glossary does) or the `09:00-?      (unclosed)` stint line, so a reader who checks the README after `mlm status -5` finds neither.

9. **The README misplaces the status blocks** (S4). The README says today's status "appends a block for every date" with an old unclosed start, and that the carry-in line is "the last line of the page". In the output, both appear above the stint list, between the week line and the stints. With notes or a long stint list, a reader looking at the bottom of the page will not find them.

10. **Carry-in starts at the first week holding any punch or note, which is undocumented and contradicts the glossary** (S4/S5). On a fresh db, one note dated 8 days ago gave this week `Carry-in: -40h 00m` and `80h 00m left to 40h 00m target`. One start 20 days ago gave -120h and `160h 00m left to 40h 00m target`. The `status --help` glossary says "Notes change no total, marker or footer", but a note did change the total. Nothing in the README says where carry-in history begins, so a new user who backfills one note or punch suddenly owes weeks of time. `160h left to 40h target` also reads like nonsense at a glance.

11. **The carry-in `[!]` line blames the anomaly for a debt that comes from empty weeks** (S4). With the 20-day-old start, the line said `carry-in includes anomalies from 1 earlier week, 2026-41`, which suggests the -120h comes from the anomaly. Most of it comes from weeks 42 and 43 having no work against their 40h target. Fixing the anomaly by stopping the start would clear the `[!]` but keep the large deficit. Only deleting the punch (removing the week's data) reset it. A user cannot tell from the page what is actually driving the figure.

12. **The delete listing does not name its date** (S3, nitpick). `mlm delete punch -d -1` prints `1  start 09:00 ...` with no header, so with `-N` dates you have to trust you typed the right offset before running `delete punch N`. The stale-id caveat makes that more of a concern.

13. **The export conflict error shows an odd usage line** (S5, nitpick). `mlm ex n -d -4 -w 44` prints `Usage: mlm export notes --date <DATE>`, which reads as if `--date` were required. The README's "Give at most one range" is fine. Otherwise week --notes and export agreed in every case I tried.

## Verdict

findings(13)
