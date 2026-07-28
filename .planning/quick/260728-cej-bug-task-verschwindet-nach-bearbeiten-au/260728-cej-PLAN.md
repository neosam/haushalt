---
quick_id: 260728-cej
description: "Bug: Task verschwindet nach Bearbeiten aus der Haushalt-Uebersicht"
date: 2026-07-28
mode: quick
status: planned
---

# Quick Task 260728-cej: Task verschwindet nach dem Bearbeiten aus der Übersicht

## Problem

Wird ein Task in der Haushalt-Übersicht bearbeitet und gespeichert, ist er danach
nicht mehr an seiner Stelle in der Liste. Drei unabhängige Defekte greifen ineinander:

1. **Backend liefert einen Task ohne Kategorie zurück.**
   `task_service::update_task` liest mit `TaskRow` (`SELECT * FROM tasks`, ohne JOIN auf
   `task_categories`) und gibt `task.to_shared()` zurück — `category_name` und
   `category_color` sind darin immer `None`. `create_task` baut die Antwort ebenso von
   Hand mit `category_name: None`. `archive_task`, `unarchive_task`, `pause_task` und
   `unpause_task` machen es bereits richtig und geben `get_task(...)` zurück.

2. **Die Übersicht übernimmt genau diese verstümmelte Antwort.**
   `HouseholdPage::on_task_save` ersetzt nur `t[pos].task = saved_task`. Der Task verliert
   dadurch seine Kategorie und rutscht in `group_tasks_by_category` aus seiner
   Kategoriegruppe ans Listenende unter „Sonstige". Zusätzlich bleibt `next_due_date` im
   `TaskWithStatus`-Wrapper veraltet: Eine geänderte Wiederholung wirkt sich nicht auf die
   Tagesgruppe aus.

3. **Der Aufklappzustand der Gruppen überlebt kein Re-Render.**
   `GroupedTaskList` rendert `<details open=starts_expanded>`; nur „Heute" startet offen.
   Jede Listenaktualisierung baut den DOM neu, wodurch eine vom Nutzer geöffnete Gruppe
   („Morgen", Wochentag, „Später", „Ohne Termin") wieder zuklappt — ein dort bearbeiteter
   Task ist danach unsichtbar.

## Tasks

### Task 1 — Backend: Update/Create liefern den Task inkl. Kategorie

**files:** `backend/src/services/tasks.rs`

**action:**
- `update_task`: statt `Ok(task.to_shared())` den frisch geladenen Task über
  `get_task(pool, task_id)` zurückgeben (gleiches Muster wie `archive_task`/`pause_task`).
- `create_task`: die Antwort ebenfalls über `get_task` liefern, statt die `shared::Task`
  von Hand mit `category_name: None` zusammenzusetzen.

**verify:** `cargo test -p backend`

**done:** Neue Tests `test_update_task_returns_category` und `test_create_task_returns_category`
belegen `category_name`/`category_color` in beiden Antworten; ein Task ohne Kategorie liefert
weiterhin `None`.

### Task 2 — Frontend: Übersicht lädt nach dem Speichern neu

**files:** `frontend/src/pages/household.rs`

**action:** `on_task_save` lädt die Liste über `ApiClient::get_all_tasks_with_status`
neu, statt nur `t[pos].task` zu ersetzen — konsistent mit `on_save_date` und
`on_context_pause` auf derselben Seite. Damit sind Kategorie *und* `next_due_date`
nach dem Speichern korrekt.

**verify:** `cargo check -p frontend --target wasm32-unknown-unknown` bzw. Workspace-Check

**done:** Nach dem Speichern zeigt die Übersicht den Task mit aktueller Kategorie und
korrekter Tagesgruppe.

### Task 3 — Frontend: Aufklappzustand der Gruppen überlebt Re-Render

**files:** `frontend/src/components/task_card.rs`, `frontend/src/pages/household.rs`,
`frontend/src/pages/dashboard.rs`

**action:**
- `DueDateGroup::state_key()` liefert einen stabilen Schlüssel je Tagesgruppe
  (`today`, `tomorrow`, `weekday-N`, `later-YYYY-MM-DD`, `no-schedule`);
  `category_state_key(date_key, category)` denselben für Kategorie-Untergruppen.
- `GroupedTaskList` bekommt einen optionalen Prop
  `group_state: RwSignal<HashMap<String, bool>>`. Beim Rendern wird der Zustand
  ungetrackt gelesen (`unwrap_or(starts_expanded)`), beim `toggle`-Event geschrieben.
  Ungetracktes Lesen verhindert eine Render-Schleife.
- `HouseholdPage` und `DashboardPage` halten das Signal auf Seitenebene, sodass es
  Re-Renders der Liste überlebt.

**verify:** `cargo test -p frontend`

**done:** Tests decken `state_key` über alle `DueDateGroup`-Varianten,
`category_state_key` und die Auflöselogik (`group_open_state`) inkl. Default-Fallback ab.

## must_haves

**truths:**
- Die Update-Antwort des Backends trägt Kategoriename und -farbe.
- Die Übersicht zeigt einen bearbeiteten Task unverändert an seiner Stelle.
- Eine manuell geöffnete Tagesgruppe bleibt nach dem Speichern geöffnet.

**artifacts:**
- `backend/src/services/tasks.rs` (Fix + Tests)
- `frontend/src/pages/household.rs` (Reload nach Save)
- `frontend/src/components/task_card.rs` (Gruppenzustand + Tests)

**key_links:**
- `backend/src/services/tasks.rs:443` `update_task`
- `frontend/src/pages/household.rs:304` `on_task_save`
- `frontend/src/components/task_card.rs:585` `GroupedTaskList`
