---
task: 260727-a1u
title: "Task-Archetypen Welle 0: Archetype-Enum + Ableitung in shared/src/types.rs"
type: quick
status: complete
created: 2026-07-27
completed: 2026-07-27
wave: 0
files_modified:
  - shared/src/types.rs
commits:
  - hash: f50620a1
    message: "feat(tasks): derive task archetypes from existing flags"
  - hash: 7fe821a8
    message: "feat(tasks): add archetype presets and assignment requirement"
tests_added: 12
tests_total_shared: 59
---

# 260727-a1u: Task-Archetypen — Welle 0 — Summary

`Task` ist polymorph: dieselbe Struct bedeutet je nach vier Schaltern etwas völlig anderes. Diese
Welle legt die gemeinsame Ableitung darunter — `Archetype` als reines Darstellungskonzept, plus die
Gegenrichtung `Archetype::defaults()` für spätere Formular- und Kartenwellen. Rein additiv,
ausschließlich `shared/src/types.rs`.

## Was entstanden ist

| Artefakt | Ort | Zweck |
| -------- | --- | ----- |
| `pub enum Archetype` | nach `impl Task`, vor `CreateTaskRequest` | 5 Varianten: `OneOff`, `Routine`, `Shared`, `BadHabit`, `Maintenance`; `snake_case`-serde wie `ConditionType` |
| `Task::archetype()` | im bestehenden `impl Task`-Block, neben `is_household_wide()` | Ableitung aus vorhandenen Flags, feste Prioritätsreihenfolge |
| `pub struct ArchetypeDefaults` | nach `enum Archetype` | Flag-Preset (ohne `Copy`, da `RecurrenceType` nicht `Copy` ist) |
| `Archetype::defaults()` | `impl Archetype` | Gegenrichtung, exhaustives `match` über alle Varianten |
| `Archetype::assignment_required()` | `impl Archetype` | `true` nur für `Maintenance` |

Die Prioritätsreihenfolge in `archetype()` ist exakt wie im Design-Contract §2 umgesetzt
(`assignee_cannot_uncomplete` → `habit_type.is_inverted()` → `anyone_can_complete` →
`OneTime` → sonst `Routine`), als `if`-Kette. Beide geforderten Doc-Comment-Absätze sind drin:
Darstellung-vs-Berechtigung („der Archetyp bestimmt ausschließlich die Darstellung, was jemand
tatsächlich darf kommt weiterhin aus `can_complete()`/`can_uncomplete()`") und die Begründung der
Reihenfolge. `defaults()` trägt den `Maintenance`/`is_household_wide()`-Hinweis,
`assignment_required()` die `is_assignee()`-Begründung.

## Tests (12 neu, TDD)

Beide Tasks liefen echt rot zuerst — Task 1 mit 20 Compile-Fehlern (`no method named archetype`,
`undeclared type Archetype`), Task 2 mit 4 (`cannot find type ArchetypeDefaults`, `no method named
defaults`/`assignment_required`).

- 10 Ableitungstests (`test_archetype_*`), davon 5 Grenzfälle für die Prioritätsreihenfolge:
  `Bad`+`OneTime` → `BadHabit`, `Good`+`assignee_cannot_uncomplete` → `Maintenance`,
  `Bad`+`anyone_can_complete` → `BadHabit`, `assignee_cannot_uncomplete`+`anyone_can_complete` →
  `Maintenance`, `Bad`+`assignee_cannot_uncomplete` → `Maintenance`
- `test_archetype_defaults_round_trip` — iteriert über `ALL_ARCHETYPES` (einzige Aufzählungsstelle),
  wendet die Presets via `apply_defaults()` auf `create_base_task()` an
- `test_archetype_assignment_required` — `true` nur für `Maintenance`

**Mutationsprobe:** Ein grüner Round-Trip-Test nach reinem Compile-Fehler-RED beweist noch nicht,
dass die Assertions greifen. Gegenprobe: `Shared`-Preset auf `anyone_can_complete: false` gesetzt →
`assertion left == right failed: round-trip failed for Shared`. Danach zurückgedreht und per `diff`
gegen die Vorabsicherung als byte-identisch bestätigt.

`shared`: 47 → 59 Tests, alle grün.

## DRY-Refactor am Test-Helfer

`create_base_task()` (nackter `Task`, `RecurrenceType::Daily`, `HabitType::Good`, keine Sonderflags)
neu angelegt; `create_task_with_status_full` baut seinen `Task` jetzt darüber und überschreibt nur
noch `target_count`, `allow_exceed_target`, `anyone_can_complete`. Keine Signatur geändert, alle
vorher existierenden Tests unverändert grün.

`RecurrenceType::Daily` als Basis ist tragend: der Round-Trip verlässt sich darauf, dass ein Preset
mit `recurrence_type: None` einen wiederkehrenden Task wiederkehrend lässt.

## Verifikation

| Kommando | Ergebnis |
| -------- | -------- |
| `nix develop -c cargo test -p shared` | 59 passed, 0 failed |
| `nix develop -c cargo clippy -p shared --all-targets` | sauber, keine Warnung |
| `nix develop -c cargo check --workspace` | grün (shared, backend, frontend) — die additive Änderung bricht nichts |

## Abgrenzung eingehalten

Rein additiv. `can_complete()`, `can_uncomplete()`, `is_assignee()`, `is_completable_by_user()` und
`is_household_wide()` sind Zeile für Zeile unverändert. Kein bestehendes Feld, keine Signatur
angefasst, kein Backend, kein Frontend, keine Migration, kein `cargo sqlx prepare` nötig.

## Bestandsschuld (nicht angefasst)

`nix develop -c cargo clippy --workspace` schlägt weiterhin fehl — Backend- und Frontend-Findings,
u. a. `frontend/src/components/solo_mode_banner.rs` `clippy::type_complexity`. Das ist
vorbestehend (siehe STATE.md Blockers und `phases/02.1-.../deferred-items.md`), lag außerhalb des
Scopes und wurde bewusst nicht mitrepariert. Verbindliche Schranke hier war `-p shared
--all-targets`, und die ist sauber.

## Deviations from Plan

Keine inhaltlichen Abweichungen — der Design-Contract wurde exakt umgesetzt.

Eine Verfahrensabweichung: Der Plan nennt im Abschnitt „Commit" einen einzelnen Commit. Die
Executor-Vorgabe verlangt bei mehreren Code-Tasks je einen atomaren Commit pro Task, deshalb sind es
zwei geworden (Task 1 mit der Commit-Message aus dem Plan, Task 2 mit eigener). Beide via
`jj commit shared/src/types.rs -m ...` mit Pfadfilter, damit die Planungsdokumente uncommitted im
Working Copy bleiben.

## Nachtrag: Korrektur durch den Orchestrator (Commit `6b9e5bb6`)

Das `Maintenance`-Preset stand auf `habit_type: HabitType::Good`. Fachlich ist eine Instandhaltung
aber immer invertiert: Das Abhaken meldet einen *Verstoß*. Mit `Good` hätte eine gemeldete
Verfehlung Punkte **gutgeschrieben** statt abgezogen.

Der Round-Trip-Test kann das prinzipiell nicht fangen — Regel 1 leitet `Maintenance` allein aus
`assignee_cannot_uncomplete` ab, unabhängig vom `habit_type`. Das Preset braucht deshalb eine
eigene Zusicherung. Ergänzt:

- `Archetype::Maintenance` → `habit_type: HabitType::Bad`
- `test_archetype_defaults_invert_points_where_completing_is_the_failure` — `Maintenance` und
  `BadHabit` müssen invertiert sein
- `test_archetype_defaults_keep_points_upright_for_chores` — `OneOff`, `Routine`, `Shared` nicht

Mutationsprobe: Preset testweise auf `Good` zurückgesetzt → neuer Test schlägt fehl; zurückgesetzt
und Byte-Identität per `diff` bestätigt. Danach 61 Tests grün, `clippy -p shared --all-targets`
sauber.

Ursache lag in der Aufgabenstellung, nicht in der Ausführung: Der Plan gab als einzige Vorgabe für
die Preset-Werte „so wählen, dass der Round-Trip aufgeht" — und das erfüllt `Good` ebenfalls.

## Known Stubs

Keine.

## Self-Check: PASSED

- `shared/src/types.rs` vorhanden, enthält `pub enum Archetype`, `pub struct ArchetypeDefaults`,
  `Task::archetype()`, `Archetype::defaults()`, `Archetype::assignment_required()`
- Commit `f50620a1` vorhanden (Task 1, 158 insertions / 27 deletions in `shared/src/types.rs`)
- Commit `7fe821a8` vorhanden (Task 2)
- Working Copy enthält nur noch Planungsdokumente — kein Code uncommitted

## Anschluss

Welle 1 (Formular) und Welle 2 (Karte) können auf `Task::archetype()` und `Archetype::defaults()`
aufsetzen. `assignment_required()` ist dabei die Stelle, an der ein Formular für `Maintenance` eine
Zuweisung erzwingen muss — ohne Zuweisung greift die `can_uncomplete()`-Sperre nie.
