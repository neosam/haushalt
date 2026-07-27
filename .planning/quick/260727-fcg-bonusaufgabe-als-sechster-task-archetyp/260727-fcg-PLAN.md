---
quick_id: 260727-fcg
description: Bonusaufgabe als sechster Task-Archetyp
date: 2026-07-27
area: general
todo: .planning/todos/pending/2026-07-27-bonusaufgabe-als-sechster-archetyp.md
depends_on: 260727-dct
---

# Quick 260727-fcg: Bonusaufgabe als sechster Archetyp

Eine Bonusaufgabe muss nicht erledigt werden (Soll 0), wird aber getrackt, wenn jemand sie macht.
Die Datenlage (`target_count = 0`) existiert bereits — ihr fehlten nur Name und Oberfläche.

## Entscheidungen des Nutzers (2026-07-27, verbindlich)

1. **Punkte:** Es gibt kein Verfehlen bei Bonusaufgaben, aber sehr wohl Punkte.
   → `points_penalty` wird für Bonus ausgeblendet und beim Typwechsel geleert.
   → `points_reward` bleibt unverändert sichtbar und nutzbar.
2. **Streak/Historie:** erledigt = grün, nicht erledigt = neutral, **nie rot**.
3. **Tagesreport:** vorerst ignorieren — `report.rs` bleibt unangetastet.

## Befund, der das Todo korrigiert: Backend-Arbeit ist doch nötig

Das Todo hält fest, Bonusaufgaben zeigten im `PeriodTracker` "vermutlich lauter rote ✗", weil
`is_target_met()` bei `target_count = 0` immer `false` ist. **Das trifft nicht zu** — der
`PeriodTracker` liest `period_results`, und die entstehen in `background_jobs.rs:616` aus:

```rust
} else if completions_count >= task.target_count as i64 {
    PeriodStatus::Completed
```

Bei `target_count = 0` ist das `0 >= 0` → **immer `Completed`**. Ohne Änderung bekäme eine
Bonusaufgabe also lauter *grüne* ✓, auch an Tagen, an denen niemand sie angefasst hat — das
Gegenteil von Entscheidung 2. `is_target_met()` spielt hier gar keine Rolle.

Deshalb enthält dieser Plan einen Backend-Task, den das Todo nicht vorgesehen hatte.

`PeriodStatus::Skipped` ist das passende Ziel für "nicht gemacht": Es rendert neutral
(`period-skipped`, grau "-") und bricht — anders als `Failed` — keinen Streak
(`period_results.rs:254`). Für Bonus ist genau das richtig.

## Weiterer Befund: `Maintenance` mit invertiertem `habit_type` ist Absicht

Beim Lesen sah `Archetype::Maintenance => ArchetypeDefaults { habit_type: HabitType::Bad, ... }`
nach einem Copy-Paste-Fehler aus `BadHabit` aus. Ist es nicht: `types.rs:2522`
(`test_archetype_defaults_invert_points_where_completing_is_the_failure`) hält ausdrücklich fest,
dass bei Maintenance das Abhaken einen *Verstoß* dokumentiert und die Punkte deshalb invertiert
laufen müssen. Nicht anfassen.

## Tasks

### Task 1 — shared: `Archetype::Bonus`

**Files:** `shared/src/types.rs`

**Action:**
- `Archetype::Bonus` als sechste Variante mit Doc-Kommentar.
- `ArchetypeDefaults` um `target_count: i32` erweitern.
  **Abweichung vom Todo:** Das Todo schlägt `Option<i32>` vor, verlangt aber im selben Atemzug,
  dass *alle* Presets den Wert explizit setzen (sonst bliebe beim Zurückwechseln von Bonus die `0`
  stehen und die Ableitung kippte sofort wieder auf Bonus). Wenn ohnehin jede Variante einen
  konkreten Wert trägt, ist `Option` nur eine Fehlerquelle — `i32` erzwingt die Vollständigkeit
  über den Compiler. Bonus bekommt `0`, die übrigen fünf `1`.
- Ableitungsregel in `Task::archetype()` **an Position 3**:
  ```rust
  if self.assignee_cannot_uncomplete      { Maintenance }
  else if self.habit_type.is_inverted()   { BadHabit }
  else if self.target_count <= 0          { Bonus }      // neu
  else if self.anyone_can_complete        { Shared }
  else if recurrence == OneTime           { OneOff }
  else                                    { Routine }
  ```
  `<= 0` statt `== 0` — konsistent mit `background_jobs.rs` und dem Fix aus 260727-dct.
- `ALL_ARCHETYPES` im Test-Modul auf 6 erweitern.

**Verify:** `nix develop -c cargo test -p shared`

**Done:** Round-Trip-Test grün für alle sechs Archetypen; ein BadHabit mit Soll 0 bleibt BadHabit.

### Task 2 — Backend: Perioden-Status für Bonusaufgaben

**Files:** `backend/src/services/background_jobs.rs`

**Action:** In der Perioden-Finalisierung vor dem `completions_count >= target_count`-Zweig:

```rust
} else if task.target_count <= 0 && !task.habit_type.is_inverted() {
    // Kein Soll: gemacht = Erfolg, nicht gemacht = keine Wertung (nie ein Fehlschlag).
    if completions_count > 0 { Completed } else { Skipped }
}
```

**Verify:** `nix develop -c cargo test -p backend period`

**Done:** Bonusaufgabe ohne Erledigung → `Skipped`; mit Erledigung → `Completed`; nie `Failed`.

**Tests:** zwei neue Tests für genau diese beiden Fälle.

### Task 3 — Frontend-Modell: Preset, Sichtbarkeit, Typwechsel

**Files:** `frontend/src/components/task_form_model.rs`

**Action:**
- `ALL_ARCHETYPES` auf 6; Bonus nach `Routine` einsortiert (verwandtester Nachbar).
- `preset(Bonus)`: Icon `🎁`, i18n-Keys, `note: Some((Info, ...))`, `base_is_date: false`.
- `derive_archetype`: Bonus-Regel an Position 3 spiegeln (Soll aus dem Formular als `&str`).
- Neu: `target_count_after_preset(current, selected)` analog zu `recurrence_after_preset` —
  schreibt beim Typwechsel den Preset-Wert.
- Neu: `shows_target_count(a)` / `shows_points_penalty(a)` → beide `false` für Bonus.
- `initial_open_groups`: das hartcodierte `s.target_count.trim() != "1"` gegen
  `selected.defaults().target_count` prüfen, sonst startet die Ziel-Gruppe bei Bonus immer offen.

**Verify:** `nix develop -c cargo test -p frontend`

**Done:** Typwechsel Bonus → Routine setzt das Soll zurück auf `1` (Round-Trip-Test).

### Task 4 — Formular: sechste Karte, Feld-Sichtbarkeit

**Files:** `frontend/src/components/task_modal.rs`

**Action:**
- `apply_archetype`: `target_count` aus dem Preset setzen; bei Bonus `points_penalty` leeren.
- Ziel-Gruppe: `TaskTargetCountField` nur wenn `shows_target_count(...)`.
- Punkte-Gruppe: `TaskPointsPenaltyField` nur wenn `shows_points_penalty(...)`.
  `TaskPointsRewardField` bleibt immer sichtbar (Entscheidung 1).

**Verify:** `nix develop -c cargo check -p frontend`

**Done:** Bonus-Formular zeigt weder Zielanzahl noch Strafpunkte; alle anderen Typen unverändert.

### Task 5 — Karte und Perioden-Anzeige

**Files:** `frontend/src/components/task_card.rs`, `frontend/src/components/period_tracker.rs`

**Action:**
- `task_card.rs`: Fortschritt bei `target <= 0` als `"{completions} ×"` statt `"{completions}/0"`.
- `period_tracker.rs`: neuer Prop `is_bonus: bool` (beide Komponenten). Bei Bonus wird
  `PeriodStatus::Failed` neutral gerendert (`period-skipped`, "-") statt rot — Entscheidung 2.
  Der Backend-Fix aus Task 2 macht `Failed` für neue Perioden unmöglich; der Prop deckt
  Alt-Bestand ab, der vor dem Fix bereits als `Failed` finalisiert wurde.
- Aufrufstellen in `task_card.rs` reichen `is_bonus` durch.

**Verify:** `nix develop -c cargo test -p frontend`

**Done:** Keine roten Markierungen für Bonusaufgaben, kein `/0` im Zähler.

### Task 6 — i18n

**Files:** `frontend/src/translations/de.json`, `frontend/src/translations/en.json`

**Action:** Sechs neue Keys je Sprache (`name`, `desc`, `form_title`, `note`, `assign_label`,
`assign_hint` unter `task_modal.archetype.bonus.*`). Beide Dateien müssen dieselbe Key-Menge
behalten.

**Verify:** Key-Anzahl beider Dateien vergleichen.

**Done:** Gleiche Key-Menge in de und en, keine fehlenden Übersetzungen.

### Task 7 — Qualitätsgates

**Action:** `nix develop -c cargo test --workspace`, `nix develop -c cargo clippy --workspace`

**Bekannte Vorbelastung:** 6 clippy-Findings in `backend/src/services/tasks.rs` und ~61 im
Frontend (STATE.md Blockers). Nur neu hinzugekommene zählen.

## must_haves

**Truths:**
- Ein Task mit `target_count = 0` und gutem `habit_type` leitet `Archetype::Bonus` ab
- Ein BadHabit mit Soll 0 bleibt BadHabit (Reihenfolge der Ableitung)
- Typwechsel von Bonus weg stellt `target_count = 1` wieder her
- Bonusaufgaben werden nie rot markiert und zeigen keine Strafpunkte
- Belohnungspunkte bleiben für Bonus verfügbar

**Artifacts:**
- `shared/src/types.rs`, `backend/src/services/background_jobs.rs`,
  `frontend/src/components/{task_form_model,task_modal,task_card,period_tracker}.rs`,
  `frontend/src/translations/{de,en}.json`
