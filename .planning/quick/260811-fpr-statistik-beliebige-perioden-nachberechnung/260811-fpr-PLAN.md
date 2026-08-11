---
quick_id: 260811-fpr
description: Statistiken für beliebige Wochen/Monate erstellen und Zeiträume nachberechnen
date: 2026-08-11
status: planned
---

# Quick Task 260811-fpr: Beliebige Statistik-Perioden

## Problem

Die Statistik-Seite (`frontend/src/pages/statistics.rs`) bietet als Periodenauswahl nur ein
`<select>`, das aus `list_available_weeks`/`list_available_months` befüllt wird. Diese liefern
ausschließlich Perioden, für die bereits ein Datensatz in `weekly_statistics`/`monthly_statistics`
existiert. Bei einem Haushalt ohne Statistiken ist das Dropdown leer, `selected_week`/
`selected_month` bleibt `None`, und der „Berechnen"-Button fällt im Backend auf die *aktuelle*
Woche bzw. den aktuellen Monat zurück. Vergangene Perioden lassen sich gar nicht anwählen —
Henne-Ei-Problem.

Das Backend kann bereits beliebige Perioden berechnen (`?week_start=`/`?month=`), es fehlt nur
die Eingabemöglichkeit im UI sowie eine Massen-Nachberechnung.

## Entscheidungen (mit Nutzer geklärt)

1. Wochen-/Monatsraster bleibt — kein freier Von-Bis-Zeitraum als Statistik-Einheit.
2. Nachberechnung eines Zeitraums (alle Wochen bzw. Monate zwischen zwei Daten) wird ergänzt.
3. Aufgaben ohne zugewiesenes Mitglied bleiben weiterhin aus der Statistik ausgeschlossen.

## Tasks

### Task 1 — Backend: Range-Nachberechnung

**Files:** `shared/src/types.rs`, `backend/src/services/statistics.rs`,
`backend/src/handlers/statistics.rs`, `backend/src/test_utils.rs`

**Action:**
- `RecalculateStatisticsResponse { periods_calculated, first_period, last_period }` in `shared`.
- Reine Hilfsfunktionen `weekly_period_starts(from, to, week_start_day)` und
  `monthly_period_starts(from, to)` in `services/statistics.rs`.
- `recalculate_weekly_range` / `recalculate_monthly_range` iterieren darüber und rufen die
  bestehende Einzelberechnung auf.
- Neue Fehlervarianten `InvalidRange` (from > to) und `RangeTooLarge` (> `MAX_RECALCULATION_PERIODS`).
- Routen `POST /statistics/weekly/recalculate?from=&to=` und
  `POST /statistics/monthly/recalculate?from=&to=`, Auth analog zu `calculate` (Mitgliedschaft).
- Test-Schema um `weekly_statistics`, `weekly_statistics_tasks`, `monthly_statistics`,
  `monthly_statistics_tasks` ergänzen.

**Verify:** `cargo test -p backend`, `cargo clippy --workspace`

**Done:** Unit-Tests für Periodenlisten (Wochenstart-Ausrichtung, Monatsgrenzen, Jahreswechsel,
leerer/invertierter Bereich, Limit) und ein DB-Test, der über einen Mehrwochenbereich
nachberechnet und die entstandenen Zeilen prüft.

### Task 2 — Frontend: freie Periodenauswahl + Nachberechnen-UI

**Files:** `frontend/src/api/mod.rs`, `frontend/src/pages/statistics.rs`,
`frontend/src/translations/de.json`, `frontend/src/translations/en.json`

**Action:**
- `ApiClient::recalculate_weekly_statistics` / `recalculate_monthly_statistics`.
- Periodenauswahl: `<input type="date">` als primäre Eingabe (beliebige Periode), Eingabe wird auf
  Wochenstart (`week_start_day` aus den Haushaltseinstellungen) bzw. Monatsersten normalisiert.
  Das bestehende Dropdown bleibt als Schnellwahl über bereits berechnete Perioden erhalten.
- Aufklappbarer Bereich „Zeitraum nachberechnen" mit Von-/Bis-Datum und Button; danach werden die
  Verfügbar-Listen und die aktuelle Ansicht neu geladen.
- Leerzustand: auch bei geladener, aber mitgliederloser Statistik den Hinweis „Berechnen" zeigen.

**Verify:** `cargo check -p frontend --target wasm32-unknown-unknown` bzw. Workspace-Build,
`cargo test --workspace`

**Done:** Beliebige Woche/Monat ist ohne vorhandenen Datensatz wählbar und berechenbar; ein
Zeitraum lässt sich in einem Schritt nachberechnen.

## must_haves

- Beliebige (auch vergangene) Woche/Monat ist im UI wählbar, unabhängig von vorhandenen Daten.
- Backend-Endpunkte berechnen einen ganzen Zeitraum in einem Request.
- Wochenausrichtung folgt `week_start_day` der Haushaltseinstellungen.
- Grenze gegen versehentlich riesige Bereiche.
- Tests für Periodenlisten und Range-Nachberechnung.
