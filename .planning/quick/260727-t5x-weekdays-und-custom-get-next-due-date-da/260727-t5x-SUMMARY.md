---
quick_id: 260727-t5x
description: "Weekdays und Custom: get_next_due_date darf den heutigen Termin nicht überspringen"
date: 2026-07-27
status: complete
area: backend
commits:
  - a3ba009e  # test: failing tests für inklusive Semantik (RED)
  - 42e46cfc  # fix(scheduler): heutiger Termin wird nicht mehr übersprungen (GREEN)
  - 3701d89d  # test: report.rs/tasks.rs nachgezogen, complete_task wird wirklich aufgerufen
  - 4d5324f9  # chore: vorbestehende clippy-Funde in tasks.rs
files_modified:
  - backend/src/services/scheduler.rs
  - backend/src/services/report.rs
  - backend/src/services/tasks.rs
---

# Quick 260727-t5x — Summary

## Was kaputt war

`scheduler::get_next_due_date` verspricht "the next due date **on or after** the given date".
Daily, Weekly und Monthly hielten das Versprechen. Weekdays und Custom brachen es:

```rust
// Weekdays: heute war geplant -> heute wird übersprungen
if is_today_scheduled { Some(from_date + Duration::days(7)) }

// Custom: strikt größer -> heute fällt raus
.filter(|d| **d > from_date)
```

`complete_task` speichert die Erledigung mit `due_date = get_next_due_date(&task, today)`.
Wer einen Weekdays-Task an seinem Fälligkeitstag abhakte, bekam die Completion also mit einem
**zukünftigen** due_date in `task_completions` — `process_missed_tasks` prüft exakt
`due_date = gestern`, fand nichts und verhängte Strafe samt Punktabzug. Der Nutzer sah einen
Haken und wurde bestraft.

Zweiter, bislang unbemerkter Bug: Beim Custom-Zweig lieferte `> from_date` am **letzten**
geplanten Datum gar nichts. `get_next_due_date` gab `None` zurück, und `complete_task` machte
daraus `TaskError::NotDueToday` — ein Custom-Task ließ sich an seinem letzten Termin überhaupt
nicht abhaken.

## Was jetzt gilt

"On or after" wörtlich: ist `from_date` selbst ein geplanter Termin, kommt `from_date` zurück.
Nur ein NICHT geplanter `from_date` wandert nach vorn — und genau das trägt das frühe Abhaken.

In allen sechs Recurrence-Zweigen gilt jetzt dieselbe Invariante:

> `is_task_due_on_date(task, d) == true` ⟹ `get_next_due_date(task, d) == Some(d)`

| Fall | vorher | jetzt |
|---|---|---|
| Weekdays Mo/Mi/Fr, abgehakt am Montag | Mo + 7 Tage | **Montag** |
| Weekdays nur Montag, abgehakt am Montag | Mo + 7 Tage | **Montag** |
| Weekdays Mo/Mi/Fr, abgehakt am Dienstag | Mittwoch | Mittwoch (unverändert) |
| Custom [15.01., 20.02., 25.03.], am 15.01. | 20.02. | **15.01.** |
| Custom [15.01.], am 15.01. (letzter Termin) | `None` → NotDueToday | **15.01.** |
| Custom [15.01.], am 20.01. | `None` | `None` (unverändert) |

Der Weekdays-Zweig ist dabei von der alten Zweiteilung (`if geplant { +7 } else { suche }`) auf
eine einzige Schleife ab Offset 0 geschrumpft; der Custom-Zweig auf `>= from_date`.

## Erwartete Nebenwirkung (gewollt)

Die Task-Karte gruppiert nach `next_due_date` (`DueDateGroup::from_date`). Ein Weekdays-Task an
seinem geplanten Tag rutscht damit von "nächste Woche" nach **"heute"** — er *ist* heute fällig.
Kein Frontend-Test behauptet das Gegenteil; es war nichts zu ändern.

## Aufrufer-Prüfung (Task 2 Punkt 5)

Sechs Aufrufstellen im Backend, keine im Frontend. `grep -rn "get_next_due_date"` über
`backend/src` und `frontend/src` liefert genau die Stellen aus dem Plan — nichts Zusätzliches.

| Stelle | Befund | Änderung |
|---|---|---|
| `report.rs:165` (`is_completed_for_today`) | leitet die Periode aus `get_next_due_date` ab; der Aufrufer hat `is_task_due_on_date` schon geprüft, also liefert der Aufruf jetzt `today`. Bleibt stehen: fängt zusätzlich OneTime (`None` → `today`) ab und hält die Periodenherleitung an einer Stelle. | nur Kommentar |
| `tasks.rs:146` (`get_task_with_status`) | zählt in der Periode um `next_due_date` — am Termintag jetzt *heute*, also dieselbe Periode, in die `complete_task` schreibt. Konsistent. | keine |
| `tasks.rs:268` (`calculate_task_statistics`) | reicht `next_due` nur zur Anzeige durch. | keine |
| `tasks.rs:752/755` (`complete_task`, Vorprüfung) | Die `AlreadyCompleted`-Prüfung greift am Termintag jetzt für heute statt für morgen: bei erreichtem Tagesziel lässt sich nicht mehr zusätzlich "für morgen vorgearbeitet". Genau das tut Daily längst — **vom Nutzer akzeptiert**. Der `next_due.is_none()`-Zweig bei Custom feuert seltener, weil der letzte Termin kein `None` mehr ist — gewollt (zweiter Bug). | keine |
| `tasks.rs:782` (`complete_task`, Schreibpfad) | die eigentlich geheilte Stelle: `completion_due_date` trägt jetzt das Kalenderdatum der Tat. | keine (durch Task 1 geheilt) |
| `tasks.rs:966` (`uncomplete_task`) | löscht aus derselben Periode, in die `complete_task` schreibt. Bleibt paarig. | keine |
| `background_jobs::process_missed_tasks` | ruft `get_next_due_date` gar nicht auf, sondern prüft `due_date = yesterday_local` direkt (Zeile 265). Dass diese Prüfung jetzt trifft, ist der Zweck der Übung. Auch die Periodenfinalisierung (Zeile 589/603) rechnet direkt auf `yesterday_local`. | keine |

## Migrationsanalyse — Befund und Empfehlung (Task 3 Punkt 1)

**Es wurde keine Migration geschrieben und kein UPDATE abgesetzt.** Die Entscheidung gehört dem
Nutzer. Hier der Befund.

### Was falsch in der Datenbank steht

Altbestand: Completions von Weekdays-/Custom-Tasks, die am Termintag abgehakt und mit dem
**Folgetermin** gespeichert wurden. Read-only-Diagnose als Startpunkt — sie findet Kandidaten,
keine Diagnose:

```sql
SELECT t.title, t.recurrence_type, tc.completed_at, tc.due_date
FROM task_completions tc
JOIN tasks t ON t.id = tc.task_id
WHERE t.recurrence_type IN ('weekdays', 'custom')
  AND tc.due_date > date(tc.completed_at);
```

Die Abfrage ist gegen das Schema geprüft (`task_completions` hat `completed_at` und `due_date`;
`RecurrenceType` serialisiert als `'weekdays'` / `'custom'`). Sie wurde **nicht ausgeführt** —
im Repo liegt keine Datenbankdatei, und ein Lauf gegen Produktivdaten steht dem Nutzer zu.

### Warum diese Abfrage allein nicht reicht

Sie kann falsch Verschobenes nicht von legitim früh Abgehaktem unterscheiden — beides sieht
identisch aus, nämlich `due_date` in der Zukunft. Der Unterschied liegt in einer Frage, die SQL
nicht beantwortet: **War `date(tc.completed_at)` selbst ein geplanter Tag?**

- Für Weekdays ginge das notdürftig über `strftime('%w', ...)` gegen das JSON in
  `recurrence_value`.
- Für Custom geht es überhaupt nicht, ohne die Terminliste aus `recurrence_value` auszuwerten.
- Dazu kommt: `completed_at` ist UTC, über den Kalendertag entscheidet aber die Zeitzone des
  Haushalts.

### Warum eine Korrektur teurer ist, als sie aussieht

Der sichtbare Schaden steht nicht in `task_completions`, sondern in dem, was daraus gefolgert
wurde: `missed_task_penalties` und `task_period_results`, samt bereits abgezogener Punkte.
`due_date` allein umzuschreiben heilt nichts — es müssten Strafen zurückgenommen und Perioden
neu finalisiert werden.

### Was ohne Migration passiert

Die Altfälle bleiben stehen. Konkret spürbar: Ein Task, der gestern abgehakt und dessen
Completion auf den Folgetermin geschrieben wurde, erscheint an jenem Termintag als bereits
erledigt, obwohl an dem Tag nichts getan wurde (`get_task_with_status` zählt in der Periode um
`next_due_date`). Ab jetzt entsteht das nicht mehr; der Bestand wächst also nicht weiter.

### Empfehlung

**Keine automatische Migration.** Der Bestand ist historisch und wächst nicht mehr; eine korrekte
Rückabwicklung bräuchte Zeitzonen-Auflösung, Punkte-Rücknahme und Neuberechnung der Perioden —
deutlich mehr Risiko als Nutzen. Die Diagnoseabfrage oben ist dem Nutzer an die Hand gegeben,
damit er die Menge selbst sehen und entscheiden kann.

## Tests

Der Weg war TDD: erst die Erwartungen umgeschrieben (8 rote scheduler-Tests), dann der Fix.

**scheduler.rs** — 36 Tests grün (vorher 32).

Vier bestehende Tests standen auf dem alten Verhalten und wurden samt ihrer erklärenden
Kommentare korrigiert: `test_get_next_due_date_weekdays`,
`test_get_next_due_date_weekdays_on_scheduled_day`, `test_get_next_due_date_custom`,
`test_get_next_due_date_custom_on_scheduled_date`.

Vier neu:

| Test | Nagelt fest |
|---|---|
| `..._weekdays_single_scheduled_day_returns_today` | Guard gegen ein übrig gebliebenes "+7" |
| `..._custom_on_last_scheduled_date_returns_that_date` | Regression für den zweiten Bug (vorher `None`) |
| `..._agrees_with_is_task_due_on_date_weekdays` | Invariante über eine ganze Woche |
| `..._agrees_with_is_task_due_on_date_custom` | Invariante über die Kalenderspanne der Termine |

Fünf Tests wurden bewusst **nicht** angefasst und sind unverändert grün geblieben — sie belegen,
dass frühes Abhaken heil ist: `..._weekdays_early_completion`, `..._weekdays_no_match_in_week`,
`..._custom_early_completion`, `..._custom_all_past`, `..._custom_last_date_passed`.

**report.rs** — 47 Tests grün (vorher 46).
`test_due_today_marks_weekdays_task_done_via_period_bounds` schlug nach dem Fix fehl (der fixierte
Testtag 2027-01-04 ist ein Montag, die Vorgabe Mo–Fr) und heißt jetzt
`test_due_today_marks_weekdays_task_done_on_its_scheduled_day`, mit der Completion auf
`pinned_today()`. Daneben der Gegentest
`test_due_today_weekdays_completion_on_later_occurrence_is_not_done`: eine Completion auf
2027-01-11 markiert heute **nicht** als done. Der wäre vor dem Fix grün gewesen und ist es jetzt
aus dem umgekehrten Grund.

**tasks.rs** — 74 Tests grün (vorher 74; drei ersetzt, keine Netto-Änderung).

Die drei alten Tests (`test_complete_weekday_task_early`,
`test_complete_weekday_task_on_scheduled_day`, `test_complete_custom_task_early`) riefen
`complete_task` **nie** auf — sie schrieben die Completion selbst per `INSERT` und behaupteten
dann, das Eingefügte sei das Eingefügte. Sie konnten den Bug nicht sehen. An ihre Stelle traten:

- `test_complete_weekdays_task_on_scheduled_day_stores_today`
- `test_complete_custom_task_on_scheduled_date_stores_today`
- `test_complete_weekdays_task_early_stores_next_occurrence`

Alle drei gehen über `complete_task(&pool, &task.id, &user_id, &household_id)` und lesen danach
`SELECT due_date FROM task_completions WHERE task_id = ?`.

`complete_task` liest `Utc::now().date_naive()`, der Testtag lässt sich nicht setzen — deshalb
wird der Task **um den Testtag herum** gebaut. Die Gleichheit von `scheduler::weekday_to_u8`
(privat) und `Weekday::num_days_from_sunday()` (So = 0 … Sa = 6) steckt in einem einzigen Helfer
`scheduler_weekday_of`, damit sie nicht still auseinanderläuft.

**Gegenprobe, dass diese Tests den Bug wirklich gesehen hätten:** Der Fix wurde temporär
zurückgedreht und die Tests noch einmal ausgeführt.

```
test ..._complete_custom_task_on_scheduled_date_stores_today ... FAILED
test ..._complete_weekdays_task_on_scheduled_day_stores_today ... FAILED
test ..._complete_weekdays_task_early_stores_next_occurrence ... ok   <- Kontrolle
```

Beide Bug-Tests rot, der Kontrolltest für frühes Abhaken grün. Der Fix wurde danach vollständig
wiederhergestellt (`jj diff` zeigt `scheduler.rs` seither unverändert gegenüber dem Fix-Commit).

## Qualitätsgates — echte Ausgabe

| Gate | Ergebnis |
|---|---|
| `cargo test -p backend services::scheduler` | `ok. 36 passed; 0 failed` |
| `cargo test -p backend services::report` | `ok. 47 passed; 0 failed` |
| `cargo test -p backend services::tasks` | `ok. 74 passed; 0 failed; 1 ignored` |
| `cargo test --workspace` | `ok. 297 / 148 / 67 / 1 passed; 0 failed; 1 ignored` |
| `cargo check --workspace` | `Finished dev profile` — Exit 0, keine Warnung |
| `cargo clippy -p backend --all-targets` | `Finished dev profile` — **Exit 0** |

Kein Schema geändert, also war kein `cargo sqlx prepare` nötig.

**Außerhalb des Scopes und weiter rot:** `cargo clippy -p frontend --all-targets` meldet
unverändert 61 vorbestehende Funde (siehe `deferred-items.md`). Dieser Plan hat kein Frontend-File
angefasst; `cargo clippy --workspace` bleibt dadurch rot. Das Backend-Gate war die verbindliche
Schranke und ist grün.

## Nebenbei erledigt

Die vier vorbestehenden clippy-Funde in `tasks.rs` sind weg (`assign_op_pattern` ×2,
`bool_assert_comparison` ×2). Sie standen in
`.planning/phases/02.1-daily-task-report-inserted-urgent/deferred-items.md` als "fremde Datei,
außerhalb des Scopes" — dieser Plan ändert `tasks.rs` aber selbst, damit trug die Ausrede nicht
mehr. Von den ursprünglich 6 gemeldeten Funden verschwanden zwei ohnehin mit den in Task 2
ersetzten Tests.

**Hinweis für den Nutzer:** Punkt 1 in `deferred-items.md` (backend clippy in `tasks.rs`) ist damit
erledigt und kann dort abgehakt werden. Punkt 2 (61 Frontend-Funde) steht unverändert.

## Abweichungen vom Plan

1. **[Rule 3 – blockierend]** Der Plan gibt als Verifikation
   `cargo test -p backend services::report services::tasks` an. `cargo test` nimmt nur **einen**
   TESTNAME-Filter und bricht mit `error: unexpected argument` ab. Die beiden Filter wurden
   deshalb getrennt ausgeführt; Abdeckung identisch.
2. **[zusätzlich, nicht im Plan]** Die oben beschriebene Gegenprobe mit temporär
   zurückgedrehtem Fix. Der Plan verlangt als Erfolgskriterium "der Bug wäre mit diesen Tests
   aufgefallen" — statt das zu behaupten, wurde es gemessen.

Sonst wurde der Plan exakt so ausgeführt wie geschrieben.

## Self-Check

- `backend/src/services/scheduler.rs` — vorhanden, enthält `filter(|d| **d >= from_date)`
- `backend/src/services/report.rs` — vorhanden, ohne "KNOWN LIMITATION"
- `backend/src/services/tasks.rs` — vorhanden, drei Tests rufen `complete_task` auf
- Commits `a3ba009e`, `42e46cfc`, `3701d89d`, `4d5324f9` liegen im jj-Log
- Working Copy enthält nur noch `.planning/`-Artefakte, keine unkommittierten Codeänderungen

## Self-Check: PASSED
