---
quick_id: 260728-c7k
description: "Kategorie-Farbe in der Task-Liste anzeigen; Kategorie- und Tages-Gruppen klappbar machen"
date: 2026-07-28
status: complete
commit: 4ed8223d
---

# Quick Task 260728-c7k — Zusammenfassung

## Was war das Problem

Die Kategoriefarbe wurde im `CategoryModal` gesetzt und in `task_categories.color`
gespeichert, kam aber nie in der Task-Liste an: `shared::Task` transportierte nur
`category_name`. Der Kategorie-Header in `GroupedTaskList` nutzte darum fest
`var(--text-muted)` auf `var(--bg-secondary)` — die angezeigte Farbe hatte
nichts mit der eingestellten zu tun.

Zusätzlich zeigte die Liste immer alle Tage und alle Kategorien vollständig an.

## Was gemacht wurde

### Task 1 — Farbe durchreichen
- `shared::Task.category_color: Option<String>` ergänzt (16 weitere `Task {}`-Literale
  im Workspace um `category_color: None` erweitert).
- `TaskRowWithCategory.category_color` + Mapping in `to_shared`.
- Alle fünf Kategorie-Joins in `services/tasks.rs` selektieren jetzt zusätzlich
  `tc.color as category_color`. Die Queries nutzen `sqlx::query_as` (runtime-checked),
  also war kein `cargo sqlx prepare` nötig.
- Beim Testen fiel auf, dass das lokale Test-Schema in `background_jobs.rs`
  bei `task_categories` die `color`-Spalte fehlte (die echte Migration hat sie) —
  ergänzt, sonst schlug `test_process_auto_archive_completed_onetime_task` fehl.

### Task 2 — Kategorie-Gruppe mit Farbe und Collapse
- `group_tasks_by_category` gibt statt `(String, Vec<_>)`-Tupeln jetzt
  `Vec<CategoryGroup>` mit `name` / `color` / `tasks` zurück. Die Farbe kommt vom
  ersten Task der Gruppe, der eine hat.
- Der redundante `sort_by` nach dem `BTreeMap` ist entfallen — die Map liefert
  die Kategorien bereits alphabetisch.
- Kategorie-Header: `border-left: 4px solid {color}`, Fallback `var(--border-color)`.
- Gruppe ist ein `<details open=true>` mit `<summary>`-Header, Chevron und Task-Zähler.

### Task 3 — Tages-Gruppen klappbar
- `DueDateGroup::is_collapsible()` / `starts_expanded()` — nur `Today` ist beides nicht
  bzw. ist als einziges standardmäßig offen.
- Tage außer heute sind `<details>` ohne `open`, "Heute" bleibt ein normales `<div>`
  ohne Toggle.
- Chevron-Rotation und Marker-Unterdrückung in `styles.css` unter
  `.collapsible-group`.

## Entscheidungen

- `<details>/<summary>` statt Signalen: nativ tastaturbedienbar, kein Reaktivitätsproblem
  mit den einmalig aus bewegten Daten gebauten Task-Views, und der Collapse-Zustand
  ist automatisch nur pro Seitenaufruf gültig (wie gewünscht). Entspricht auch dem
  bestehenden `Accordion`-Component.
- Task-Zähler im Header ergänzt (über den Plan hinaus): eine standardmäßig zugeklappte
  Gruppe gäbe sonst keinerlei Hinweis darauf, was sich darin verbirgt.

## Tests

- Backend (`services/tasks.rs`): `test_list_tasks_carries_category_color`,
  `test_get_task_carries_category_color`, `test_task_without_category_has_no_color`.
- Frontend (`components/task_card.rs`): 5 Tests zu `group_tasks_by_category`
  (Farbe, Fallback, Farbe vom ersten Task mit Farbe, Sonstige zuletzt, Sortierung)
  und 2 Tests zu `is_collapsible`/`starts_expanded`.
- Die neuen Frontend-Tests laufen als `#[test]` (nicht `#[wasm_bindgen_test]`),
  weil sie reine Logik testen und `wasm_bindgen_test` unter `cargo test` gar nicht
  ausgeführt wird — dieselbe Konvention wie `task_card_model.rs`.

`cargo test --workspace`: 300 Backend + 156 Frontend, alle grün.
`cargo check --workspace` und `cargo clippy --workspace` warnungsfrei.

## Offen

Nicht visuell im Browser verifiziert — die Prüfung erfolgte über Build, Clippy und Tests.
