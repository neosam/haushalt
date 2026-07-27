---
quick_id: 260727-fcg
description: Bonusaufgabe als sechster Task-Archetyp
date: 2026-07-27
status: complete
commits:
  - ac0d590a (backend, Periodenstatus)
  - f43c0238 (shared + frontend + i18n)
area: general
---

# Quick 260727-fcg — Summary

## Entscheidungen des Nutzers, die umgesetzt wurden

1. **Kein Verfehlen, aber Punkte** → `points_penalty` bei Bonus ausgeblendet und beim Typwechsel
   geleert; `points_reward` unverändert verfügbar.
2. **Verlauf** → erledigt grün, nicht erledigt neutral, nie rot.
3. **Tagesreport** → unangetastet (`report.rs` nicht berührt).

## Befund, der das Todo korrigiert hat

Das Todo hielt fest, der `PeriodTracker` zeige für Bonusaufgaben „vermutlich lauter rote ✗", weil
`is_target_met()` bei `target_count = 0` immer `false` ist, und folgerte: „Backend: keine weitere
Arbeit nötig."

**Beides traf nicht zu.** Der `PeriodTracker` liest `period_results`, nicht `is_target_met()`. Die
entstehen in `background_jobs.rs:616` aus `completions_count >= task.target_count` — bei Soll 0 ist
das `0 >= 0` und damit **immer wahr**. Ohne Änderung hätte eine Bonusaufgabe lauter *grüne* Haken
bekommen, auch an Tagen, an denen niemand sie angefasst hat: das genaue Gegenteil von
Entscheidung 2, und schlimmer als das befürchtete Rot, weil es Erfolge erfindet.

Deshalb enthält die Umsetzung einen Backend-Task, den das Todo nicht vorgesehen hatte
(Commit `ac0d590a`): Tasks ohne Soll bekommen einen eigenen Zweig — erledigt → `Completed`, nicht
erledigt → `Skipped`. `Skipped` ist richtig, weil es neutral rendert und — anders als `Failed` —
keinen Streak bricht (`period_results.rs:254`).

## Zweiter Befund: `Maintenance` mit invertiertem `habit_type` ist Absicht

Sah nach Copy-Paste aus `BadHabit` aus, ist aber durch
`test_archetype_defaults_invert_points_where_completing_is_the_failure` (`types.rs:2522`)
ausdrücklich abgesichert: Bei Instandhaltung dokumentiert das Abhaken einen *Verstoß*, deshalb
laufen die Punkte invertiert. Nicht angefasst.

## Änderungen

| Datei | Was |
|-------|-----|
| `shared/src/types.rs` | `Archetype::Bonus`; `ArchetypeDefaults.target_count`; Ableitung an Position 3 (`<= 0`) |
| `backend/src/services/background_jobs.rs` | Perioden-Zweig für Tasks ohne Soll |
| `frontend/.../task_form_model.rs` | Preset, `shows_target_count`/`shows_points_penalty`, `target_count_after_preset`, `parse_target_count`; `initial_open_groups` vergleicht gegen das Preset statt gegen ein hartcodiertes „1" |
| `frontend/.../task_modal.rs` | sechste Typkarte, Feld-Sichtbarkeit, Strafwert leeren beim Wechsel |
| `frontend/.../task_card.rs` | Zähler `"3 ×"` statt `"3/0"` |
| `frontend/.../period_tracker.rs` | `period_appearance()` herausgezogen (war in beiden Komponenten dupliziert), Prop `is_bonus` |
| `frontend/src/translations/{de,en}.json` | je 6 neue Keys |

## Abweichung vom Todo: `i32` statt `Option<i32>`

Das Todo schlug `target_count: Option<i32>` vor, verlangte im selben Absatz aber, dass *alle*
Presets den Wert explizit setzen — sonst bliebe beim Zurückwechseln von Bonus die `0` stehen. Wenn
ohnehin jede Variante einen konkreten Wert trägt, ist `Option` nur eine Fehlerquelle; `i32`
erzwingt die Vollständigkeit über den Compiler. Abgesichert durch
`test_archetype_defaults_pin_the_target_count`.

## Tests

19 neue Tests:

- **shared (7):** Ableitung aus Soll 0 und aus negativem Soll; Bonus schlägt Shared; BadHabit und
  Maintenance schlagen Bonus; alle Presets pinnen `target_count`; Round-Trip über sechs Archetypen
- **backend (2):** unberührter Tag → `Skipped` und nie `Failed`; erledigter Tag → `Completed`
- **frontend (10):** Ableitung inkl. leerem Eingabefeld (darf nicht auf Bonus springen);
  Typwechsel-Round-Trip; Feld-Sichtbarkeit; Gruppen-Aufklappen; vier Tests für
  `period_appearance` inkl. „Bad Habit schlägt Bonus"

`nix develop -c cargo test --workspace` → 486 Tests grün, 0 failed.

## Qualitätsgates

`nix develop -c cargo clippy --workspace --all-targets` → 69 Findings, exakt die in STATE.md
dokumentierte Vorbelastung (6 in `backend/src/services/tasks.rs`, 61 im Frontend). Keine aus einer
der geänderten Dateien.

## Offen

- Der Tagesreport (`report.rs:173`) zeigt Bonusaufgaben weiterhin nie als `(done)` — auf Wunsch des
  Nutzers vorerst ignoriert.
- Die Aktionsknöpfe der Karte sind unverändert (`+`/`−`). Das Mockup schlägt für die anderen
  Archetypen sprechende Knöpfe vor („Rückfall eintragen", „Verstoß melden") — das ist Welle 2 und
  war hier nicht im Scope.
