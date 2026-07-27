---
quick_id: 260727-hke
description: "Zähler-Buttons: optimistisch zählen, Request debouncen"
date: 2026-07-27
area: frontend
---

# Quick 260727-hke: Zähler-Buttons reagieren sofort

## Problem

Was in `task_card.rs` als Debounce bezeichnet war, war eine Verzögerung mit Sperre:

```rust
if can_complete && !is_debouncing.get() {
    is_debouncing.set(true);
    set_timeout(move || { on_complete.call(task_id_clone); is_debouncing.set(false); },
                Duration::from_secs(1));
}
```

1. **Jeder Klick wurde eine Sekunde aufgeschoben**, statt sofort zu wirken.
2. **Klicks während der Sperre gingen verloren.** Bei Soll 5 fünfmal getippt = eine Erledigung.
   Man musste fünf Sekunden lang im Sekundentakt tippen.
3. **`−` hatte gar keine Sperre** — Doppelklick feuerte zwei Uncomplete-Requests.
4. Feuerte der Timeout nach dem Entsorgen der Komponente, lief der Callback auf totem Scope.

Ursprünglich (Commit `5c2869d`, „UX enhancements") war es anders gedacht — bestätigt vom Nutzer
am 2026-07-27: optimistisch zählen, Request debouncen.

## Lösung

Richtung umdrehen: Der Zähler folgt dem Tippen, das Netzwerk folgt der Ruhe.

- `pending_delta` sammelt Taps; die Anzeige ist `completions + pending_delta`
- Jeder Tap startet den Timer neu; nach 500 ms Ruhe geht **ein** Aufruf mit der Gesamtzahl raus
- Callback-Signatur `Callback<(String, i32)>` statt `Callback<String>`
- `+` und `−` schließen auf dem optimistischen Stand, nicht auf dem Serverstand
- `on_cleanup` sendet ausstehende Taps beim Verlassen, statt sie zu verschlucken

## Tasks

1. `task_card_model.rs`: `effective_completions`, `can_complete_pending`, `can_undo_pending`,
   `COUNTER_FLUSH_MS` — reine Funktionen, testbar ohne DOM
2. `task_card.rs`: Signals, Timer mit Handle, Cleanup, reaktive Anzeige; Callback-Signatur
3. `pages/dashboard.rs` + `pages/household.rs`: Handler nehmen die Anzahl entgegen, rufen die API
   n-mal sequenziell auf und laden die Liste **einmal** neu
4. Qualitätsgates

## Entscheidung: kein Batch-Endpoint

`ApiClient` kennt nur `complete_task`/`uncomplete_task` für je eine Erledigung. Die Handler rufen
sie deshalb in einer Schleife auf. Das kostet weiterhin n Requests, löst aber beide spürbaren
Probleme: das Tippen fühlt sich sofort an, und die Liste wird einmal statt n-mal neu geladen.
Ein echter Batch-Endpoint wäre Backend-Arbeit und gehört nicht in einen Karten-Task.

Bricht ein Request ab, hört die Schleife auf, statt dem Server den Rest hinterherzuwerfen.

## must_haves

- Der Zähler bewegt sich beim Tippen, nicht erst nach der Antwort
- Fünf schnelle Taps ergeben fünf Erledigungen, nicht eine
- `+` schließt, sobald das Ziel *optimistisch* erreicht ist
- `−` ist erst verfügbar, wenn es etwas zurückzunehmen gibt
- Kein Tap geht verloren, auch nicht beim Verlassen der Ansicht
