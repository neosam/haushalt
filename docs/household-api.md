# Haushalts-API

Referenz aller Endpunkte, die zu einem einzelnen Haushalt gehören — also alles unter
`/api/households/{hid}/…`. Genau diese Endpunkte sind mit einem **API-Token** erreichbar
(siehe [Authentifizierung](#authentifizierung)); mit einem normalen Login-JWT sind sie
ebenfalls nutzbar.

> Die genauen Felder der Request- und Response-Bodies stehen als Rust-Typen im `shared`-Crate
> (`shared/src/types.rs`) und sind dort die verbindliche Quelle. Diese Datei nennt pro
> Endpunkt den **Typnamen**; Details bitte dort nachschlagen.

---

## Authentifizierung

Zwei Wege, beide über denselben Header:

```
Authorization: Bearer <token>
```

- **Login-JWT** — der normale Nutzer-Token aus `POST /api/auth/login`.
- **API-Token** — ein `hht_…`-Token, das ein Mitglied unter
  [`/api/users/me/api-tokens`](#api-tokens-verwalten) anlegt.

### Regeln für API-Tokens

- Ein Token ist an **genau einen Haushalt** gebunden. Nur dessen `/api/households/{hid}/…`
  ist erreichbar; ein fremder Haushalt oder ein kontobezogener Pfad ergibt `403`.
- **Lesen** (`GET`, `HEAD`) geht mit jedem gültigen Token.
- **Schreiben** (`POST`, `PUT`, `PATCH`, `DELETE`) braucht ein Token mit `can_write = true`.
- Ein Token handelt mit der **Rolle seines Erstellers**. Aktionen, die Owner/Admin verlangen
  (z. B. Einstellungen oder Mitglieder verwalten), scheitern mit `403`, wenn der Ersteller
  diese Rolle nicht hat — auch mit `can_write`.

---

## Antwort- und Fehlerformat

Erfolgreiche Antworten sind in ein `data`-Objekt gehüllt:

```json
{ "data": { "...": "..." } }
```

Listen liefern `{ "data": [ … ] }`. `DELETE` antwortet meist `204 No Content`.

Fehler:

```json
{ "error": "forbidden", "message": "Token is read-only" }
```

| Status | Bedeutung |
|--------|-----------|
| `200` / `201` / `204` | OK |
| `400` | ungültiger Body oder ungültige Parameter |
| `401` | Token fehlt, ist ungültig, deaktiviert oder hat das falsche Format |
| `403` | falscher Haushalt · Schreibversuch mit Read-only-Token · fehlende Rolle |
| `404` | Ressource nicht gefunden |

In der folgenden Referenz ist `{hid}` immer die Haushalts-ID. Eine leere Body-Spalte („—")
heißt: kein JSON-Body. Der Response-Typ pro Abschnitt steht in dessen Überschrift.

---

## Haushalt & Mitglieder

Response-Typen: `Household`, `MemberWithUser`, `LeaderboardEntry`, `HouseholdSettings`,
`Invitation`, `AdjustPointsResponse`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/api/households/{hid}` | — | Haushalt lesen |
| PUT | `/api/households/{hid}` | `UpdateHouseholdRequest` | Haushalt umbenennen |
| DELETE | `/api/households/{hid}` | — | Haushalt löschen |
| GET | `/api/households/{hid}/members` | — | Mitglieder auflisten |
| GET | `/api/households/{hid}/leaderboard` | — | Rangliste nach Punkten |
| POST | `/api/households/{hid}/members/{user_id}/points` | `AdjustPointsRequest` | Punkte anpassen |
| PUT | `/api/households/{hid}/members/{user_id}/role` | `UpdateRoleRequest` | Rolle ändern |
| DELETE | `/api/households/{hid}/members/{user_id}` | — | Mitglied entfernen |
| POST | `/api/households/{hid}/invite` | `CreateInvitationRequest` | Mitglied einladen |
| GET | `/api/households/{hid}/invitations` | — | offene Einladungen |
| DELETE | `/api/households/{hid}/invitations/{inv_id}` | — | Einladung zurückziehen |
| GET | `/api/households/{hid}/settings` | — | Einstellungen lesen |
| PUT | `/api/households/{hid}/settings` | `UpdateHouseholdSettingsRequest` | Einstellungen ändern |
| POST | `/api/households/{hid}/solo-mode/activate` | — | Solo-Modus aktivieren |
| POST | `/api/households/{hid}/solo-mode/request-exit` | — | Solo-Modus-Ausstieg beantragen |
| POST | `/api/households/{hid}/solo-mode/cancel-exit` | — | Ausstieg abbrechen |

---

## Aufgaben

Basis: `/api/households/{hid}/tasks`. Response-Typen: `Task`, `TaskWithStatus`,
`TaskWithDetails`, `TaskCompletion`, `PendingReview`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/tasks` | — | Aufgaben auflisten |
| POST | `/tasks` | `CreateTaskRequest` | Aufgabe anlegen |
| GET | `/tasks/due` | — | heute fällige Aufgaben |
| GET | `/tasks/all` | — | alle Aufgaben mit Status (`TaskWithStatus`) |
| GET | `/tasks/assigned-to-me` | — | mir zugewiesene Aufgaben |
| GET | `/tasks/archived` | — | archivierte Aufgaben |
| GET | `/tasks/suggestions` | — | Aufgaben-Vorschläge |
| GET | `/tasks/pending-reviews` | — | Erledigungen, die auf Freigabe warten |
| GET | `/tasks/{task_id}` | — | einzelne Aufgabe |
| GET | `/tasks/{task_id}/details` | — | Aufgabe mit Details (`TaskWithDetails`) |
| PUT | `/tasks/{task_id}` | `UpdateTaskRequest` | Aufgabe ändern |
| DELETE | `/tasks/{task_id}` | — | Aufgabe löschen |
| POST | `/tasks/{task_id}/complete` | — | abhaken |
| POST | `/tasks/{task_id}/uncomplete` | — | Abhaken rückgängig |
| POST | `/tasks/{task_id}/archive` | — | archivieren |
| POST | `/tasks/{task_id}/unarchive` | — | Archivierung aufheben |
| POST | `/tasks/{task_id}/pause` | — | pausieren |
| POST | `/tasks/{task_id}/unpause` | — | fortsetzen |
| POST | `/tasks/{task_id}/approve` | — | Vorschlag annehmen |
| POST | `/tasks/{task_id}/deny` | — | Vorschlag ablehnen |
| POST | `/tasks/completions/{completion_id}/approve` | — | Erledigung freigeben |
| POST | `/tasks/completions/{completion_id}/reject` | — | Erledigung ablehnen |
| GET | `/tasks/{task_id}/rewards` | — | verknüpfte Belohnungen |
| POST | `/tasks/{task_id}/rewards/{reward_id}` | — | Belohnung verknüpfen |
| DELETE | `/tasks/{task_id}/rewards/{reward_id}` | — | Verknüpfung lösen |
| GET | `/tasks/{task_id}/punishments` | — | verknüpfte Strafen |
| POST | `/tasks/{task_id}/punishments/{punishment_id}` | — | Strafe verknüpfen |
| DELETE | `/tasks/{task_id}/punishments/{punishment_id}` | — | Verknüpfung lösen |

---

## Kategorien

Basis: `/api/households/{hid}/categories`. Response-Typ: `TaskCategory`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/categories` | — | auflisten |
| POST | `/categories` | `CreateTaskCategoryRequest` | anlegen |
| GET | `/categories/{category_id}` | — | einzeln |
| PUT | `/categories/{category_id}` | `UpdateTaskCategoryRequest` | ändern |
| DELETE | `/categories/{category_id}` | — | löschen |

---

## Belohnungen

Basis: `/api/households/{hid}/rewards`. Response-Typen: `Reward`, `UserRewardWithUser`,
`PendingRewardRedemption`, `RandomRewardPickResult`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/rewards` | — | auflisten |
| POST | `/rewards` | `CreateRewardRequest` | anlegen |
| GET | `/rewards/{reward_id}` | — | einzeln |
| PUT | `/rewards/{reward_id}` | `UpdateRewardRequest` | ändern |
| DELETE | `/rewards/{reward_id}` | — | löschen |
| POST | `/rewards/{reward_id}/purchase` | — | für Punkte kaufen |
| POST | `/rewards/{reward_id}/assign/{user_id}` | — | einem Mitglied zuweisen |
| POST | `/rewards/{reward_id}/unassign/{user_id}` | — | Zuweisung aufheben |
| GET | `/rewards/{reward_id}/options` | — | Optionen lesen |
| POST | `/rewards/{reward_id}/options/{option_id}` | — | Option hinzufügen |
| DELETE | `/rewards/{reward_id}/options/{option_id}` | — | Option entfernen |
| GET | `/rewards/user-rewards` | — | eigene erhaltene Belohnungen |
| GET | `/rewards/user-rewards/all` | — | alle erhaltenen Belohnungen |
| DELETE | `/rewards/user-rewards/{id}` | — | erhaltene Belohnung löschen |
| POST | `/rewards/user-rewards/{id}/redeem` | — | einlösen |
| POST | `/rewards/user-rewards/{id}/approve` | — | Einlösung bestätigen |
| POST | `/rewards/user-rewards/{id}/reject` | — | Einlösung ablehnen |
| POST | `/rewards/user-rewards/{id}/pick` | — | zufällige Belohnung ziehen |
| GET | `/rewards/pending-confirmations` | — | offene Einlösungen |

---

## Strafen

Basis: `/api/households/{hid}/punishments`. Response-Typen: `Punishment`,
`UserPunishmentWithUser`, `PendingPunishmentCompletion`, `RandomPickResult`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/punishments` | — | auflisten |
| POST | `/punishments` | `CreatePunishmentRequest` | anlegen |
| GET | `/punishments/{punishment_id}` | — | einzeln |
| PUT | `/punishments/{punishment_id}` | `UpdatePunishmentRequest` | ändern |
| DELETE | `/punishments/{punishment_id}` | — | löschen |
| POST | `/punishments/{punishment_id}/assign/{user_id}` | — | zuweisen |
| POST | `/punishments/{punishment_id}/unassign/{user_id}` | — | Zuweisung aufheben |
| GET | `/punishments/{punishment_id}/options` | — | Optionen lesen |
| GET | `/punishments/user-punishments` | — | eigene erhaltene Strafen |
| GET | `/punishments/user-punishments/all` | — | alle erhaltenen Strafen |
| DELETE | `/punishments/user-punishments/{id}` | — | erhaltene Strafe löschen |
| POST | `/punishments/user-punishments/{id}/complete` | — | als erledigt markieren |
| POST | `/punishments/user-punishments/{id}/approve` | — | Erledigung bestätigen |
| POST | `/punishments/user-punishments/{id}/reject` | — | Erledigung ablehnen |
| POST | `/punishments/user-punishments/{id}/pick` | — | zufällige Strafe ziehen |
| GET | `/punishments/pending-confirmations` | — | offene Erledigungen |

---

## Punktebedingungen

Basis: `/api/households/{hid}/point-conditions`. Response-Typ: `PointCondition`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/point-conditions` | — | auflisten |
| POST | `/point-conditions` | `CreatePointConditionRequest` | anlegen |
| GET | `/point-conditions/{condition_id}` | — | einzeln |
| PUT | `/point-conditions/{condition_id}` | `UpdatePointConditionRequest` | ändern |
| DELETE | `/point-conditions/{condition_id}` | — | löschen |

---

## Kommunikation & Inhalte

Basis: `/api/households/{hid}`. Response-Typen: `ChatMessageWithUser`, `NoteWithUser`,
`JournalEntryWithUser`, `Announcement`, `ActivityLogWithUsers`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/chat` | — | Nachrichten lesen |
| POST | `/chat` | `CreateChatMessageRequest` | Nachricht senden |
| PUT | `/chat/{message_id}` | `UpdateChatMessageRequest` | bearbeiten |
| DELETE | `/chat/{message_id}` | — | löschen |
| GET | `/notes` | — | Notizen lesen |
| POST | `/notes` | `CreateNoteRequest` | anlegen |
| GET | `/notes/{note_id}` | — | einzeln |
| PUT | `/notes/{note_id}` | `UpdateNoteRequest` | ändern |
| DELETE | `/notes/{note_id}` | — | löschen |
| GET | `/journal` | — | Journal lesen |
| POST | `/journal` | `CreateJournalEntryRequest` | Eintrag anlegen |
| GET | `/journal/{entry_id}` | — | einzeln |
| PUT | `/journal/{entry_id}` | `UpdateJournalEntryRequest` | ändern |
| DELETE | `/journal/{entry_id}` | — | löschen |
| GET | `/announcements` | — | Ankündigungen lesen |
| GET | `/announcements/active` | — | nur aktive |
| POST | `/announcements` | `CreateAnnouncementRequest` | anlegen |
| GET | `/announcements/{announcement_id}` | — | einzeln |
| PUT | `/announcements/{announcement_id}` | `UpdateAnnouncementRequest` | ändern |
| DELETE | `/announcements/{announcement_id}` | — | löschen |
| GET | `/activities` | — | Aktivitätsprotokoll |

---

## Statistik & Tagesbericht

Basis: `/api/households/{hid}`. Response-Typen: `WeeklyStatisticsResponse`,
`MonthlyStatisticsResponse`, `DailyReportResponse`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/statistics/weekly` | — | Wochenstatistik |
| POST | `/statistics/weekly/calculate` | — | Wochenstatistik neu berechnen |
| GET | `/statistics/weekly/available` | — | verfügbare Wochen |
| GET | `/statistics/monthly` | — | Monatsstatistik |
| POST | `/statistics/monthly/calculate` | — | Monatsstatistik neu berechnen |
| GET | `/statistics/monthly/available` | — | verfügbare Monate |
| GET | `/report` | — | Tagesbericht als `DailyReportResponse` (JSON mit dem Berichtstext) |

---

## API-Tokens verwalten

Diese Endpunkte sind **nicht** per API-Token erreichbar — sie brauchen ein Login-JWT und
liegen unter `/api/users/me/api-tokens`. So kann ein Token keine weiteren Tokens erzeugen.

Response-Typen: `ApiToken`, `CreatedApiToken`.

| Methode | Pfad | Body | Zweck |
|---------|------|------|-------|
| GET | `/api/users/me/api-tokens` | — | eigene Tokens auflisten (ohne Secret) |
| POST | `/api/users/me/api-tokens` | `CreateApiTokenRequest` | Token anlegen → `CreatedApiToken` (Secret **einmalig**) |
| GET | `/api/users/me/api-tokens/{id}` | — | einzelnes Token |
| PUT | `/api/users/me/api-tokens/{id}` | `UpdateApiTokenRequest` | Name/`enabled`/`can_write` ändern |
| DELETE | `/api/users/me/api-tokens/{id}` | — | Token widerrufen (wirkt sofort) |

`CreateApiTokenRequest`:

```json
{ "household_id": "…uuid…", "name": "Nomi", "can_write": false }
```

Die `201`-Antwort ist die **einzige** Stelle mit dem Klartext-Secret:

```json
{ "data": {
    "token": { "id": "…", "household_id": "…", "user_id": "…", "name": "Nomi",
               "token_prefix": "hht_1a2b3c4d", "can_write": false, "enabled": true,
               "created_at": "…", "last_used_at": null },
    "secret": "hht_…"
} }
```

---

## Nicht per Token erreichbar

Diese liegen außerhalb von `/api/households/{hid}` und sind nur mit Login-JWT nutzbar:

- `GET /api/households` (alle eigenen Haushalte) und `POST /api/households` (Haushalt anlegen)
- alles unter `/api/users/…`, `/api/auth/…`, `/api/invitations/…`, `/api/dashboard/…`

---

## Beispiel

```bash
BASE=https://dein-host/api
TOKEN=hht_…

# Tagesbericht holen (Lesen)
curl "$BASE/households/$HID/report" -H "Authorization: Bearer $TOKEN"

# Aufgabe abhaken (braucht can_write)
curl -X POST "$BASE/households/$HID/tasks/$TID/complete" -H "Authorization: Bearer $TOKEN"

# Aufgabe anlegen (braucht can_write)
curl -X POST "$BASE/households/$HID/tasks" -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"title":"Wasser trinken","recurrence_type":"daily","target_count":8,"time_period":"day"}'
```
