---
created: 2026-07-27
completed: 2026-07-27
resolved_by: quick 260727-fcg (commits ac0d590a, f43c0238)
title: Bonusaufgabe als sechster Task-Archetyp
area: general
note: >
  Die Annahme "Backend: keine weitere Arbeit nötig" war falsch — die Perioden-Finalisierung
  wertete bei Soll 0 jeden Tag als Completed. Details im SUMMARY des Quick-Tasks.
  Offene Entscheidungen 1 und 2 wurden vom Nutzer am 2026-07-27 beantwortet.
files:
  - shared/src/types.rs:669-763 (Archetype, ArchetypeDefaults)
  - frontend/src/components/task_form_model.rs
  - frontend/src/components/task_modal.rs
  - frontend/src/components/task_card.rs
  - frontend/src/components/period_tracker.rs
  - backend/src/services/background_jobs.rs:265-280
---

## Problem

Es fehlt ein sechster Archetyp: die **Bonusaufgabe**. Sie muss nicht erledigt werden — das Soll ist
0 — wird aber trotzdem getrackt, wenn jemand sie macht. Nutzeranforderung vom 2026-07-27.

Die Datenlage dafür existiert bereits: `target_count = 0` ist an mehreren Stellen als "free-form"
dokumentiert (`shared/src/types.rs:789`, `background_jobs.rs:269`, `report.rs:149`). Die Semantik
hat also nur keinen Namen und keine Oberfläche.

Kontext: Die Archetypen-Ableitung wurde am 2026-07-27 in drei Quick-Tasks eingeführt
(`260727-a1u` shared, `260727-apd` Bulk-Edit-Extraktion, `260727-b9u` Formularumbau). Die fünf
bestehenden Typen sind OneOff, Routine, Shared, BadHabit, Maintenance. Design-Referenz ist das
Mockup unter `.planning/quick/260727-b9u-welle-1b-task-formular-nach-archetypen-u/260727-b9u-MOCKUP.html`
(im Browser öffnen — Typ oben wechseln, unten Schalter umlegen: der Chip leitet den Typ live neu ab).

**Blocker:** Bonusaufgaben werden vom Backend aktuell bestraft, wenn sie nicht erledigt werden —
siehe [2026-07-27-freiform-tasks-werden-faelschlich-bestraft.md](./2026-07-27-freiform-tasks-werden-faelschlich-bestraft.md).
Ohne diesen Fix ist der Typ funktional sinnlos. Der Fix ist unabhängig und sollte zuerst kommen.

## Solution

### 1. shared

- `Archetype::Bonus` ergänzen
- `ArchetypeDefaults` um `target_count: Option<i32>` erweitern
- Ableitungsregel **an Position 3** einfügen:

  1. `assignee_cannot_uncomplete` → Maintenance
  2. `habit_type.is_inverted()` → BadHabit
  3. **`target_count == 0` → Bonus**
  4. `anyone_can_complete` → Shared
  5. `recurrence == OneTime` → OneOff
  6. sonst → Routine

  Begründung: "kein Soll" verändert die Karte stärker als die Frage, wer abhaken darf, aber
  schwächer als die beiden Typen mit eigener Punktesemantik. So bleibt eine schlechte Angewohnheit
  mit Soll 0 weiterhin BadHabit.

- **Wichtig für den Round-Trip:** Alle Presets müssen `target_count` ab dann *explizit* setzen —
  Bonus auf `0`, die übrigen fünf auf `1`. Sonst bliebe beim Typwechsel von Bonus zurück auf
  Routine die `0` stehen und die Ableitung kippt sofort wieder auf Bonus. Der bestehende
  Round-Trip-Test würde das nur zufällig nicht bemerken, weil `create_base_task()`
  `target_count: 1` hat.

### 2. Backend

Voraussetzung: der verlinkte Straf-Bug. Danach keine weitere Backend-Arbeit nötig.

### 3. Frontend

- Sechste Typ-Karte in der Auswahl (`task_form_model.rs` Presets + `task_modal.rs`)
- Basisfelder wie Routine: Wiederholung + Zuweisung ("Zugewiesen an", optional)
- Zielanzahl in der Gruppe "Ziel & Zählweise" ausblenden oder auf 0 fixieren
- `task_card.rs`: Zähler ohne `/N` — nur "3 ×" statt "3/0"; kein Fortschrittsbalken;
  `is_target_met()` ist per Definition immer `false`, die Karte darf daraus **nicht** "offen/rot"
  ableiten
- i18n: Typname, Kurzbeschreibung, Hinweistext, Zuweisungs-Label + Hilfetext in `de.json` und
  `en.json` (beide Dateien müssen dieselbe Key-Menge behalten — aktuell 708)

### Offene Entscheidungen (vom Nutzer noch nicht beantwortet)

1. **Punkte bei Verfehlen** — Vorschlag: für Bonus ganz ausblenden und beim Typwechsel auf `None`
   setzen, da es kein Verfehlen gibt. Alternative: Feld erreichbar lassen.
2. **Streak und Historie** — bei `target_count = 0` wird eine Periode nie "completed"
   (`is_target_met()` immer `false`). Der `PeriodTracker` zeigt für Bonusaufgaben dann vermutlich
   lauter rote ✗. Muss zusammen mit der Karten-Darstellung (Welle 2) geprüft und eigens behandelt
   werden — für Bonus wäre richtig: erledigt = grün, nicht erledigt = neutral, nie rot.
