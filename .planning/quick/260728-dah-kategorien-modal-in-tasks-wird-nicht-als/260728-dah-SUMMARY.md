---
quick_id: 260728-dah
description: "Kategorien-Modal in Tasks wird nicht als Overlay dargestellt"
date: 2026-07-28
status: complete
---

# Quick Task 260728-dah — Zusammenfassung

Gemeldetes Symptom: Das Kategorien-Modal auf der Tasks-Seite wird nicht als Overlay
angezeigt, sondern einfach unten eingeblendet.

## Ursache

`frontend/src/components/category_modal.rs` war das einzige Modal im Projekt, dessen
CSS-Klassen in `frontend/styles.css` überhaupt nicht existieren:

- `modal-overlay` statt `modal-backdrop` — `.modal-backdrop` liefert `position: fixed`,
  den abdunkelnden Hintergrund, die Flex-Zentrierung und `z-index: 200`. Ohne eine
  passende Regel blieb der Container ein gewöhnliches Block-Element im Dokumentfluss
  und landete darum unterhalb des Seiteninhalts.
- `form-control` statt `form-input` — die beiden Eingabefelder (Name, Farbe) hatten
  dadurch weder Rahmen noch Höhe noch Fokus-Stil.

Zusätzlich wich die Kopfzeile mit `<h2>` von der Konvention `<h3 class="modal-title">`
ab, die `modal.rs` und alle übrigen Modals verwenden.

## Was geändert wurde

### 1. Kaputte Klassen korrigiert (`frontend/src/components/category_modal.rs`)

`modal-overlay` → `modal-backdrop`, beide `form-control` → `form-input`,
`<h2>` → `<h3 class="modal-title">`. Reine Markup-Korrektur, keine Logikänderung.

### 2. Regressionstest (`frontend/src/components/css_contract.rs`, neu)

Der Fehler konnte deshalb unbemerkt einziehen, weil Leptos Klassennamen nicht prüft
und die vorhandenen `wasm_bindgen_test`-Blöcke von `cargo test` gar nicht ausgeführt
werden — sie testen ohnehin nur Tautologien der Form
`assert_eq!("modal-backdrop", "modal-backdrop")`.

Das neue Modul läuft nativ und liest zur Testzeit `styles.css` sowie alle Quellen
unter `src/components/` und `src/pages/`. Es prüft, dass jede in `class="…"`
verwendete Klasse mit Präfix `modal-` oder `form-` im Stylesheet als Selektor
definiert ist, dass `modal-overlay` nirgends mehr vorkommt, dass das Kategorien-Modal
die geteilte Modal-Struktur verwendet und dass `.modal-backdrop` weiterhin
`position: fixed` samt `z-index` trägt.

Gegenprobe durchgeführt: Setzt man `modal-backdrop` versuchsweise wieder auf
`modal-overlay`, schlagen 3 der 5 Tests fehl.

Zwei bestehende, bewusst ungestylte Klassen stehen in einer Allowlist —
`modal-body` (semantischer Container, Padding kommt von `.modal`) und `modal-sm`.
Ein eigener Test schlägt an, sobald eine davon doch eine Regel bekommt, damit die
Allowlist nicht stillschweigend veraltet.

## Prüfung

- `cargo test --workspace`: grün, 166 Frontend-Tests (vorher 161).
- `cargo clippy --workspace`: keine Warnungen.

## Offen / nicht angefasst

- `modal-sm` (in `set_date_modal.rs`) hat nirgends eine CSS-Regel und ist damit
  wirkungslos. Eine Definition würde die Breite dieses Modals ändern, ohne dass
  bekannt ist, welche beabsichtigt war — bewusst unverändert gelassen.
- Die tautologischen `assert_eq!("x", "x")`-Blöcke in `loading.rs`, `modal.rs`,
  `text_input.rs` u. a. lassen `cargo clippy --workspace --all-targets` fehlschlagen
  (`identical args used in this assert_eq! macro call`). Bestand von vorher, durch
  diesen Task weder verursacht noch behoben.
