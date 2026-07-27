---
created: 2026-07-27T07:27:33.047Z
completed: 2026-07-27
resolved_by: quick 260727-dct (commit 3f8ea3db)
title: Freiform-Tasks (target_count 0) werden fälschlich bestraft
area: backend
files:
  - backend/src/services/background_jobs.rs:265-280
  - backend/src/services/report.rs:147-173
  - shared/src/types.rs:789 (TaskWithStatus::is_target_met)
---

## Problem

`process_missed_tasks` bestraft Tasks mit `target_count = 0`, wenn sie an einem Tag **gar nicht**
erledigt wurden. Das widerspricht der Bedeutung von `target_count = 0`, die an drei Stellen im Code
ausdrücklich als "free-form / kein zu erreichendes Ziel" dokumentiert ist.

`backend/src/services/background_jobs.rs:271`:

```rust
let counts_as_done = if task.habit_type.is_inverted() || task.target_count <= 0 {
    completion_count > 0
} else {
    completion_count >= i64::from(task.target_count)
};

if counts_as_done { continue; }   // sonst: missed_task_penalty
```

Bei `target_count = 0` und `completion_count = 0` ergibt der erste Zweig `false` → es wird eine
Strafe gesetzt. Ein Task ohne Soll kann aber nicht verfehlt werden.

Der vorhandene Test `test_missed_tasks_free_form_task_completed_once_is_not_missed`
(`background_jobs.rs:1043`) prüft nur den Fall *einmal erledigt → keine Strafe*. Der Fall
*gar nicht erledigt* ist ungetestet — deshalb ist das nie aufgefallen.

Gefunden am 2026-07-27 beim Entwurf des Archetyps "Bonusaufgabe" (siehe
[2026-07-27-bonusaufgabe-als-sechster-archetyp.md](./2026-07-27-bonusaufgabe-als-sechster-archetyp.md)).
Der Bug ist aber **unabhängig davon** und betrifft alle bereits existierenden Tasks mit
`target_count = 0`.

## Solution

In `background_jobs.rs` die beiden heute zusammengefassten Fälle trennen — sie haben nichts
miteinander zu tun:

```rust
// Kein Soll: kann nicht verfehlt werden.
if task.target_count <= 0 && !task.habit_type.is_inverted() {
    continue;
}
```

Zwei Tests ergänzen:
- Freiform-Task (`target_count = 0`, `HabitType::Good`) ohne jede Erledigung → `missed_tasks == 0`
- Der bestehende Fall "einmal erledigt" bleibt grün

**Vorher zu klären:** Wie soll sich `target_count = 0` in Kombination mit
`habit_type.is_inverted()` verhalten? Der aktuelle Code behandelt beide im selben Zweig, und für
invertierte Habits sieht die Logik ebenfalls verdächtig aus: `counts_as_done = completion_count > 0`
bedeutet, dass eine schlechte Angewohnheit **ohne** Rückfall als "nicht erledigt" gilt und bestraft
wird. Das könnte an anderer Stelle kompensiert werden (Punkte-Conditions, `report.rs`) — vor dem
Fix prüfen, sonst repariert man die eine Hälfte und bricht die andere.

Auch prüfen: `report.rs:173` (`task.target_count > 0 && count >= ...`) spiegelt dieselbe Definition
von "done" und ist von der Änderung mitbetroffen.
