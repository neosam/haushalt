---
quick_id: 260727-fs5
description: "Welle 2: Task-Karte nach Archetypen"
date: 2026-07-27
status: complete
commit: fc0ac402
area: frontend
---

# Quick 260727-fs5 — Summary

Welle 1 hat das Formular nach Archetypen umgebaut, Welle 1c (260727-fcg) den Bonus-Typ ergänzt.
Diese Welle bringt die Archetypen in die *Liste*.

## Der eigentliche Punkt: der tote Knopf ist weg

Für die zuständige Person eines Instandhaltungs-Tasks war der `−`-Knopf dauerhaft `disabled`, mit
der Begründung im `title`-Attribut — auf Touchgeräten also unsichtbar. An seiner Stelle steht jetzt
`CardAction::Locked`: eine Klartextzeile mit Anzahl und Datum des letzten Eintrags.

**Ohne den Namen der eintragenden Person.** Das Mockup schreibt „zuletzt gestern von Anna";
`TaskWithStatus` kennt `last_completion` (wann), aber nicht *wer*. Den Namen zu zeigen hieße, die
API zu erweitern — das gehört nicht in einen Karten-Task. Der Text nennt Anzahl und Datum.

## Entscheidung: Bauform folgt dem Ziel, nicht dem Typ

Das Mockup zeigt Routine mit Zähler und die übrigen Typen mit einem Einzelknopf. Wörtlich
umgesetzt hätte eine *gemeinsame Aufgabe mit Soll 3* ihre Feinsteuerung verloren.

Deshalb: `target_count > 1` → Zähler, sonst Einzelknopf — für jeden Typ. Der Archetyp bestimmt
ausschließlich Beschriftung und Farbe. Eine Routine mit Soll 1 bekommt also „✓ Erledigt", eine
gemeinsame Aufgabe mit Soll 3 behält `−` `1/3` `+`.

Ebenso bleibt das Rückgängigmachen erreichbar: beim Einzelknopf erscheint ein `−`, sobald es etwas
zurückzunehmen gibt *und* der Nutzer darf. Keine Funktion geht verloren, kein Knopf zeigt ins Leere.

## Änderungen

| Datei | Was |
|-------|-----|
| `task_card_model.rs` (neu) | `CardAction`, `ActionStyle`, `card_action()`, `accent_class()`, `type_badge()` — reine Funktionen, testbar ohne DOM |
| `task_card.rs` | Aktionsbereich verzweigt über `card_action`; Locked-Zeile; Typ-Badge; Akzentklasse |
| `styles.css` | `.task-item--*` (6 Akzente), `.task-card-locked`, `.btn-warn`, `.task-action-btn` (44 px), zwei neue Theme-Variablen inkl. Dark Mode |
| `translations/{de,en}.json` | je 12 Keys (6 Knopftexte, 4 Badges, 2 Locked-Varianten) |

Farbwerte stammen aus dem Mockup und laufen über die vorhandenen Theme-Variablen — Dark Mode
funktioniert damit automatisch. Nur `shared` (#0ea5e9) und `bonus` (#a855f7) brauchten eigene
Variablen, die im Dark-Mode-Block wie alle anderen aufgehellt werden.

`.btn-warn` gab es noch nicht — „Rückfall eintragen" ist weder Erfolg (grün) noch Alarm (rot).

## Tests

11 neue Tests in `task_card_model`:

- Locked-Fall greift genau dann, wenn etwas eingetragen ist *und* nicht zurückgenommen werden darf
- `ReadOnly` schlägt alles andere (für jeden der sechs Typen geprüft)
- Soll > 1 behält den Zähler — für jeden Typ
- Bonus (Soll 0) fällt nicht in den Zähler-Zweig
- Rückfall ist `Warn`, gemeldeter Verstoß ist `Danger`
- Akzentklassen und Aktions-Labels sind je Typ eindeutig
- nur die vier besonderen Typen tragen ein Badge

`cargo test --workspace` → 498 grün, 0 failed (backend 292, frontend 138, shared 67, 1 Doc-Test).
`cargo clippy --workspace --all-targets` → 69 Findings, exakt die dokumentierte Vorbelastung,
keine aus den geänderten Dateien. Beide Übersetzungsdateien tragen dieselben 727 Keys.

## Nicht visuell geprüft

Die Chrome-Extension ist in dieser Session nicht verbunden, ein Screenshot war deshalb nicht
möglich. Die Umsetzung folgt den Werten und Abständen des Mockups; ein Blick mit `trunk serve`
lohnt trotzdem, bevor das live geht — besonders auf den Zeilenumbruch der Aktionsleiste auf
schmalen Bildschirmen, weil die Knöpfe jetzt Wörter statt Zeichen tragen.

## Offen

- `report.rs`: Bonusaufgaben erscheinen im Tagesreport weiterhin nie als `(done)` (Nutzerwunsch:
  vorerst ignorieren)
- Der `title`-Tooltip `task_card.cannot_uncomplete` wird nur noch im Zähler-Zweig gebraucht; im
  Einzelknopf-Zweig ersetzt ihn die Locked-Zeile
