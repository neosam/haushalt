---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Outbound Messaging
current_phase: "06"
current_phase_name: public-cross-household-report-links
status: executing
stopped_at: Phase 06 umgesetzt — Backend und Frontend gebaut, Testsuite grün
last_updated: "2026-08-07T00:00:00.000Z"
last_activity: 2026-08-07
last_activity_desc: "Bericht: (offen)-Marker für nicht erledigte Aufgaben, neuer deutscher Leerzustand für \"Gestern verpasst\""
progress:
  total_phases: 7
  completed_phases: 0
  total_plans: 17
  completed_plans: 11
  percent: 65
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-23)

**Core value:** Households can fairly delegate, track, and gamify recurring chores and habits across members with transparent points and streaks.
**Current focus:** Phase 02.1 — daily-task-report-inserted-urgent

## Current Position

Milestone: v1.2 Outbound Messaging
Phase: 06 (public-cross-household-report-links) — umgesetzt am 2026-07-31, 6 Pläne in 2 Commits
Plan: 6 of 6
Status: Code fertig und gegen einen laufenden Server verifiziert; offen ist nur die Sichtprüfung
der Einstellungen-Sektion im Browser.

Phase 05 (nomi.ai) ist auf Wunsch des Nutzers zurückgestellt. Die Pläne unter
`.planning/phases/05-nomi-ai-daily-report-push/` bleiben gültig — es wurde nichts davon
ausgeführt, also gibt es auch nichts zurückzudrehen.

Phase 02.1 ist code-seitig fertig (5 von 6 Plänen), nur die menschliche Abnahme 02.1-06 und die
SUMMARYs für 02.1-02..05 fehlen. Am 2026-07-28 nachgewiesen — siehe Doku-Nachtrag in ROADMAP.md.
Last activity: 2026-08-11 - Completed quick task 260811-fpr: Statistik-Perioden frei wählbar, Zeitraum-Nachberechnung ergänzt

Progress: [██░░░░░░░░] 17% (3 of ~10 plans across v1.1)

## Performance Metrics

**Velocity:**

- Total plans completed: 3 (01-01, 02-01, 02-02)
- Average duration: not yet tracked under GSD
- Total execution time: not yet tracked under GSD

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
| ----- | ----- | ----- | -------- |
| 1     | 1     | -     | -        |
| 2     | 2     | -     | -        |

**Recent Trend:** (no data yet — first GSD sessions)

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 02.1 P01 | 28min | 3 tasks | 2 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v1.1]: In-memory SQLite per test; pass `current_date` to functions under test (no global time mocking)
- [v1.1]: Tests stay in `#[cfg(test)] mod tests` next to code; fixtures use builder pattern
- [migration]: Switched spec-driven workflow from OpenSpec to GSD (`.planning/`)
- [v1.2, 2026-07-31]: Ein geteilter Bericht ist **pro Benutzer**, nicht pro Haushalt, und seine
  Haushaltsauswahl ist **explizit** — niemals "alle meine Haushalte". Sonst würde ein neu
  beigetretener Haushalt still in eine bereits verteilte URL rutschen.
- [v1.2, 2026-07-31]: Die Sprache ist eine Eigenschaft des einzelnen Berichts. Das schränkt
  Phase 2.1s D-01 ("der Bericht ist immer englisch") auf den Haushalts-Endpoint ein, statt es
  aufzuheben: `generate_daily_report` bleibt englisch, `generate_daily_report_localized` ist neu.
- [v1.2, 2026-07-31]: Das Rate-Limit des öffentlichen Endpoints zählt **nur existierende Tokens**.
  Der Limiter ist eine In-Memory-Map über den URL-Parameter; würde er unbekannte Tokens zählen,
  könnte jeder sie mit Zufalls-UUIDs unbegrenzt wachsen lassen.
- [v1.2]: Outbound push to nomi.ai instead of an inbound MCP server. An MCP design (phases 5-8, inbound read-only bearer tokens, separate `rmcp` server process) was fully planned on 2026-07-28 and then **discarded** — it solves the opposite problem. Do not revive it without a new requirement; the commits were abandoned, `jj op restore fbeab6da1e65` brings them back if ever needed.
- [v1.2]: The nomi.ai API key is stored **encrypted at rest**, not hashed. Unlike an inbound token, which is only ever compared, an outgoing key must be recoverable in plaintext to be used. The project has no encryption-at-rest facility yet — this is an open decision for `/gsd-discuss-phase 5`.
- [v1.2]: nomi.ai auth uses the **raw key** in the `Authorization` header, with no `Bearer ` prefix, per the official docs. Secondary sources claim otherwise.
- [Phase 02.1]: Extended TestTaskBuilder with with_suggestion() to unblock Task 2's suggestion-CHECK smoke test (builder had no way to set the field before this plan)

### Roadmap Evolution

- 2026-07-28: Milestone v1.2 mit Phasen 5-8 (MCP-Server) angelegt, Phase 5 vollständig geplant — und am selben Tag komplett verworfen. Der Nutzer braucht Push statt Pull.
- 2026-07-28: Milestone v1.2 als "Outbound Messaging" neu definiert, Phase 5 "Nomi.ai Daily Report Push" (NOMI-01..06). Hängt an Phase 2.1, die den Berichtstext liefert.

### Pending Todos

Keine — siehe `.planning/todos/completed/`.

### Blockers/Concerns

- Phase 2 plan 02-02 is partial: points-service integration tests for completion/uncomplete are still pending (TEST rows 4.6, 4.7, 4.9 in the former OpenSpec change) — they depend on the points service being wired into the test harness.
- Phase 3 (Extend Recurrence Types) has no defined scope — must run `/gsd-discuss-phase 3` before planning.
- Phase 2.1 is captured with assumed scope only — needs `/gsd-discuss-phase 2.1` before planning.
- Pre-existing clippy failure unrelated to current work: `frontend/src/components/solo_mode_banner.rs:66` trips `clippy::type_complexity` under the current toolchain, which fails `-D warnings`.
- GSD tooling must be run through the devShell (`nix develop -c gsd-tools ...`) — `node`, `cargo` and `gsd-tools` are not on the bare system PATH. Resolved for the devShell by quick task 260726-nxt; the old `NIXPKGS_ALLOW_UNFREE=1 --impure` note is obsolete since `claude-code` left the devShell in `2e42e4e`.
- GSD subagents (`gsd-planner`, `gsd-phase-researcher`, ...) ship in `.pi/gsd/agents/` but Claude Code only reads `~/.claude/agents/`. They were copied there on 2026-07-26; the agent registry loads at session start, so a fresh Claude Code session is required after any re-install. `gsd-pattern-mapper` is missing from `.pi/gsd/agents/` entirely (its plan:pre hook is non-blocking).
- ~~backend/src/services/tasks.rs (6 clippy errors)~~ — **RESOLVED** am 2026-07-27 durch Quick-Task `260727-t5x` (Commit `4d5324f9`); `deferred-items.md` Punkt 1 ist entsprechend markiert. `nix develop -c cargo clippy -p backend --all-targets` endet mit 0, am 2026-07-28 nachgemessen. Der Backend-Crate braucht **keine** Clippy-Ausnahme mehr — eine Ausnahme dort würde jetzt echte Regressionen verdecken. Offen bleiben nur ~61 Frontend-Findings über ~20 Dateien inkl. `frontend/src/components/solo_mode_banner.rs:66` (`clippy::type_complexity`).

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260726-nxt | flake.nix: nodejs und gsd-flake in den devShell aufnehmen | 2026-07-26 | 55f9485c | [260726-nxt-flake-nix-nodejs-und-gsd-flake-in-den-de](./quick/260726-nxt-flake-nix-nodejs-und-gsd-flake-in-den-de/) |
| 260726-th8 | Optionales Task-Setting: alle Mitglieder dürfen abhaken, auch nicht zugewiesene | 2026-07-26 | 1f3e965d | [260726-th8-anyone-can-complete-task](./quick/260726-th8-anyone-can-complete-task/) |
| 260726-vdz | Optionales Task-Setting: zugewiesene Person darf eine Erledigung nicht selbst zurücknehmen | 2026-07-26 | a7e0260a | [260726-vdz-assignee-cannot-uncomplete](./quick/260726-vdz-assignee-cannot-uncomplete/) |
| 260727-a1u | Task-Archetypen Welle 0: Archetype-Enum und Ableitung in shared | 2026-07-27 | 6b9e5bb6 | [260727-a1u-task-archetypen-welle-0-archetype-enum-u](./quick/260727-a1u-task-archetypen-welle-0-archetype-enum-u/) |
| 260727-apd | Welle 1a: Bulk-Edit aus task_modal.rs in eigene Komponente herausgelöst | 2026-07-27 | 7ea7397a | [260727-apd-welle-1a-bulk-edit-aus-task-modal-rs-in-](./quick/260727-apd-welle-1a-bulk-edit-aus-task-modal-rs-in-/) |
| 260727-b9u | Welle 1b: Task-Formular nach Archetypen umgebaut (Typauswahl, Basisfelder, Accordions) | 2026-07-27 | 5274d0ee | [260727-b9u-welle-1b-task-formular-nach-archetypen-u](./quick/260727-b9u-welle-1b-task-formular-nach-archetypen-u/) |
| 260727-dct | Freiform-Tasks (target_count 0) werden nicht mehr fälschlich bestraft | 2026-07-27 | 3f8ea3db | [260727-dct-freiform-tasks-target-count-0-werden-nic](./quick/260727-dct-freiform-tasks-target-count-0-werden-nic/) |
| 260727-fcg | Bonusaufgabe als sechster Task-Archetyp | 2026-07-27 | f43c0238 | [260727-fcg-bonusaufgabe-als-sechster-task-archetyp](./quick/260727-fcg-bonusaufgabe-als-sechster-task-archetyp/) |
| 260727-fs5 | Welle 2: Task-Karte nach Archetypen (sprechende Knöpfe, Akzente, Klartext statt totem Knopf) | 2026-07-27 | fc0ac402 | [260727-fs5-welle-2-task-karte-nach-archetypen](./quick/260727-fs5-welle-2-task-karte-nach-archetypen/) |
| 260727-hke | Zähler-Buttons reagieren sofort, Request wird gebündelt (echter Debounce) | 2026-07-27 | 4b88a78d | [260727-hke-zaehler-buttons-optimistisch-zaehlen-req](./quick/260727-hke-zaehler-buttons-optimistisch-zaehlen-req/) |
| 260727-t5x | Weekdays und Custom: get_next_due_date überspringt den heutigen Termin nicht mehr | 2026-07-27 | 4d5324f9 | [260727-t5x-weekdays-und-custom-get-next-due-date-da](./quick/260727-t5x-weekdays-und-custom-get-next-due-date-da/) |
| 260727-vst | Bad-Habit-Texte: "Rückfall" durch "Verstoß" ersetzt (de + en) | 2026-07-27 | 1008cd0a | — (via /gsd-fast) |
| 260728-c7k | Kategorie-Farbe in der Task-Liste anzeigen; Kategorie- und Tages-Gruppen klappbar | 2026-07-28 | 4ed8223d | [260728-c7k-kategorie-farbe-und-collapsible-gruppen](./quick/260728-c7k-kategorie-farbe-und-collapsible-gruppen/) |
| 260728-cej | Task verschwindet nach dem Bearbeiten aus der Übersicht: Kategorie in der Update-Antwort, Reload nach Save, Aufklappzustand der Gruppen | 2026-07-28 | 1ca3f836 | [260728-cej-bug-task-verschwindet-nach-bearbeiten-au](./quick/260728-cej-bug-task-verschwindet-nach-bearbeiten-au/) |
| 260728-dah | Kategorien-Modal wurde unten in den Seitenfluss gerendert: undefinierte CSS-Klassen korrigiert, Regressionstest ergänzt | 2026-07-28 | 17c21678 | [260728-dah-kategorien-modal-in-tasks-wird-nicht-als](./quick/260728-dah-kategorien-modal-in-tasks-wird-nicht-als/) |
| 260811-fpr | Statistiken für beliebige Wochen/Monate erstellen und ganze Zeiträume nachberechnen | 2026-08-11 | (siehe jj log) | [260811-fpr-statistik-beliebige-perioden-nachberechnung](./quick/260811-fpr-statistik-beliebige-perioden-nachberechnung/) |

## Session Continuity

Last session: 2026-07-26T16:56:26.676Z
Stopped at: Completed 02.1-01-PLAN.md
Resume file: .planning/phases/02.1-daily-task-report-inserted-urgent/02.1-CONTEXT.md
