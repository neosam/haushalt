---
quick_id: 260727-hke
description: "Zähler-Buttons: optimistisch zählen, Request debouncen"
date: 2026-07-27
status: complete
commit: 4b88a78d
area: frontend
---

# Quick 260727-hke — Summary

## Was kaputt war

Der als Debounce bezeichnete Code (`task_card.rs`, aus Commit `5c2869d`) war eine Verzögerung mit
Sperre. Vier Probleme:

1. Jeder Klick wurde eine Sekunde aufgeschoben, statt sofort zu wirken.
2. Klicks während der Sperre verfielen — bei Soll 5 fünfmal getippt ergab **eine** Erledigung.
3. `−` hatte keine Sperre: Doppelklick = zwei Uncomplete-Requests.
4. Feuerte der Timeout nach dem Entsorgen der Komponente, lief der Callback auf totem Scope.

## Was jetzt passiert

| | vorher | jetzt |
|---|--------|-------|
| Reaktion auf den Tap | nach 1 s | sofort |
| 5 schnelle Taps | 1 Erledigung | 5 Erledigungen |
| Requests dafür | 1 (4 verfallen) | 5, gebündelt nach 500 ms Ruhe |
| Neuladen der Liste | pro Request | einmal am Ende |
| `−` doppelt geklickt | 2 Requests sofort | 1 gebündelter Aufruf |
| Karte verlassen mit offenen Taps | Callback auf totem Scope | Taps werden noch gesendet |

Kern ist `pending_delta`: Taps sammeln sich dort, die Anzeige zeigt `completions + pending_delta`,
und jeder Tap startet den 500-ms-Timer neu. Erst wenn das Tippen aufhört, geht **ein** Aufruf mit
der Gesamtzahl raus.

Die Knöpfe entscheiden auf dem optimistischen Stand: `+` schließt, sobald das Ziel erreicht ist —
auch wenn der Server noch nichts davon weiß. Ohne das bliebe `+` durch den ganzen Tap-Schwall
offen und würde das Ziel überschreiten, das der Server dann ablehnt.

## Entscheidung: kein Batch-Endpoint

`ApiClient` kennt nur Einzelaufrufe. Die Seiten rufen sie in einer Schleife auf. Das bleiben n
Requests, löst aber beide spürbaren Probleme (sofortige Reaktion, ein Reload statt n). Ein echter
Batch-Endpoint wäre Backend-Arbeit und gehört nicht in einen Karten-Task. Bricht ein Request ab,
hört die Schleife auf, statt dem Server den Rest hinterherzuwerfen.

## Änderungen

| Datei | Was |
|-------|-----|
| `task_card_model.rs` | `effective_completions`, `can_complete_pending`, `can_undo_pending`, `COUNTER_FLUSH_MS` |
| `task_card.rs` | `pending_delta`, Timer mit Handle, `on_cleanup`, reaktive Anzeige, Callback-Signatur `(String, i32)` |
| `pages/dashboard.rs`, `pages/household.rs` | Handler nehmen die Anzahl, rufen n-mal auf, laden einmal neu |

`flush` musste `Copy` sein, damit beide Handler es halten können — die `task_id` liegt deshalb in
einem `StoredValue` statt als geklonter `String`.

## Tests

10 neue Tests in `task_card_model`:

- Anzeige addiert ausstehende Taps und wird nie negativ
- `+` schließt auf dem *optimistischen* Zähler (der Kern des Umbaus), bleibt darunter offen
- `allow_exceed_target` und Soll 0 halten `+` offen
- Berechtigung schlägt weiterhin alles
- `−` öffnet auf einen einzelnen ausstehenden Tap, bleibt ohne Berechtigung zu

`cargo test --workspace` → 508 grün, 0 failed.
`cargo clippy --workspace --all-targets` → 67 Findings, exakt die dokumentierte Vorbelastung,
keine aus den geänderten Dateien.

## Nicht abgedeckt

Das Timer-Verhalten selbst (500 ms, Neustart, Cleanup) ist nicht getestet — dafür bräuchte es
einen WASM-Testlauf mit Zeitsteuerung. Getestet ist die Entscheidungslogik, die den Umbau
überhaupt erst korrekt macht; das Timing ist geradlinig genug, um es beim Ausprobieren zu sehen.
