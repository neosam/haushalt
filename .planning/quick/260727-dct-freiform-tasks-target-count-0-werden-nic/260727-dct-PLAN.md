---
quick_id: 260727-dct
description: Freiform-Tasks (target_count 0) werden nicht mehr fälschlich bestraft
date: 2026-07-27
area: backend
todo: .planning/todos/pending/2026-07-27-freiform-tasks-werden-faelschlich-bestraft.md
---

# Quick 260727-dct: Freiform-Tasks werden nicht mehr fälschlich bestraft

## Problem

`process_missed_tasks` (`backend/src/services/background_jobs.rs:271`) fasst zwei fachlich
unabhängige Fälle in einer Bedingung zusammen:

```rust
let counts_as_done = if task.habit_type.is_inverted() || task.target_count <= 0 {
    completion_count > 0
} else { ... };
```

Bei `target_count = 0` **und** `completion_count = 0` ergibt der Zweig `false` → der Task fällt in
den Straf-Block. Ein Task ohne Soll kann aber nicht verfehlt werden. Der Kommentar bei Zeile 206
(`// Skip free-form and one-time tasks (they can't be "missed")`) beschreibt bereits die
beabsichtigte Semantik — der Code setzt für free-form aber nichts davon um.

## Vorab geklärt: `target_count = 0` in Kombination mit `is_inverted()`

Das Todo verlangt, vor dem Fix das Verhalten invertierter Habits zu prüfen — Verdacht: eine
schlechte Angewohnheit ohne Rückfall werde bestraft. **Der Verdacht bestätigt sich nicht.**

`background_jobs.rs:315` verzweigt innerhalb des vermeintlichen Straf-Blocks:

- `is_inverted()` → `award_bad_habit_avoided_points` + `assign_bad_habit_avoided_rewards`
- sonst → `deduct_missed_task_points` + `assign_missed_task_punishments`

Für invertierte Habits ist "nicht erledigt" also der **Belohnungspfad** und damit korrekt. Nur die
Zählvariable `missed_tasks` und der Blockname sind irreführend benannt.

**Konsequenz für diesen Fix:** Der invertierte Zweig bleibt unangetastet. Ein Bad Habit mit
`target_count = 0` verhält sich weiter wie bisher (Reward bei Nicht-Ausüben) — konsistent mit der
Archetypen-Ableitung, in der BadHabit vor Bonus gewinnt.

## Nicht im Scope

`report.rs:173` (`task.target_count > 0 && count >= target_count`) spiegelt `is_target_met` und
lässt Freiform-Tasks im Tagesreport nie als `(done)` erscheinen. Das ist eine **Anzeigefrage**, kein
Straf-Bug, und gehört zur Karten-/Report-Darstellung des Archetyps "Bonusaufgabe"
(siehe `.planning/todos/pending/2026-07-27-bonusaufgabe-als-sechster-archetyp.md`, Welle 2). Eine
Änderung hier würde die dokumentierte Report-Semantik D-07 verschieben — bewusst ausgelassen.

## Tasks

### Task 1 — Freiform-Fall aus der `counts_as_done`-Bedingung herauslösen

**Files:** `backend/src/services/background_jobs.rs`

**Action:**
- Direkt nach `tasks_checked += 1;` einen frühen Ausstieg einfügen:
  ```rust
  // Kein Soll: ein Task ohne Zielanzahl kann nicht verfehlt werden. Für invertierte
  // Habits gilt das nicht — dort ist "nicht ausgeübt" der Belohnungspfad (siehe unten).
  if task.target_count <= 0 && !task.habit_type.is_inverted() {
      continue;
  }
  ```
- `counts_as_done` auf die verbleibenden zwei Fälle reduzieren (`|| task.target_count <= 0` entfällt)
  und den Kommentar entsprechend kürzen.
- Irreführenden Kommentar bei Zeile 206 korrigieren: dort wird nur `OneTime` übersprungen.

**Verify:** `nix develop -c cargo check -p backend`

**Done:** Ein nicht-invertierter Task mit `target_count <= 0` erreicht den Straf-Block nicht mehr.

### Task 2 — Tests

**Files:** `backend/src/services/background_jobs.rs` (`#[cfg(test)] mod tests`)

**Action:** Zwei Tests im Stil der bestehenden `setup_missed_task_env`-Tests ergänzen:
- `test_missed_tasks_free_form_task_never_completed_is_not_missed` — `target_count = 0`,
  `HabitType::Good`, keine Erledigung → `missed_tasks == 0` (der eigentliche Bug)
- `test_missed_tasks_bad_habit_with_zero_target_still_processed` — `target_count = 0`,
  `HabitType::Bad`, keine Erledigung → `missed_tasks == 1`; hält fest, dass der Fix den
  invertierten Belohnungspfad nicht mit abschneidet

Der bestehende `test_missed_tasks_free_form_task_completed_once_is_not_missed` bleibt unverändert
grün.

**Verify:** `nix develop -c cargo test -p backend background_jobs`

**Done:** Beide neuen Tests grün, keine Regression in der bestehenden Suite.

### Task 3 — Qualitätsgates

**Action:** `nix develop -c cargo test --workspace` und `nix develop -c cargo clippy -p backend`

**Bekannte Vorbelastung:** Für `backend/src/services/tasks.rs` und ~20 Frontend-Dateien existieren
vorbestehende clippy-Findings (siehe STATE.md Blockers). Nur neu hinzugekommene Findings zählen.

**Done:** Keine neuen Warnungen aus den geänderten Zeilen.

## must_haves

**Truths:**
- Ein Task mit `target_count = 0` und `HabitType::Good` wird nie bestraft — unabhängig davon, ob er
  erledigt wurde
- Der Belohnungspfad für invertierte Habits bleibt in allen `target_count`-Konstellationen erhalten

**Artifacts:**
- `backend/src/services/background_jobs.rs` — früher Ausstieg + zwei neue Tests

**Key links:**
- `backend/src/services/background_jobs.rs:271` (Bug)
- `backend/src/services/background_jobs.rs:315` (invertierter Belohnungszweig)
