---
quick_id: 260728-cej
description: "Bug: Task verschwindet nach Bearbeiten aus der Haushalt-Uebersicht"
date: 2026-07-28
status: complete
---

# Quick Task 260728-cej — Zusammenfassung

Gemeldetes Symptom: Ein in der Übersicht bearbeiteter Task ist nach dem Speichern
nicht mehr da. Ursache waren drei ineinandergreifende Defekte, alle drei behoben.

## Was geändert wurde

### 1. Backend liefert die Kategorie mit (`backend/src/services/tasks.rs`)

`update_task` las den Task mit `TaskRow` (`SELECT * FROM tasks`) und gab
`task.to_shared()` zurück — ohne den JOIN auf `task_categories` waren
`category_name` und `category_color` in der Antwort immer `None`. `create_task`
setzte seine Antwort ebenso von Hand mit `category_name: None` zusammen. Beide
geben jetzt den über `get_task` geladenen Task zurück, wie es `archive_task`,
`unarchive_task`, `pause_task` und `unpause_task` bereits taten.

Tests: `test_create_task_returns_category`, `test_update_task_returns_category`,
`test_update_task_clearing_category_drops_name_and_color`.

### 2. Übersicht lädt nach dem Speichern neu (`frontend/src/pages/household.rs`)

`on_task_save` ersetzte nur `t[pos].task`. Der umgebende `TaskWithStatus` behielt
dadurch sein berechnetes `next_due_date`, sodass eine geänderte Wiederholung sich
nicht auf die Tagesgruppe auswirkte. Die Seite lädt die Liste jetzt über
`get_all_tasks_with_status` neu — dasselbe Muster wie `on_save_date` und
`on_context_pause` auf derselben Seite.

### 3. Aufklappzustand überlebt das Neuladen (`frontend/src/components/task_card.rs`)

`GroupedTaskList` rendert Tages- und Kategoriegruppen als `<details>` mit festem
`open`-Attribut; nur „Heute" startet offen. Da jede Listenaktualisierung den DOM neu
baut, klappte eine vom Nutzer geöffnete Gruppe wieder zu — der dort bearbeitete Task
war danach unsichtbar. Neu:

- `DueDateGroup::state_key()` und `category_state_key()` liefern stabile,
  übersetzungsunabhängige Schlüssel je Gruppe.
- Neuer optionaler Prop `group_states: RwSignal<GroupStates>`; `HouseholdPage` und
  `DashboardPage` halten das Signal auf Seitenebene, sodass es die Re-Renders überlebt.
- Gelesen wird `with_untracked`, geschrieben `update_untracked` im `toggle`-Handler —
  sonst würde jeder Klick auf eine Gruppe die Liste unter sich selbst neu bauen.

Tests: `test_state_key_is_distinct_per_group`, `test_state_key_ignores_weekday_label`,
`test_category_state_key_is_scoped_to_its_date_group`,
`test_group_open_state_falls_back_to_default`,
`test_group_open_state_prefers_remembered_choice`.

`web-sys` bekam das Feature `HtmlDetailsElement` (Workspace-`Cargo.toml`), um den
offenen Zustand typsicher aus dem `toggle`-Event zu lesen.

## Verifikation

- `cargo test --workspace`: 303 Backend- + 161 Frontend- + 67 shared-Tests grün.
- `cargo clippy --workspace`: sauber.
- `cargo check --target wasm32-unknown-unknown` im Frontend: sauber.

Nicht verifiziert: die visuelle Gegenprobe im laufenden UI — die Chrome-Extension war
in dieser Session nicht verbunden. Der laufende Backend-Prozess trägt den Fix erst
nach einem Neustart (`cargo run -p backend`); der trunk-Dev-Server hat das Frontend
bereits neu gebaut.

## Offen / angrenzend

- `cargo clippy --workspace --all-targets` meldet vorbestehende `clippy::eq_op`-Funde
  in 19 Dateien (tautologische Tests wie `assert_eq!("loading", "loading")`) — nicht
  Teil dieses Fixes, aber einen eigenen Aufräum-Task wert.
- `HouseholdPage` wiederholt `get_all_tasks_with_status` an sieben Stellen; ein
  gemeinsamer `reload_tasks`-Helfer wie im Dashboard wäre der nächste DRY-Schritt.
