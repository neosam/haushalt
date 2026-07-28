---
quick_id: 260728-dah
description: "Kategorien-Modal in Tasks wird nicht als Overlay dargestellt"
date: 2026-07-28
mode: quick
status: planned
---

# Quick Task 260728-dah: Kategorien-Modal wird unten in den Seitenfluss gerendert

## Problem

Das Kategorien-Modal auf der Tasks-Seite erscheint nicht als zentriertes Overlay über
der Seite, sondern wird schlicht unten an den Seiteninhalt angehängt — ohne
abgedunkelten Hintergrund, ohne Zentrierung, mit ungestylten Eingabefeldern.

`frontend/src/components/category_modal.rs` ist das einzige Modal im Projekt, das
CSS-Klassen verwendet, die es in `frontend/styles.css` überhaupt nicht gibt:

| Verwendet | Existiert in styles.css | Projektkonvention |
|-----------|------------------------|-------------------|
| `modal-overlay` (Zeile 120) | nein | `modal-backdrop` |
| `form-control` (Zeilen 139, 149) | nein | `form-input` |

`.modal-backdrop` trägt `position: fixed`, das abdunkelnde `rgba(0,0,0,0.5)`,
`display: flex` mit Zentrierung und `z-index: 200`. Weil `.modal-overlay` nirgends
definiert ist, bleibt der Container ein gewöhnliches Block-Element im normalen
Dokumentfluss — genau das gemeldete Symptom. Analog bleiben die Eingabefelder ohne
`.form-input` unformatiert (kein Rahmen, keine Höhe, kein Fokus-Stil).

Zusätzlich weicht die Kopfzeile ab: `<h2>` statt `<h3 class="modal-title">`, das alle
übrigen Modals (`modal.rs`, `reward_modal.rs`, `note_modal.rs`, `task_modal.rs`, …)
verwenden.

## Tasks

### Task 1 — Kaputte CSS-Klassen in category_modal.rs korrigieren

**files:** `frontend/src/components/category_modal.rs`

**action:**
- Wurzel-Element: `class="modal-overlay"` → `class="modal-backdrop"`.
- Titelzeile: `<h2>…</h2>` → `<h3 class="modal-title">…</h3>` (Konvention aus `modal.rs`).
- Beide Eingabefelder (Name, Farbe): `class="form-control"` → `class="form-input"`.

**verify:** `cargo check -p frontend` ohne Warnungen; Datei enthält weder
`modal-overlay` noch `form-control`.

**done:** Das Modal nutzt dieselben Wurzel- und Formularklassen wie alle anderen Modals.

### Task 2 — Regressionstest gegen undefinierte Modal-/Formular-Klassen

**files:** `frontend/src/components/css_contract.rs` (neu), `frontend/src/components/mod.rs`

**action:**
- Neues, nur unter `cfg(test)` kompiliertes Modul, das nativ (nicht wasm) läuft — die
  bestehenden `wasm_bindgen_test`-Blöcke werden von `cargo test` nicht ausgeführt und
  hätten diesen Fehler daher nie gefunden.
- Der Test liest `frontend/styles.css` sowie alle Quelldateien unter `src/components/`
  und `src/pages/` über `CARGO_MANIFEST_DIR`, sammelt jede in `class="…"` verwendete
  Klasse mit Präfix `modal-` oder `form-` und stellt sicher, dass sie in `styles.css`
  als Selektor definiert ist.
- Zwei bereits im Bestand vorhandene, bewusst ungestylte Klassen kommen in eine
  dokumentierte Allowlist: `modal-body` (rein semantischer Container, `.modal` liefert
  das Padding) und `modal-sm` (wirkungsloser Größen-Modifier in `set_date_modal.rs`,
  separat zu klären — dieser Task ändert dessen Darstellung nicht).
- Zusätzlicher, expliziter Test: `modal-overlay` kommt in keiner Quelldatei mehr vor.

**verify:** `cargo test -p frontend --lib` grün; Test schlägt fehl, wenn man
`modal-backdrop` in `category_modal.rs` versuchsweise wieder auf `modal-overlay` setzt.

**done:** Jede künftig eingeführte, im Stylesheet unbekannte `modal-*`/`form-*`-Klasse
lässt die Testsuite fehlschlagen.

## Nicht in diesem Task

- `.modal-sm` (in `set_date_modal.rs` verwendet, nirgends definiert) bleibt unverändert.
  Eine Definition würde die Größe dieses Modals ändern, ohne dass bekannt ist, welche
  Breite beabsichtigt war — das ist eine eigene Entscheidung.
