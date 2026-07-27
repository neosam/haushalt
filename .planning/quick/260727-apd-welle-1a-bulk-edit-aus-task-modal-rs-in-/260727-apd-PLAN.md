---
phase: quick/260727-apd
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - frontend/src/components/bulk_edit_modal.rs
  - frontend/src/components/mod.rs
  - frontend/src/pages/tasks.rs
  - frontend/src/components/task_modal.rs
autonomous: true
requirements: [QUICK-260727-apd]

must_haves:
  truths:
    - "Bulk-Edit im Task-Screen verhält sich exakt wie vorher: gleiche Felder, gleiche Reihenfolge, gleiche Texte, gleicher Fortschrittsbalken, gleiche Fehlerliste"
    - "task_modal.rs enthält keinerlei Bulk-Logik mehr (kein is_bulk_edit, keine apply_*-Signale, keine Bulk-Props)"
    - "Das Bauen des UpdateTaskRequest aus den apply_*-Flags ist eine freie Funktion mit host-lauffähigen Tests"
    - "Die sechs Nicht-Bulk-Aufrufstellen von TaskModal kompilieren unverändert"
  artifacts:
    - path: "frontend/src/components/bulk_edit_modal.rs"
      provides: "BulkEditModal-Komponente + BulkEditForm + build_bulk_update_request + Tests"
      min_lines: 350
    - path: "frontend/src/components/mod.rs"
      provides: "Modul-Registrierung"
      contains: "pub mod bulk_edit_modal;"
    - path: "frontend/src/pages/tasks.rs"
      provides: "Aufruf von BulkEditModal statt TaskModal im Bulk-Zweig"
      contains: "BulkEditModal"
    - path: "frontend/src/components/task_modal.rs"
      provides: "Task-Modal ohne Bulk-Zweig"
      max_lines: 1650
  key_links:
    - from: "frontend/src/components/bulk_edit_modal.rs"
      to: "build_bulk_update_request"
      via: "Aufruf innerhalb der Update-Schleife von on_bulk_submit"
      pattern: "build_bulk_update_request\\(&"
    - from: "frontend/src/pages/tasks.rs"
      to: "BulkEditModal"
      via: "view! im show_bulk_edit_modal-Zweig"
      pattern: "<BulkEditModal"
    - from: "frontend/src/components/bulk_edit_modal.rs"
      to: "ApiClient::update_task"
      via: "Schleife über bulk_task_ids"
      pattern: "ApiClient::update_task"
---

<objective>
Bulk-Edit aus `task_modal.rs` in eine eigene Komponente `frontend/src/components/bulk_edit_modal.rs` herauslösen.

Purpose: `task_modal.rs` (1953 Zeilen, fünf Modi in einer Komponente) beherrschbar machen — Vorarbeit für den größeren Umbau des Task-Formulars.
Output: Neue Datei `bulk_edit_modal.rs` mit `BulkEditModal` + testbarer Request-Builder-Funktion; `task_modal.rs` ohne jede Bulk-Spur.

**Dies ist ein reiner Refactor.** Verhalten, UI, Texte, CSS-Klassen und DOM-Struktur müssen danach identisch sein. Keine neuen Features. Keine "Verbesserungen" nebenbei — auch nicht bei den unten explizit dokumentierten Kuriositäten.
</objective>

<execution_context>
@~/.claude/get-shit-done/workflows/execute-plan.md
@~/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@frontend/src/components/task_modal.rs
@frontend/src/components/task_fields.rs
@frontend/src/pages/tasks.rs
@frontend/src/components/mod.rs

## Toolchain (bindend)

`cargo` liegt NUR im nix devShell. Immer `nix develop -c cargo ...`. Ein nacktes `cargo` schlägt mit "command not found" fehl.

VCS ist jujutsu. Commits mit `jj commit <pfade> -m "..."`, NIEMALS `git commit`. Ohne Pfadfilter nimmt `jj commit` alles aus dem Working Copy — für atomare Commits also immer Pfade mitgeben.

## Gemessene Baselines (Stand vor dieser Änderung — bereits erhoben, nicht neu ermitteln)

| Kommando | Baseline |
| --- | --- |
| `nix develop -c cargo clippy -p frontend` | **GRÜN**, Exit 0, null Findings — Produktivcode ist clippy-sauber |
| `nix develop -c cargo clippy -p frontend --all-targets` | **61 Findings**, ausnahmslos in `#[cfg(test)]`-Code |
| `nix develop -c cargo test -p frontend --lib` | **31 passed, 0 failed** |
| `wc -l frontend/src/components/task_modal.rs` | **1953** |

Histogramm der 61 Testcode-Findings (zur Delta-Prüfung):

```
51 error: identical args used in this `assert_eq!` macro call
 7 error: useless use of `vec!`
 2 error: called `is_none()` after searching an `Iterator` with `position`
 1 error: this assertion is always `true`
```

Diese Altlast wird **nicht** mitrepariert. Maßstab ist "keine NEUEN Findings".

Wichtige Folgerung: weil `cargo clippy -p frontend` (ohne `--all-targets`) grün ist, ist das die harte Schranke für allen neuen Produktivcode — sie muss grün bleiben.

Ebenso wichtig: `cargo test -p frontend --lib` läuft auf dem Host und führt die einfachen `#[test]`-Funktionen wirklich aus (die `#[wasm_bindgen_test]`-Funktionen werden nur kompiliert, nicht ausgeführt). Das ist der einzige echte automatisierte Testhebel im Frontend — deshalb bekommt die neue reine Funktion `#[test]`, nicht `#[wasm_bindgen_test]`.

## Entscheidungen (getroffen, nicht neu aufrollen)

**D-01 — Tests der neuen reinen Funktion sind `#[test]`, nicht `#[wasm_bindgen_test]`.**
Begründung: `#[wasm_bindgen_test]` läuft nur im Browser und ist damit im normalen Build wertlos. `#[test]` wird von `cargo test -p frontend --lib` tatsächlich ausgeführt. Präzedenz: die bestehenden `delete_action_available`-Tests in `task_modal.rs` sind bereits einfache `#[test]`. CLAUDE.md verlangt Tests für Änderungen — nur so werden sie auch ausgeführt.

**D-02 — `delete_action_available` bleibt erhalten, verliert nur den `is_bulk_edit`-Parameter.**
Neue Signatur: `fn delete_action_available(has_task: bool, has_callback: bool) -> bool`. Sie ist danach zwar trivial (`has_task && has_callback`), trägt aber die dokumentierte Invariante "kein Callback = keine Berechtigung = kein Löschen" und ihre drei Tests sind die einzigen host-lauffähigen Tests in `task_modal.rs`. Ersatzloses Streichen würde Testabdeckung einer Berechtigungsregel für null Gewinn opfern.
Der Test `delete_hidden_during_bulk_edit` entfällt — seine Prämisse existiert in dieser Komponente nicht mehr. Die Aussage "im Bulk-Edit gibt es kein Löschen" ist in der neuen Welt strukturell garantiert: `BulkEditModal` enthält überhaupt keinen Löschpfad.

**D-03 — `on_bulk_save` wird bei `BulkEditModal` ein Pflicht-Prop `Callback<usize>` statt `Option<Callback<usize>>`.**
Die einzige Aufrufstelle (`tasks.rs`) übergibt es immer, das Verhalten ist also beobachtungsgleich, aber der `if let Some(...)`-Umweg entfällt.

**D-04 — `BulkEditModal` bekommt NICHT die Props `household_rewards`, `household_punishments`, `linked_rewards`, `linked_punishments`, `task`, `on_save`, `default_*`, `is_suggestion`, `on_delete`.**
Verifiziert: der Bulk-Zweig liest keine davon. Belohnungen/Bestrafungen kommen im Bulk-View nicht vor und `on_bulk_submit` fasst keine Reward-Links an. `tasks.rs` hört auf, sie zu übergeben.

## Kuriositäten, die 1:1 erhalten bleiben MÜSSEN

Diese Punkte sehen nach Bugs aus. Sie sind **nicht** Teil dieses Refactors. Nicht reparieren, sondern übernehmen und je mit einem kurzen Kommentar im Code markieren:

1. **`saving_text` im Bulk-Modus ist `t("task_modal.creating")`.** Weil `is_edit=false` und `is_suggestion=false` gilt, fällt die Kaskade in `task_modal.rs:715-721` auf den "Erstelle…"-Zweig. Während des Bulk-Speicherns steht also "Erstelle…" auf dem Button. Genau so übernehmen.
2. **`assigned_user_id` kann per Bulk-Edit nicht geleert werden.** `Some(None).flatten()` ist `None` — Häkchen "Zugewiesen an" gesetzt + leere Auswahl bewirkt nichts. Übernehmen.
3. **`due_time` kann per Bulk-Edit nicht geleert werden.** Gleiche `.flatten()`-Mechanik. Übernehmen.
4. **`category_id` kann per Bulk-Edit sehr wohl geleert werden** (`Some(None)`, nicht geflattet) — und eine ungültige UUID leert die Kategorie ebenfalls. Übernehmen.
5. **Auto-Auswahl bei genau einem zuweisbaren Mitglied.** `task_modal.rs:87-98` setzt `assigned_user` im Create-Modus vor, wenn `members.len() == 1` — das greift heute auch im Bulk-Modus und ist im Dropdown sichtbar. `BulkEditModal` muss das reproduzieren.
6. **`target_count` klemmt auf `.max(0)`**, nicht `.max(1)` — "-5" wird zu `Some(0)`.
7. **Startwerte im Bulk-Modus**: `recurrence_type = "daily"`, `target_count = "1"`, `allow_exceed_target = true`, alle übrigen Booleans `false`, `habit_type = "good"`, alle Text-/Zahlenfelder leer, `bulk_selected_weekday = 1`, `bulk_selected_month_day = 1`, `bulk_selected_weekdays = vec![]` (leer!), `selected_custom_dates` leer, alle 14 `apply_*` auf `false`.

## Relevante Interfaces (aus dem Codebase extrahiert — keine eigene Erkundung nötig)

Aus `shared/src/types.rs`:

```rust
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub recurrence_type: Option<RecurrenceType>,
    pub recurrence_value: Option<RecurrenceValue>,
    pub assigned_user_id: Option<Uuid>,
    pub target_count: Option<i32>,
    pub time_period: Option<TimePeriod>,
    pub allow_exceed_target: Option<bool>,
    pub anyone_can_complete: Option<bool>,
    pub assignee_cannot_uncomplete: Option<bool>,
    pub requires_review: Option<bool>,
    pub points_reward: Option<i64>,
    pub points_penalty: Option<i64>,
    pub due_time: Option<String>,
    pub habit_type: Option<HabitType>,
    pub category_id: Option<Option<Uuid>>,   // Some(None) = Kategorie löschen
    pub archived: Option<bool>,
    pub paused: Option<bool>,
}

pub enum RecurrenceValue {
    WeekDay(u8),            // 0 = Sonntag
    MonthDay(u8),           // 1-31
    Weekdays(Vec<u8>),
    CustomDates(Vec<chrono::NaiveDate>),
    None,
}
```

Aus `frontend/src/components/task_fields.rs` — alle bereits vorhanden, werden nur weiterverwendet:

```rust
pub fn BulkEditField(label: String, apply: RwSignal<bool>, children: Children)
pub fn TaskCategoryField(value: RwSignal<String>, categories: Vec<TaskCategory>, disabled: bool, hide_label: bool)
pub fn TaskAssignedUserField(value: RwSignal<String>, members: Vec<MemberWithUser>, disabled: bool, hide_label: bool)
pub fn TaskRecurrenceTypeField(value: RwSignal<String>, ...)
pub fn TaskWeekdayField(value: RwSignal<u8>, ...)
pub fn TaskMonthDayField(value: RwSignal<u8>, ...)
pub fn TaskWeekdaysField(value: RwSignal<Vec<u8>>, ...)
pub fn TaskTargetCountField(value: RwSignal<String>, ...)
pub fn TaskAllowExceedField(value: RwSignal<bool>, ...)
pub fn TaskAnyoneCanCompleteField(value: RwSignal<bool>, ...)
pub fn TaskAssigneeCannotUncompleteField(value: RwSignal<bool>, ...)
pub fn TaskRequiresReviewField(value: RwSignal<bool>, ...)
pub fn TaskOnDashboardField(value: RwSignal<bool>, ...)
pub fn TaskHabitTypeField(value: RwSignal<String>, ...)
pub fn TaskPointsRewardField(value: RwSignal<String>, ...)
pub fn TaskPointsPenaltyField(value: RwSignal<String>, ...)
pub fn TaskDueTimeField(value: RwSignal<String>, ...)
pub fn TaskPausedField(value: RwSignal<bool>, ...)
```

`BulkEditField` und alle `Task*Field` bleiben unverändert in `task_fields.rs`. `TaskAnyoneCanCompleteField` und `TaskAssigneeCannotUncompleteField` werden auch vom Nicht-Bulk-Zweig benutzt — der `use crate::components::task_fields::*;`-Import in `task_modal.rs` bleibt also bestehen.

## Bulk-Fundstellen in task_modal.rs (vorab kartiert)

| Zeilen | Inhalt |
| --- | --- |
| 15–17 | `delete_action_available(is_bulk_edit, ...)` |
| 44–47 | Props `bulk_task_ids`, `on_bulk_save` |
| 56–57 | `is_bulk_edit`, `bulk_task_count` |
| 63 | Aufruf `delete_action_available(is_bulk_edit, ...)` |
| 236–237 | `paused`-Signal (verifiziert: **ausschließlich** Bulk — nur in Z. 621f und Z. 1522f benutzt) |
| 239–253 | 14 `apply_*`-Signale |
| 255–258 | `bulk_selected_weekday`, `bulk_selected_month_day`, `bulk_selected_weekdays` |
| 260–262 | `bulk_progress`, `bulk_errors` |
| 493–659 | `on_bulk_submit` |
| 697–714 | `modal_title` / `submit_button_text` verzweigen auf `is_bulk_edit` |
| 756–798 | Fortschrittsanzeige + Fehlerliste |
| 800–806 | `on:submit`-Weiche |
| 809, 837–841, 844, 1420 | `<Show when=… is_bulk_edit>`-Weichen |
| 1422–1527 | Bulk-View-Zweig |
| 1616–1619 | Test `delete_hidden_during_bulk_edit` |

## Aufrufstellen von TaskModal (vorab verifiziert)

`dashboard.rs:694`, `quick_task_fab.rs:198`, `tasks.rs:628`, `tasks.rs:656`, `tasks.rs:684`, `tasks.rs:755`, `household.rs:1339`.

Per `grep` bestätigt: **nur `tasks.rs:755` setzt `bulk_task_ids`/`on_bulk_save`**. Die anderen sechs bleiben Zeichen für Zeichen unverändert.
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Reine Request-Builder-Logik als testbare freie Funktion anlegen</name>
  <files>frontend/src/components/bulk_edit_modal.rs (neu), frontend/src/components/mod.rs</files>

  <behavior>
Tests für `build_bulk_update_request` (alle als einfaches `#[test]`, siehe D-01). Mindestens diese Fälle:

- Nichts angehakt → jedes Feld des `UpdateTaskRequest` ist `None`
- `title` und `description` sind **immer** `None`, auch wenn alles andere angehakt ist
- `apply_category` + leerer String → `category_id == Some(None)` (Kategorie löschen)
- `apply_category` + gültige UUID → `category_id == Some(Some(uuid))`
- `apply_category` + Müll-String → `category_id == Some(None)`
- `apply_assigned_user` + leerer String → `assigned_user_id == None` (Kuriosität 2: Zuweisung ist per Bulk nicht löschbar)
- `apply_assigned_user` + gültige UUID → `assigned_user_id == Some(uuid)`
- `apply_due_time` + leerer String → `due_time == None` (Kuriosität 3)
- `apply_due_time` + `"07:30"` → `due_time == Some("07:30")`
- `apply_target_count` + `"invalid"` → `Some(1)`
- `apply_target_count` + `"-5"` → `Some(0)` (Kuriosität 6: `.max(0)`, nicht `.max(1)`)
- `apply_points_reward` + leerer String → `None`
- `apply_points_reward` + `"42"` → `Some(42)`
- `apply_recurrence` + `"weekly"` → `recurrence_type == Some(Weekly)` und `recurrence_value == Some(WeekDay(n))`
- `apply_recurrence` + `"daily"` → `recurrence_type == Some(Daily)` und `recurrence_value == None`
- `apply_recurrence` + unbekannter String → Fallback `Some(Daily)`
- `apply_recurrence == false`, aber `recurrence_type_raw` gesetzt → beide `None`
- `apply_paused` + `paused = true` → `paused == Some(true)`
- `apply_habit_type` + `"bad"` → `Some(HabitType::Bad)`; jeder andere String → `Some(HabitType::Good)`
  </behavior>

  <action>
**Schritt 0 — Baseline-Belege ablegen (vor jeder Codeänderung).**
Die Zahlen stehen bereits im `<context>`-Block oben; erhebe sie einmalig neu, damit du beim Abgleich am Ende dieselbe Kommandovariante vergleichst:

```bash
SCRATCH=/tmp/claude-1000/-home-neosam-programming-projects-haushalt/24dc3422-b44b-4148-bd9f-6c6cbb9095d7/scratchpad
mkdir -p "$SCRATCH"
cd /home/neosam/programming/projects/haushalt

nix develop -c cargo clippy -p frontend --all-targets --message-format=short 2>&1 \
  | grep -oE "(error|warning): [^[]*" | sed 's/[[:space:]]*$//' | sort | uniq -c | sort -rn \
  > "$SCRATCH/clippy-baseline.txt"
wc -l frontend/src/components/task_modal.rs > "$SCRATCH/lines-baseline.txt"
```

Erwartete Baseline: 51× `identical args used in this assert_eq!`, 7× `useless use of vec!`, 2× `is_none() after position`, 1× `this assertion is always true` — plus die Zusammenfassungszeile `could not compile frontend (lib test) due to 61 previous errors`. Weicht das ab, halte an und melde es, bevor du weitermachst.

**Schritt 1 — Neue Datei `frontend/src/components/bulk_edit_modal.rs` anlegen** mit ausschließlich der reinen Logik (die Komponente kommt in Task 2). Inhalt:

```rust
use shared::{HabitType, RecurrenceType, RecurrenceValue, UpdateTaskRequest};
use uuid::Uuid;

/// Momentaufnahme des Bulk-Edit-Formulars: welche Felder angehakt sind und mit welchem Wert.
///
/// Bewusst signalfrei, damit sich das Bauen des Requests ohne Browser testen lässt.
/// Rohwerte kommen so aus den Formularfeldern, wie der Nutzer sie eingegeben hat
/// (`*_raw`); das Parsen passiert hier zentral.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BulkEditForm {
    pub apply_category: bool,
    pub category_id_raw: String,
    pub apply_assigned_user: bool,
    pub assigned_user_raw: String,
    pub apply_recurrence: bool,
    pub recurrence_type_raw: String,
    pub weekday: u8,
    pub month_day: u8,
    pub weekdays: Vec<u8>,
    pub custom_dates: Vec<chrono::NaiveDate>,
    pub apply_target_count: bool,
    pub target_count_raw: String,
    pub apply_allow_exceed: bool,
    pub allow_exceed_target: bool,
    pub apply_anyone_can_complete: bool,
    pub anyone_can_complete: bool,
    pub apply_assignee_cannot_uncomplete: bool,
    pub assignee_cannot_uncomplete: bool,
    pub apply_requires_review: bool,
    pub requires_review: bool,
    pub apply_points_reward: bool,
    pub points_reward_raw: String,
    pub apply_points_penalty: bool,
    pub points_penalty_raw: String,
    pub apply_due_time: bool,
    pub due_time_raw: String,
    pub apply_habit_type: bool,
    pub habit_type_raw: String,
    pub apply_paused: bool,
    pub paused: bool,
}

/// Baut den Update-Request aus den angehakten Feldern. Nicht angehakte Felder bleiben
/// `None`, das Backend lässt sie dann unangetastet.
///
/// `title` und `description` sind immer `None` — beim Massen-Bearbeiten würden sie sonst
/// alle ausgewählten Aufgaben gleich benennen.
///
/// Nicht enthalten: die Dashboard-Sichtbarkeit. Sie hängt nicht am Task-Update, sondern an
/// eigenen Endpunkten und wird in der Komponente nach dem Update behandelt.
pub fn build_bulk_update_request(form: &BulkEditForm) -> UpdateTaskRequest {
    let category_id = if form.apply_category {
        if form.category_id_raw.is_empty() {
            Some(None)
        } else {
            Some(Uuid::parse_str(&form.category_id_raw).ok())
        }
    } else {
        None
    };

    // Doppeltes Option, dann geflattet: eine leere Auswahl kann die Zuweisung damit nicht
    // löschen. Übernommen aus dem bisherigen Verhalten.
    let assigned_user_id = if form.apply_assigned_user {
        if form.assigned_user_raw.is_empty() {
            Some(None)
        } else {
            Some(Uuid::parse_str(&form.assigned_user_raw).ok())
        }
    } else {
        None
    }
    .flatten();

    let recurrence_type = if form.apply_recurrence {
        Some(match form.recurrence_type_raw.as_str() {
            "onetime" => RecurrenceType::OneTime,
            "daily" => RecurrenceType::Daily,
            "weekly" => RecurrenceType::Weekly,
            "monthly" => RecurrenceType::Monthly,
            "weekdays" => RecurrenceType::Weekdays,
            "custom" => RecurrenceType::Custom,
            _ => RecurrenceType::Daily,
        })
    } else {
        None
    };

    let recurrence_value = if form.apply_recurrence {
        match form.recurrence_type_raw.as_str() {
            "weekly" => Some(RecurrenceValue::WeekDay(form.weekday)),
            "monthly" => Some(RecurrenceValue::MonthDay(form.month_day)),
            "weekdays" => Some(RecurrenceValue::Weekdays(form.weekdays.clone())),
            "custom" => Some(RecurrenceValue::CustomDates(form.custom_dates.clone())),
            _ => None, // onetime und daily brauchen keinen Wert
        }
    } else {
        None
    };

    UpdateTaskRequest {
        title: None,
        description: None,
        recurrence_type,
        recurrence_value,
        assigned_user_id,
        target_count: form
            .apply_target_count
            .then(|| form.target_count_raw.parse::<i32>().unwrap_or(1).max(0)),
        time_period: None,
        allow_exceed_target: form.apply_allow_exceed.then_some(form.allow_exceed_target),
        anyone_can_complete: form
            .apply_anyone_can_complete
            .then_some(form.anyone_can_complete),
        assignee_cannot_uncomplete: form
            .apply_assignee_cannot_uncomplete
            .then_some(form.assignee_cannot_uncomplete),
        requires_review: form.apply_requires_review.then_some(form.requires_review),
        points_reward: form
            .apply_points_reward
            .then(|| form.points_reward_raw.parse::<i64>().ok())
            .flatten(),
        points_penalty: form
            .apply_points_penalty
            .then(|| form.points_penalty_raw.parse::<i64>().ok())
            .flatten(),
        // Wie bei der Zuweisung: eine leere Uhrzeit löscht die Fälligkeit nicht.
        due_time: form
            .apply_due_time
            .then(|| {
                if form.due_time_raw.is_empty() {
                    None
                } else {
                    Some(form.due_time_raw.clone())
                }
            })
            .flatten(),
        habit_type: form.apply_habit_type.then(|| {
            match form.habit_type_raw.as_str() {
                "bad" => HabitType::Bad,
                _ => HabitType::Good,
            }
        }),
        category_id,
        archived: None,
        paused: form.apply_paused.then_some(form.paused),
    }
}
```

Diese Funktion ist die 1:1-Übersetzung von `task_modal.rs:513-626`. Gleiche Semantik, gleiche Reihenfolge, gleiche Fallbacks. Nichts glattziehen.

**Schritt 2 — Testmodul** am Dateiende mit den Fällen aus `<behavior>`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> BulkEditForm {
        BulkEditForm::default()
    }
    // … Tests
}
```

Beachte für die Tests:
- `#[test]`, **kein** `#[wasm_bindgen_test]`, **kein** `wasm_bindgen_test_configure!` (D-01).
- Kein `assert_eq!` mit syntaktisch identischen Argumenten und kein `vec![]` dort, wo ein Array reicht — sonst wachsen die 61 Clippy-Findings aus `--all-targets`.
- Die Komponente darf `BulkEditForm` später **nie** über `..Default::default()` befüllen (`weekday`/`month_day` wären dann 0 statt 1). `Default` existiert nur für die Testergonomie.

**Schritt 3 — Modul registrieren.** In `frontend/src/components/mod.rs` neben `pub mod task_modal;` eine Zeile `pub mod bulk_edit_modal;` ergänzen. Kein `pub use`-Glob — die Datei re-exportiert nur die Primitiv-Komponenten, Modals bleiben qualifiziert.

**Schritt 4 — Commit:**
```bash
jj commit frontend/src/components/bulk_edit_modal.rs frontend/src/components/mod.rs \
  -m "refactor(frontend): extract bulk edit request builder as testable function"
```
  </action>

  <verify>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo test -p frontend --lib 2>&1 | tail -3</automated>
    <expect>Mehr als 31 passed, 0 failed (31 Baseline + die neuen build_bulk_update_request-Tests)</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo clippy -p frontend 2>&1 | tail -3</automated>
    <expect>Exit 0, "Finished", keine Findings — der Produktivcode bleibt clippy-grün</expect>
  </verify>

  <done>
`frontend/src/components/bulk_edit_modal.rs` existiert mit `BulkEditForm` + `build_bulk_update_request` + Testmodul. `cargo test -p frontend --lib` meldet mehr als 31 bestandene Tests bei 0 Fehlern. `cargo clippy -p frontend` ist grün. `components/mod.rs` enthält `pub mod bulk_edit_modal;`. Commit liegt.
  </done>
</task>

<task type="auto">
  <name>Task 2: BulkEditModal-Komponente bauen und tasks.rs umstellen</name>
  <files>frontend/src/components/bulk_edit_modal.rs, frontend/src/pages/tasks.rs</files>
  <action>
**Schritt 1 — `BulkEditModal` in `bulk_edit_modal.rs` ergänzen** (oberhalb des Testmoduls, unterhalb der reinen Logik).

Props (exakt dieser Satz, hergeleitet aus dem, was der Bulk-Zweig tatsächlich liest — siehe D-03, D-04):

```rust
#[component]
pub fn BulkEditModal(
    /// IDs der ausgewählten Aufgaben, die gemeinsam aktualisiert werden
    bulk_task_ids: Vec<String>,
    household_id: String,
    members: Vec<MemberWithUser>,
    #[prop(default = vec![])] categories: Vec<TaskCategory>,
    #[prop(into)] on_close: Callback<()>,
    /// Bekommt die Anzahl erfolgreich aktualisierter Aufgaben
    #[prop(into)] on_bulk_save: Callback<usize>,
) -> impl IntoView
```

Signale — Startwerte gemäß Kuriosität 7 im `<context>`:
- Formular: `selected_category_id` (leer), `assigned_user`, `recurrence_type` (`"daily"`), `target_count` (`"1"`), `allow_exceed_target` (**`true`**), `anyone_can_complete`/`assignee_cannot_uncomplete`/`requires_review`/`on_dashboard`/`paused` (`false`), `habit_type` (`"good"`), `points_reward`/`points_penalty`/`due_time` (leer), `selected_custom_dates` (leer)
- Wiederholung: `bulk_selected_weekday = 1u8`, `bulk_selected_month_day = 1u8`, `bulk_selected_weekdays = Vec::<u8>::new()`
- Die 14 `apply_*` (alle `false`): `apply_category`, `apply_assigned_user`, `apply_target_count`, `apply_allow_exceed`, `apply_anyone_can_complete`, `apply_assignee_cannot_uncomplete`, `apply_requires_review`, `apply_on_dashboard`, `apply_habit_type`, `apply_points_reward`, `apply_points_penalty`, `apply_due_time`, `apply_paused`, `apply_recurrence`
- Zustand: `error: Option<String>`, `saving: bool`, `bulk_progress: (usize, usize)`, `bulk_errors: Vec<String>`

`assigned_user` startet mit der Auto-Auswahl aus Kuriosität 5: bei genau einem Eintrag in `members` dessen `user.id`, sonst leer. Kommentar dazusetzen, dass das aus dem alten `TaskModal`-Create-Pfad stammt.

`members` und `categories` per `store_value` ablegen (wie bisher), `i18n` per `store_value` als `i18n_stored`.

**`on_bulk_submit`** ist die Übernahme von `task_modal.rs:494-659`, mit einer einzigen strukturellen Änderung: innerhalb der Schleife wird zuerst ein `BulkEditForm` aus den Signalen gelesen und dann `build_bulk_update_request(&form)` aufgerufen.

Wichtig für Verhaltensgleichheit: das `BulkEditForm` wird **innerhalb** der Schleife pro Aufgabe neu aus den Signalen gelesen, genau wie der Request heute pro Iteration neu gebaut wird. Nicht vor die Schleife ziehen. Alle Felder explizit befüllen, kein `..Default::default()`.

Der Rest bleibt wörtlich:
- `saving.set(true)`, `error.set(None)`, `bulk_progress.set((0, ids.len()))`, `bulk_errors.set(vec![])` vor dem `spawn_local`
- Schleife mit `for (idx, task_id) in ids.iter().enumerate()`
- `ApiClient::update_task(&hid, task_id, request).await`
- Bei `Ok`: falls `apply_on_dashboard.get()`, je nach `on_dashboard.get()` `ApiClient::add_task_to_dashboard(task_id)` bzw. `remove_task_from_dashboard(task_id)`; danach `success_count += 1`
- Bei `Err(e)`: `error_list.push(format!("Task {}: {}", &task_id[..8], e))` — identisches Format inklusive 8-Zeichen-Kürzung
- Nach jeder Iteration `bulk_progress.set((idx + 1, ids.len()))`
- Nach der Schleife `saving.set(false)`, `bulk_errors.set(error_list.clone())`, und nur bei leerer Fehlerliste `on_bulk_save.call(success_count)`

**Texte** (identisch zur alten Bulk-Kaskade):
- `modal_title = i18n.t("tasks.bulk_edit_title").replace("{count}", &bulk_task_count.to_string())`
- `submit_button_text = i18n.t("tasks.edit_selected")`
- `saving_text = i18n.t("task_modal.creating")` ← Kuriosität 1, Kommentar dazusetzen

**View** — gleiche Struktur, gleiche Klassen, gleiche Inline-Styles, gleiche Reihenfolge wie heute:

1. `<div class="modal-backdrop" on:click=close>` → `<div class="modal modal-task" on:click=|e| e.stop_propagation()>`
2. `<div class="modal-header">` mit `<h3 class="modal-title">{modal_title}</h3>` und `<button class="modal-close" on:click=close>"×"</button>`
3. Fehler-Alert: `{move || error.get().map(|e| view!{ <div class="alert alert-error" style="margin: 1rem;">{e}</div> })}`
4. Fortschrittsanzeige (aus `task_modal.rs:757-776`), Bedingung wird von `is_bulk_edit && saving.get()` zu `saving.get()`; `bulk-edit-progress` / `bulk-edit-progress-bar` / `bulk-edit-progress-fill` und die Prozentrechnung `(completed * 100) / total` mit Nulldivisions-Schutz unverändert
5. Fehlerliste (aus `task_modal.rs:779-798`) unverändert, inklusive `tasks.bulk_edit_partial` mit `{success}`/`{total}`/`{failed}`
6. `<form on:submit=on_bulk_submit>`:
   - `<div style="padding: 1rem; max-height: 60vh; overflow-y: auto;">`
     - Hinweis: `<div class="alert alert-info" style="margin-bottom: 1rem;">{t("tasks.bulk_edit_hint")}</div>`
     - Die Feldblöcke aus `task_modal.rs:1429-1524` in exakt dieser Reihenfolge: Kategorie, Wiederholung, die vier bedingten Wiederholungsfelder (`margin-left: 1.5rem;`, Bedingung jeweils `apply_recurrence.get() && recurrence_type.get() == "…"`), Zielanzahl, Überschreiten erlauben, Anyone-can-complete, Assignee-cannot-uncomplete, Review nötig, Auf Dashboard, Habit-Typ, Punkte-Belohnung, Punkte-Strafe, Fälligkeitszeit, Zuweisung, Pausiert
     - Die äußeren `<Show when=move || is_bulk_edit>`-Hüllen entfallen ersatzlos (die Bedingung ist jetzt konstant wahr) — die gerenderte DOM-Struktur bleibt dadurch gleich
   - `<div class="modal-footer">` mit Abbrechen- und Submit-Button. `disabled` wird von `saving.get() || deleting.get()` zu `saving.get()` (`deleting` war im Bulk-Modus immer `false`)
   - **Kein** Lösch-Block und **kein** Lösch-Bestätigungs-Footer — `can_delete` war im Bulk-Modus immer `false`

Imports der neuen Datei ergänzen: `leptos::*`, `shared::{MemberWithUser, TaskCategory, …}`, `crate::api::ApiClient`, `crate::components::calendar_picker::CalendarPicker`, `crate::components::task_fields::*`, `crate::i18n::use_i18n`.

**Schritt 2 — `frontend/src/pages/tasks.rs` umstellen.**
- Import ergänzen: `use crate::components::bulk_edit_modal::BulkEditModal;` (der `TaskModal`-Import bleibt, drei andere Aufrufstellen brauchen ihn)
- Den Block ab Zeile 755 ersetzen: statt `<TaskModal task=None … bulk_task_ids=selected_ids … on_save=move |_| {} on_bulk_save=… />` nun

```rust
<BulkEditModal
    bulk_task_ids=selected_ids
    household_id=hid
    members=assignable_members
    categories=categories.get()
    on_close=move |_| show_bulk_edit_modal.set(false)
    on_bulk_save=Callback::new(move |_count: usize| { /* unverändert */ })
/>
```

Der `on_bulk_save`-Rumpf bleibt Zeile für Zeile identisch (Modal schließen, Multi-Select verlassen, Auswahl leeren, Aufgaben neu laden). Die Props `task`, `household_rewards`, `household_punishments`, `linked_rewards`, `linked_punishments`, `on_save` entfallen (D-04). Wenn dadurch `rewards`/`punishments` in diesem Block ungenutzt werden, entferne nur die Zeilen im Block — die Signale selbst werden von den anderen Modals weiterverwendet.

Die drei anderen `TaskModal`-Aufrufe in `tasks.rs` (Zeilen ~628, ~656, ~684) bleiben unangetastet.

**Schritt 3 — Commit:**
```bash
jj commit frontend/src/components/bulk_edit_modal.rs frontend/src/pages/tasks.rs \
  -m "refactor(frontend): move bulk edit into its own BulkEditModal component"
```
  </action>
  <verify>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo check --workspace 2>&1 | tail -5</automated>
    <expect>Finished, keine Fehler</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo clippy -p frontend 2>&1 | tail -3</automated>
    <expect>Exit 0, keine Findings</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo test -p frontend --lib 2>&1 | tail -3</automated>
    <expect>0 failed, gleiche Anzahl bestandener Tests wie nach Task 1</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && grep -c "BulkEditModal" frontend/src/pages/tasks.rs</automated>
    <expect>2 (Import + Aufruf)</expect>
  </verify>
  <done>
`BulkEditModal` existiert mit den sechs Props aus D-03/D-04 und ruft `build_bulk_update_request` in der Update-Schleife auf. `tasks.rs` nutzt `<BulkEditModal>` im Bulk-Zweig, die drei anderen `TaskModal`-Aufrufe dort sind unverändert. `cargo check --workspace` und `cargo clippy -p frontend` sind grün, die Tests laufen unverändert durch. Commit liegt.
  </done>
</task>

<task type="auto">
  <name>Task 3: Bulk-Code aus task_modal.rs entfernen und delete_action_available vereinfachen</name>
  <files>frontend/src/components/task_modal.rs</files>
  <action>
Jetzt existiert der Bulk-Pfad doppelt. Diese Aufgabe entfernt die tote Hälfte aus `task_modal.rs`. Arbeite die Fundstellen-Tabelle aus dem `<context>` von unten nach oben ab, damit die Zeilennummern stabil bleiben.

**Zu entfernen:**
1. Test `delete_hidden_during_bulk_edit` (Z. 1615-1619) — siehe D-02
2. Der gesamte Bulk-View-Zweig `<Show when=move || is_bulk_edit>…</Show>` (Z. 1422-1527)
3. Die drei `<Show>`-Weichen um den Nicht-Bulk-View: `<Show when=move || !is_bulk_edit fallback=|| ()>` bei Z. 809 (Titel/Beschreibung, schließt bei Z. 834) und bei Z. 844 (schließt bei Z. 1420) — Hülle weg, Inhalt bleibt, Einrückung anpassen
4. Der Bulk-Hinweis `<Show when=move || is_bulk_edit>` mit `tasks.bulk_edit_hint` (Z. 837-841)
5. Die `on:submit`-Weiche (Z. 800-806) wird zu `<form on:submit=on_submit>`
6. Fortschrittsanzeige (Z. 756-776) und Bulk-Fehlerliste (Z. 779-798)
7. Die `is_bulk_edit`-Zweige in `modal_title` (Z. 697-699) und `submit_button_text` (Z. 706-708) — die Kaskaden beginnen danach mit `if is_edit`
8. `on_bulk_submit` komplett (Z. 493-659)
9. `bulk_progress`, `bulk_errors` (Z. 260-262)
10. `bulk_selected_weekday`, `bulk_selected_month_day`, `bulk_selected_weekdays` (Z. 255-258)
11. Alle 14 `apply_*`-Signale (Z. 239-253)
12. `paused`-Signal samt Kommentar (Z. 236-237) — verifiziert bulk-only. `paused: None` im `UpdateTaskRequest` bei Z. 367 bleibt stehen, das ist der Normalpfad
13. Props `bulk_task_ids` und `on_bulk_save` samt Doc-Kommentaren (Z. 44-47)
14. `is_bulk_edit` und `bulk_task_count` (Z. 56-57)

**Anzupassen:**
- `delete_action_available` (Z. 15-17) verliert den ersten Parameter (D-02):

```rust
/// Whether the edit modal should offer the delete action.
///
/// Delete applies to exactly one existing task, so it is hidden in create/duplicate mode.
/// The caller opts in by passing a callback, which is how permission is expressed — no
/// callback means no delete.
fn delete_action_available(has_task: bool, has_callback: bool) -> bool {
    has_task && has_callback
}
```
- Aufrufstelle Z. 62-63: `delete_action_available(delete_task_id.is_some(), on_delete.is_some())`
- Die drei verbleibenden Tests auf die neue Signatur umstellen: `delete_action_available(true, true)`, `delete_action_available(true, false)`, `delete_action_available(false, true)`. Namen und Kommentare beibehalten (bis auf den entfernten Bulk-Test)

**Nicht anfassen:** der `use crate::components::task_fields::*;`-Import bleibt (`TaskAnyoneCanCompleteField` und `TaskAssigneeCannotUncompleteField` werden im Nicht-Bulk-View benutzt). Ebenso `CalendarPicker`. Prüfe nach dem Aufräumen aber, ob durch die Entfernung ungenutzte `use`-Einträge entstanden sind — `cargo check` meldet das.

**Abschließender Baseline-Abgleich** (gleiche Kommandovariante wie in Task 1, Schritt 0):

```bash
SCRATCH=/tmp/claude-1000/-home-neosam-programming-projects-haushalt/24dc3422-b44b-4148-bd9f-6c6cbb9095d7/scratchpad
cd /home/neosam/programming/projects/haushalt
nix develop -c cargo clippy -p frontend --all-targets --message-format=short 2>&1 \
  | grep -oE "(error|warning): [^[]*" | sed 's/[[:space:]]*$//' | sort | uniq -c | sort -rn \
  > "$SCRATCH/clippy-after.txt"
diff "$SCRATCH/clippy-baseline.txt" "$SCRATCH/clippy-after.txt"
```

Erwartung: keine neuen Meldungsarten, keine gestiegenen Zähler. Die Gesamtzahl darf sinken (der entfernte Bulk-Test), aber nicht steigen. Steigt sie, behebe die Ursache in deinem neuen Code — die 61 Altlasten bleiben unangetastet.

**Commit:**
```bash
jj commit frontend/src/components/task_modal.rs \
  -m "refactor(frontend): drop bulk edit branch from TaskModal"
```
  </action>
  <verify>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo check --workspace 2>&1 | tail -5</automated>
    <expect>Finished, keine Fehler — belegt zugleich, dass alle sieben TaskModal-Aufrufstellen weiter kompilieren</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo clippy -p frontend 2>&1 | tail -3</automated>
    <expect>Exit 0, keine Findings</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && nix develop -c cargo test -p frontend --lib 2>&1 | tail -3</automated>
    <expect>0 failed; Anzahl = 31 - 1 (entfallener Bulk-Test) + neue Tests aus Task 1</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && grep -ci "bulk" frontend/src/components/task_modal.rs</automated>
    <expect>0</expect>
    <automated>cd /home/neosam/programming/projects/haushalt && wc -l frontend/src/components/task_modal.rs frontend/src/components/bulk_edit_modal.rs</automated>
    <expect>task_modal.rs unter 1650 Zeilen (Baseline 1953), bulk_edit_modal.rs über 350 Zeilen</expect>
  </verify>
  <done>
`grep -ci "bulk" frontend/src/components/task_modal.rs` liefert 0. `delete_action_available` hat zwei Parameter und drei bestandene Tests. `cargo check --workspace`, `cargo clippy -p frontend` und `cargo test -p frontend --lib` sind grün. Der Clippy-Diff gegen die Baseline zeigt keine neuen oder gestiegenen Findings. task_modal.rs ist unter 1650 Zeilen. Commit liegt.
  </done>
</task>

</tasks>

<verification>
Nach allen drei Tasks:

```bash
cd /home/neosam/programming/projects/haushalt
nix develop -c cargo check --workspace
nix develop -c cargo clippy -p frontend                  # muss grün bleiben (Baseline: grün)
nix develop -c cargo test -p frontend --lib              # 0 failed
grep -ci "bulk" frontend/src/components/task_modal.rs    # 0
grep -rn "bulk_task_ids\|on_bulk_save" frontend/src      # nur bulk_edit_modal.rs und tasks.rs
wc -l frontend/src/components/task_modal.rs              # < 1650 (war 1953)
jj log -r 'trunk()..@' --no-graph                        # drei Commits
```

Clippy-Delta gegen die in Task 1 abgelegte Baseline: keine neue Meldungsart, kein gestiegener Zähler.

**Manuelle Sichtprüfung (nicht blockierend, aber empfohlen):** `cd frontend && trunk serve`, Aufgabenliste öffnen, Mehrfachauswahl aktivieren, zwei Aufgaben wählen, "Ausgewählte bearbeiten" — Titel mit Anzahl, Hinweistext, Feldreihenfolge, Fortschrittsbalken und Erfolgsverhalten müssen wie vorher aussehen.
</verification>

<success_criteria>
- `frontend/src/components/bulk_edit_modal.rs` enthält `BulkEditModal`, `BulkEditForm` und `build_bulk_update_request` mit host-lauffähigen `#[test]`s
- `frontend/src/components/task_modal.rs` enthält null Vorkommen von "bulk" (case-insensitive) und ist unter 1650 Zeilen
- `delete_action_available(has_task, has_callback)` mit drei bestandenen Tests
- `frontend/src/pages/tasks.rs` rendert `<BulkEditModal>` im Bulk-Zweig; die drei anderen `TaskModal`-Aufrufe dort sind unverändert
- Die sechs Nicht-Bulk-Aufrufstellen (`dashboard.rs`, `quick_task_fab.rs`, `household.rs`, `tasks.rs` ×3) sind unverändert und kompilieren
- `cargo check --workspace` grün, `cargo clippy -p frontend` grün, `cargo test -p frontend --lib` ohne Fehler
- Keine neuen Findings in `cargo clippy -p frontend --all-targets` gegenüber der 61er-Baseline
- Drei atomare `jj`-Commits
</success_criteria>

<output>
Nach Abschluss `.planning/quick/260727-apd-welle-1a-bulk-edit-aus-task-modal-rs-in-/260727-apd-SUMMARY.md` anlegen mit: Zeilenzahlen vorher/nachher, Clippy-Delta, Testanzahl vorher/nachher, und einer Liste der bewusst übernommenen Kuriositäten (für die spätere Aufräumrunde beim großen Task-Formular-Umbau).
</output>
</content>
</invoke>
