---
phase: quick/260727-b9u
plan: 01
status: complete
subsystem: frontend/task-form
tags: [ux, i18n, leptos, archetypes, forms]
requires:
  - shared::Archetype API (Welle 0, unverändert)
  - frontend/src/components/task_fields.rs (Feld-Komponenten)
  - frontend/src/components/accordion.rs
provides:
  - frontend/src/components/task_form_model.rs (signalfreie Formular-Logik)
  - TaskModal nach Archetypen (Typ-Karten, Chip, vier Gruppen)
  - utils::task_modal::settings_rewards_enabled / settings_punishments_enabled
affects:
  - frontend/src/components/task_modal.rs
  - frontend/src/components/task_fields.rs
  - frontend/src/utils/task_modal.rs
  - frontend/src/pages/{dashboard,tasks,household}.rs
  - frontend/src/components/quick_task_fab.rs
  - frontend/styles.css
  - frontend/src/translations/{de,en}.json
tech-stack:
  added: []
  patterns:
    - "Signalfreie Formular-Logik als eigenes Modul, damit sie mit #[test] auf dem Host läuft"
    - "Archetyp als Einstieg, nicht als Käfig: Presets setzen Werte, sperren nie Felder"
    - "Accordion.open einmal untracked berechnet (startet offen, folgt danach dem Nutzer)"
key-files:
  created:
    - frontend/src/components/task_form_model.rs
  modified:
    - frontend/src/components/task_modal.rs
    - frontend/src/components/task_fields.rs
    - frontend/src/components/mod.rs
    - frontend/src/i18n/mod.rs
    - frontend/src/utils/task_modal.rs
    - frontend/src/pages/dashboard.rs
    - frontend/src/pages/tasks.rs
    - frontend/src/pages/household.rs
    - frontend/src/components/quick_task_fab.rs
    - frontend/styles.css
    - frontend/src/translations/de.json
    - frontend/src/translations/en.json
decisions:
  - "D-04 umgesetzt: derive_archetype bricht das OneOff/Routine-Patt über die getroffene Auswahl"
  - "D-05 umgesetzt: 'geändert'-Marker sitzt am Chip, Accordion.summary bleibt nicht-reaktiv"
  - "D-06 umgesetzt: nur drei Note-Boxen (Shared, BadHabit, Maintenance)"
  - "Rule 1: task_modal.recurrence_hint ergänzt — TaskRecurrenceTypeField hätte sonst den rohen Key angezeigt"
  - "Rule 1: prop:value auf Wiederholungs- und Gewohnheits-Select, sonst zeigt der Select nach einem Typwechsel den alten Wert"
  - "DRY: settings_rewards_enabled/settings_punishments_enabled statt fünf Kopien von map(..).unwrap_or(true)"
metrics:
  duration: ~50min
  completed: 2026-07-27
---

# Quick 260727-b9u: Task-Formular nach Archetypen Summary

Das Task-Formular fragt jetzt zuerst *was* angelegt wird und leitet die Flags daraus ab; die
restlichen dreizehn Felder liegen in vier aufklappbaren Gruppen, und die Zuweisung steht bei
allen fünf Typen an derselben Stelle.

## Commits

| # | Commit | Titel |
| - | ------ | ----- |
| 1 | `89dcd1c3` | feat(tasks): add signal-free archetype form model |
| 2 | `d97f5b8a` | feat(tasks): restructure the task form around archetypes |
| 3 | `5274d0ee` | feat(tasks): supply the archetype form props at all six call sites |

## Vorher / Nachher

| Kennzahl | Vorher | Nachher |
| -------- | ------ | ------- |
| `task_modal.rs` Zeilen | 1570 | 1571 |
| `task_form_model.rs` Zeilen | – | 844 |
| `frontend --lib` Tests | 57 | 112 |
| `clippy -p frontend` | exit 0, 0 Warnungen | exit 0, 0 Warnungen |
| `clippy -p frontend --all-targets` | 61 Findings (nur Testcode) | 61 Findings (unverändert) |
| i18n-Keys je Sprache | 670 | 708 |
| `styles.css` Zeilen | 2438 | 2558 |

`task_modal.rs` bleibt gleich lang, weil der Reward-/Punishment-Block unverändert übernommen
wurde: was an Feld-Markup wegfiel, kam an Struktur (Karten, Chip, Gruppen) wieder dazu.

## Feldverteilung — zum Gegenprüfen mit dem Mockup

**Vor den Gruppen (immer sichtbar, in dieser Reihenfolge):**

1. Typ-Karten (`task-type-grid`) — nur im Anlege-Modus, fünf Karten in Mockup-Reihenfolge
2. Chip (`task-archetype-chip`) — immer, auch im Bearbeiten-Modus; `changed`-Klasse bei Drift
3. Note-Box (`task-form-note`) — nur bei Shared, BadHabit (info) und Maintenance (danger)
4. **Titel** (`required`)
5. **Datum** (`<input type="date">`) — nur bei OneOff · sonst **Wiederholung** + je nach Wahl
   Wochentag / Monatstag / Wochentage / CalendarPicker
6. **Zuweisung** — bei allen fünf Typen, Label/Pflichtstern/Hinweis aus dem Preset, darunter
   die Fehlermeldung `form-field-error`

**Gruppe „Details" (`task_modal.group.details`):**
- Wiederholung samt Unterfeldern — **nur bei OneOff** (D-03; dort ist das Datum das Basisfeld)
- Beschreibung (Textarea, 4 Zeilen)
- Kategorie (`TaskCategoryField`, nur wenn Kategorien existieren)
- Fällig um (`TaskDueTimeField`)
- Auf Dashboard anzeigen (`TaskOnDashboardField`)

**Gruppe „Ziel & Zählweise" (`task_modal.group.goal`):**
- Zielanzahl (`TaskTargetCountField`)
- Überschreiten erlauben (`TaskAllowExceedField`)
- Gewohnheitstyp (`TaskHabitTypeField`)

**Gruppe „Punkte & Konsequenzen" (`task_modal.group.points`):**
- Punkte bei Erledigung (`TaskPointsRewardField`)
- Punkte-Abzug bei Verpassen (`TaskPointsPenaltyField`)
- Belohnungs-Verknüpfungen — nur wenn `links_section_visible(rewards_enabled, linked.len())`
- Bestrafungs-Verknüpfungen — nur wenn `links_section_visible(punishments_enabled, linked.len())`

**Gruppe „Wer darf was" (`task_modal.group.rules`):**
- Jeder darf abhaken (`TaskAnyoneCanCompleteField`)
- Zugewiesene Person darf nicht zurücknehmen (`TaskAssigneeCannotUncompleteField`)
- Überprüfung erforderlich (`TaskRequiresReviewField`)

**Kein Feld ist verschwunden.** Die 17 Felder von vorher sind vollständig erreichbar; kein
`disabled`, kein Ausgrauen, kein Verstecken hinter einer Typ-Bedingung — die einzige
typabhängige Umschaltung ist, *ob* die Wiederholung als Basisfeld oder in „Details" steht,
und in beiden Fällen ist sie frei bedienbar.

## Bewusste Abweichungen vom Mockup

| ID | Mockup | Umsetzung | Grund |
| -- | ------ | --------- | ----- |
| D-04 | `derive()` gibt bei `current === "oneoff"` immer `oneoff` zurück | zusätzlich: `recurrence == "onetime"` ergibt **immer** `OneOff`, auch bei gewähltem Routine | deckt sich mit `Task::archetype()`; markiert den Fall korrekt als „geändert" |
| D-05 | Badge `#rulesDot` am Summary von „Wer darf was" | Marker nur am Chip | `Accordion.summary` müsste reaktiv werden — die Komponente teilt sich `statistics.rs` |
| D-06 | – | keine Note bei OneOff und Routine | Mockup hat dort `note: null`, nichts erfunden |
| — | Note-Box mit getönten Hex-Hintergründen (`#fef2f2`, `#eff6ff`) | `background-color: var(--background-color)` + farbiger `border-left` | feste Hex-Werte brechen `body.dark-mode` |
| — | `assign.def: "Simon"` auch bei Maintenance | Vorbelegung nur bei BadHabit | Plan-Spezifikation von `assignment_after_preset`; Maintenance verlangt eine bewusste Wahl (Pflichtfeld) |

## Abweichungen vom Plan

### Auto-fixed Issues

**1. [Rule 1 – Bug] Fehlender i18n-Key `task_modal.recurrence_hint`**
- **Gefunden bei:** Task 1, beim Umstieg auf `TaskRecurrenceTypeField`
- **Problem:** Die Komponente rendert `t("task_modal.recurrence_hint")`; der Key fehlte in
  beiden Sprachdateien. Der bisherige Inline-Selektor in `task_modal.rs` hatte gar keinen
  Hinweis, deshalb fiel es nie auf — nach dem Umbau hätte der Nutzer den rohen Key gesehen.
- **Fix:** Key in `de.json` und `en.json` ergänzt.
- **Folge:** **708 statt der geplanten 707 Keys** je Sprache (37 geplante + dieser eine).
  Beide Dateien weiterhin deckungsgleich.
- **Commit:** `89dcd1c3`

**2. [Rule 1 – Bug] Select zeigte nach einem Typwechsel den alten Wert**
- **Gefunden bei:** Task 2
- **Problem:** `TaskRecurrenceTypeField` und `TaskHabitTypeField` setzten die Auswahl nur über
  das `selected`-Attribut der `<option>`. Presets schreiben diese Signale aber von außen; ein
  `<select>`, den der Nutzer schon angefasst hat, folgt einer Attributänderung nicht mehr.
  Konkret: Routine wählen → Wiederholung von Hand auf „einmalig" stellen → „Gemeinsame
  Aufgabe" klicken. Gespeichert wurde korrekt `daily`, angezeigt weiterhin „einmalig".
  Beim Gewohnheitstyp genauso: jeder Wechsel auf/von BadHabit/Maintenance war betroffen.
- **Fix:** `prop:value=move || value.get()` auf beiden `<select>`-Elementen — dasselbe Muster,
  das `TaskAssignedUserField` schon benutzt.
- **Betroffene Datei:** `frontend/src/components/task_fields.rs` (nicht in `files_modified`
  des Plans; `bulk_edit_modal.rs` als zweiter Nutzer profitiert mit, Verhalten dort nur
  korrekter, nie anders)
- **Commit:** `d97f5b8a`

### Bewusste Ergänzungen

**3. [DRY, CLAUDE.md] `settings_rewards_enabled` / `settings_punishments_enabled`**
- Der Plan schrieb an jeder Aufrufstelle `settings.get().map(|s| s.rewards_enabled).unwrap_or(true)`.
  Das wären fünf identische Kopien der D-07-Fallback-Regel gewesen.
- Stattdessen zwei Funktionen in `frontend/src/utils/task_modal.rs`, die an allen fünf
  `Option<HouseholdSettings>`-Stellen benutzt werden (quick_task_fab hat ein nacktes
  `HouseholdSettings` und liest die Felder weiter direkt). Vier Tests dafür.
- Semantik unverändert: fehlende Settings → beide Abschnitte sichtbar.

**4. `RecurrenceFields`-Komponente in `task_modal.rs`**
- Der Wiederholungs-Selektor samt vier Unterfeldern erscheint an zwei Stellen (Basisfeld bzw.
  Gruppe „Details" bei OneOff). Statt den Block zu duplizieren, eine kleine private Komponente.

**5. Zwei i18n-Paritätstests in `frontend/src/i18n/mod.rs`**
- `test_archetype_preset_keys_present_in_both_languages` läuft über `ALL_ARCHETYPES` und prüft,
  dass jeder Key, auf den ein Preset zeigt, in beiden Sprachdateien auflösbar ist. Ein neuer
  Archetyp ohne Übersetzung lässt ab jetzt die Tests rot werden statt den Key anzuzeigen.
- `test_task_form_group_keys_present_in_both_languages` deckt die zehn übrigen neuen Keys ab.

## Prüfkette

| Prüfung | Ergebnis |
| ------- | -------- |
| `cargo check --workspace` | grün |
| `cargo test -p frontend --lib` | **112 passed, 0 failed** (Baseline 57) |
| `cargo test --workspace` | 288 + 112 + 61 + 1 passed, 0 failed |
| `cargo clippy -p frontend` | **exit 0, null Warnungen** |
| `cargo clippy -p frontend --all-targets` | **61 Findings — unverändert**, alle in Testcode |
| i18n-Symmetrie | `de` 708 = `en` 708, symmetrische Differenz leer |
| `CreateTaskRequest` / `UpdateTaskRequest` | im `jj diff` keine Zeile innerhalb der Literale geändert |
| `shared/src/types.rs` | `jj diff --stat` leer |
| `bulk_edit_modal.rs` | `jj diff --stat` leer |
| `grep -rn "Archetype" frontend/src` | nur `task_form_model.rs` und `task_modal.rs` |
| `rewards_enabled=` an Aufrufstellen | dashboard 1, tasks 3, household 1, quick_task_fab 1 = **6** |

## Was nicht geprüft werden konnte

Die Prüfkette deckt Logik, Übersetzungen und Kompilierbarkeit ab. Nicht automatisch geprüft —
das braucht einen Blick im Browser:

- Optik der Typ-Karten, des Chips und der Note-Boxen im Hell- und Dunkelmodus
- Ob die vier Gruppen mit den erwarteten Aufklapp-Zuständen starten (Logik ist getestet, die
  Verdrahtung der `open`-Props nicht)
- Das Datumsfeld bei OneOff (`prop:value` ↔ `on:input`-Runde durch die beiden Signale)
- Touch-Ziele auf einem echten Telefon (CSS setzt `min-height: 44px`)

`#[wasm_bindgen_test]`s werden in diesem Projekt nur kompiliert, nicht ausgeführt — deshalb
sitzt jede neue Logik als `#[test]` in `task_form_model.rs`.

## Known Stubs

Keine.

## Self-Check: PASSED

- `frontend/src/components/task_form_model.rs` — vorhanden (844 Zeilen)
- Commits `89dcd1c3`, `d97f5b8a`, `5274d0ee` — in `jj log` vorhanden
- Alle in `key-files.modified` genannten Dateien sind in den drei Commits enthalten
