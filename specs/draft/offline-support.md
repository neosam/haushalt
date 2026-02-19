# Offline Support (Draft)

> **Status:** Draft - Nicht implementiert
> **Erstellt:** 2026-02-19

## Übersicht

Read-only Offline-Unterstützung für die Household-App. Benutzer können alle Tasks ihrer Households auch ohne Internetverbindung sehen.

### Design-Entscheidungen

| Aspekt | Entscheidung |
|--------|--------------|
| **Scope** | Alle Household-Tasks (alle Households des Users) |
| **Aktionen** | Nur Anzeigen (read-only) |
| **Konflikte** | Server gewinnt - gecachte Daten werden bei Reconnect überschrieben |

---

## US-OFFLINE-001: Offline Task-Anzeige

**Als** Benutzer
**möchte ich** alle Tasks meiner Households auch ohne Internet sehen können
**damit** ich weiß, was zu tun ist, auch wenn ich offline bin

### Acceptance Criteria

- Alle Tasks aller Households des Users werden lokal gecached
- Bei Offline werden Tasks aus dem lokalen Cache geladen
- Task-Status (Completions, Streaks) wird mit gecached
- Household-Zuordnung bleibt erhalten
- Daten werden automatisch aktualisiert wenn wieder online

---

## US-OFFLINE-002: Offline-Indikator

**Als** Benutzer
**möchte ich** klar sehen, ob ich offline bin
**damit** ich weiß, dass meine Daten evtl. nicht aktuell sind

### Acceptance Criteria

- Sichtbarer Indikator (Banner oder Icon) zeigt Offline-Status
- Zeigt wann Daten zuletzt synchronisiert wurden (z.B. "Zuletzt aktualisiert: vor 2 Stunden")
- Indikator verschwindet automatisch bei Reconnect
- Optional: Warnung wenn Daten älter als X Stunden

---

## US-OFFLINE-003: Automatische Synchronisation

**Als** Benutzer
**möchte ich** dass meine Daten automatisch aktualisiert werden wenn ich wieder online bin
**damit** ich immer die aktuellen Daten sehe

### Acceptance Criteria

- Bei Reconnect werden alle Daten vom Server neu geladen
- Server-Daten überschreiben gecachte Daten (Server gewinnt)
- Gelöschte Tasks werden aus dem Cache entfernt
- Geänderte Tasks werden im Cache aktualisiert
- Neue Tasks werden dem Cache hinzugefügt

---

## US-OFFLINE-004: Offline-Einschränkungen

**Als** Benutzer
**möchte ich** verstehen, was ich offline nicht tun kann
**damit** ich keine Frustrationen erlebe

### Acceptance Criteria

- +/- Buttons sind offline deaktiviert oder versteckt
- Beim Versuch zu interagieren wird eine Meldung angezeigt: "Diese Aktion ist offline nicht verfügbar"
- Task-Erstellung ist offline nicht möglich
- Task-Bearbeitung ist offline nicht möglich
- Klar kommuniziert, dass Änderungen nur online möglich sind

---

## Architektur-Entscheidungen

### Storage

- **IndexedDB** für strukturierte Task-Daten
- Separate Stores für: Tasks, TaskWithStatus, Households
- Timestamp für "last_synced" pro Household

### Cache-Strategie

- Tasks werden bei jedem erfolgreichen API-Aufruf gecached
- Dashboard-Daten werden zusätzlich gecached
- Bei Offline: Fallback auf Cache
- Bei Online: Network-first, Cache als Fallback

### Daten-Freshness

- Keine automatische Invalidierung
- Bei Reconnect: Vollständiger Refresh
- UI zeigt "Zuletzt aktualisiert" Timestamp

---

## Out of Scope (für später)

- Offline Task-Completion (würde Sync-Queue und Konflikt-Handling erfordern)
- Offline Task-Erstellung
- Push Notifications für Sync-Status
- Selective Sync (nur bestimmte Households)
