---
quick_id: 260811-fpr
description: Statistiken für beliebige Wochen/Monate erstellen und Zeiträume nachberechnen
date: 2026-08-11
status: complete
---

# Quick Task 260811-fpr — Zusammenfassung

## Was geändert wurde

**shared/src/types.rs**
- `RecalculateStatisticsResponse` (Anzahl berechneter Perioden + erste/letzte Periode).
- `week_start_for(date, week_start_day)` und `month_start_for(date)` — geteilte Periodenausrichtung,
  damit Frontend und Backend dieselbe Woche meinen, wenn ein freies Datum gewählt wird.
  `backend::services::statistics::get_week_start`/`get_month_start` delegieren jetzt dorthin.

**backend/src/services/statistics.rs**
- `weekly_period_starts` / `monthly_period_starts`: reine Funktionen, die alle Perioden eines
  Zeitraums auflisten. Randperioden werden voll mitgenommen (die Woche, die `from` enthält, und
  die, die `to` enthält).
- `recalculate_weekly_range` / `recalculate_monthly_range` laufen darüber und rufen die
  bestehende Einzelberechnung auf.
- Fehlervarianten `InvalidRange` und `RangeTooLarge`; `MAX_RECALCULATION_PERIODS = 260`.

**backend/src/handlers/statistics.rs**
- `POST /households/{id}/statistics/weekly/recalculate?from=&to=`
- `POST /households/{id}/statistics/monthly/recalculate?from=&to=`
- Hilfsfunktionen `require_member`, `parse_range`, `range_error_response`,
  `recalculation_response` — die neuen Handler wiederholen den Auth-/Parse-Block nicht.

**frontend**
- `ApiClient::recalculate_weekly_statistics` / `recalculate_monthly_statistics`.
- Statistik-Seite: `<input type="date">` als primäre Periodenauswahl. Die Eingabe wird auf den
  Wochenstart (gemäß `week_start_day`) bzw. Monatsersten normalisiert, sodass jede beliebige
  Periode wählbar ist — auch eine, für die noch nie etwas berechnet wurde. Das bisherige Dropdown
  bleibt als Schnellwahl über bereits berechnete Perioden und zeigt einen Platzhalter, wenn die
  aktuelle Auswahl nicht darin vorkommt.
- Vorbelegung ist jetzt immer die laufende Periode (in der Haushalts-Zeitzone), nicht mehr die
  neueste vorhandene Statistik.
- Aufklappbereich „Zeitraum nachberechnen" mit Von/Bis (vorbelegt: letzte ~3 Monate bis heute)
  und Ergebnismeldung; danach werden Verfügbar-Listen und Anzeige neu geladen.
- Leerzustand einer Periode zeigt zusätzlich den „Berechnen"-Hinweis (`NoMemberData`).
- Neue Übersetzungsschlüssel in de.json und en.json.

**backend/src/test_utils.rs**
- Test-Schema um `weekly_statistics`, `weekly_statistics_tasks`, `monthly_statistics`,
  `monthly_statistics_tasks` ergänzt.
- Fixture `insert_period_result`.

## Tests

13 neue Tests, gesamte Suite grün (663 Tests):
- `shared`: Wochenausrichtung für Montag/Sonntag/Samstag, Idempotenz, Monatsanfang.
- `services::statistics::tests`: Periodenlisten inkl. Teilwochen, Sonntagsstart, Einzeltag,
  Jahreswechsel, invertierter Bereich, exakt am Limit, über dem Limit.
- `services::statistics::range_recalculation_tests`: Nachberechnung erzeugt Zeilen für Wochen ohne
  vorherige Statistik, Monatsbereich deckt jeden Monat ab, invertierter Bereich wird abgelehnt.

`cargo clippy --workspace` sauber. `cargo check -p frontend --target wasm32-unknown-unknown` sauber.
(Die `--all-targets`-Clippy-Fehler in `components/loading.rs` und `components/modal.rs` sind
vorbestehend und von dieser Änderung nicht berührt.)

## Gegen einen laufenden Server verifiziert

Frischer Haushalt, Aufgabe zugewiesen, Perioden-Ergebnisse für März 2026 direkt in die DB gelegt:

| Prüfung | Ergebnis |
| --- | --- |
| Verfügbare Wochen vorher | `[]` |
| Woche 2026-03-02 direkt berechnen (nie im Dropdown gewesen) | 5/7, 71,4 % |
| Nachberechnen 2026-03-01..2026-03-20 | 4 Perioden, 2026-02-23 bis 2026-03-16 |
| Verfügbare Wochen danach | alle 4 |
| Monate 2026-01-15..2026-04-05 | 4 Perioden, Januar bis April |
| Wochenstart auf Sonntag umgestellt | neue Perioden fallen auf Sonntage |
| Invertierter Bereich | 400 `invalid_range` |
| 1879 Perioden angefragt | 400 `range_too_large` |
| Kaputtes Datum / kein Token | 400 / 401 |

## Offen

Die Statistik-Seite wurde nicht im Browser sichtgeprüft — nur kompiliert (nativ und wasm32) und
das Backend end-to-end geprüft.
