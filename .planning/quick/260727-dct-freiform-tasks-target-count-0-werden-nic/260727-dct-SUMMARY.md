---
quick_id: 260727-dct
description: Freiform-Tasks (target_count 0) werden nicht mehr fälschlich bestraft
date: 2026-07-27
status: complete
commit: 3f8ea3db
area: backend
---

# Quick 260727-dct — Summary

## Was geändert wurde

`backend/src/services/background_jobs.rs` — `process_missed_tasks`:

1. **Früher Ausstieg für Tasks ohne Soll** (nach `tasks_checked += 1`):
   ```rust
   if task.target_count <= 0 && !task.habit_type.is_inverted() {
       continue;
   }
   ```
2. **`counts_as_done` auf zwei Fälle reduziert** — `|| task.target_count <= 0` entfällt, weil der
   Fall jetzt oberhalb abgefangen wird.
3. **Irreführenden Kommentar korrigiert:** Der Skip bei Zeile 206 behauptete, free-form Tasks zu
   überspringen, prüft aber nur `RecurrenceType::OneTime`.

## Vorab geklärte Frage (aus dem Todo)

Das Todo verlangte, das Verhalten invertierter Habits bei `target_count = 0` zu prüfen — Verdacht:
eine schlechte Angewohnheit ohne Rückfall werde bestraft.

**Der Verdacht bestätigt sich nicht.** `background_jobs.rs:315` verzweigt innerhalb des vermeintlichen
Straf-Blocks: bei `is_inverted()` läuft `award_bad_habit_avoided_points` +
`assign_bad_habit_avoided_rewards`, sonst `deduct_missed_task_points` +
`assign_missed_task_punishments`. Für invertierte Habits ist "nicht erledigt" also der
**Belohnungspfad**. Nur die Zählvariable `missed_tasks` und der Blockname sind irreführend benannt.

Der invertierte Zweig blieb deshalb unangetastet — konsistent mit der Archetypen-Ableitung, in der
BadHabit vor Bonus gewinnt.

## Tests

Zwei Tests in `#[cfg(test)] mod tests` ergänzt:

| Test | Prüft |
|------|-------|
| `test_missed_tasks_free_form_task_never_completed_is_not_missed` | `target_count = 0`, `Good`, keine Erledigung → `missed_tasks == 0` (der eigentliche Bug — war vor dem Fix rot) |
| `test_missed_tasks_bad_habit_with_zero_target_still_processed` | `target_count = 0`, `Bad`, keine Erledigung → `missed_tasks == 1`; Regressionsschutz für den Belohnungspfad |

Der bestehende `test_missed_tasks_free_form_task_completed_once_is_not_missed` blieb unverändert grün.

## Qualitätsgates

- `nix develop -c cargo test --workspace` → 464 Tests grün, 0 failed (backend 290, shared 112, frontend 61, +1)
- `nix develop -c cargo clippy -p backend --all-targets` → 6 Findings, alle in
  `backend/src/services/tasks.rs` (vorbestehend, siehe STATE.md Blockers). Keine aus
  `background_jobs.rs`.

## Bewusst nicht im Scope

`report.rs:173` (`task.target_count > 0 && count >= target_count`) lässt Freiform-Tasks im
Tagesreport nie als `(done)` erscheinen. Das ist eine Anzeigefrage, kein Straf-Bug, und gehört zur
Karten-/Report-Darstellung des Archetyps "Bonusaufgabe" (Welle 2). Eine Änderung würde die
dokumentierte Report-Semantik D-07 verschieben.

## Folgewirkung

Der Blocker für den Archetyp "Bonusaufgabe"
(`.planning/todos/pending/2026-07-27-bonusaufgabe-als-sechster-archetyp.md`) ist damit aufgehoben.
