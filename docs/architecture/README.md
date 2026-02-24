# Architecture Documentation

This directory contains Mermaid diagrams documenting the system architecture.

## Contents

| File | Description |
|------|-------------|
| [01-system-overview.md](01-system-overview.md) | High-level system context, deployment, and technology stack |
| [02-backend-components.md](02-backend-components.md) | Backend layered architecture, handlers, services, middleware |
| [03-frontend-components.md](03-frontend-components.md) | Frontend component structure, routing, state management |
| [04-database-schema.md](04-database-schema.md) | Complete ER diagram with all 23 tables and relationships |
| [05-authentication-flow.md](05-authentication-flow.md) | Registration, login, token refresh, logout sequences |
| [06-task-flow.md](06-task-flow.md) | Task completion, review, points calculation, recurrence |
| [07-api-structure.md](07-api-structure.md) | REST API endpoints, WebSocket protocol, error handling |
| [08-roles-permissions.md](08-roles-permissions.md) | Role hierarchy, permission matrix, hierarchy types |
| [09-gamification-flow.md](09-gamification-flow.md) | Points, rewards, punishments, streaks, leaderboard |

## Viewing Diagrams

These diagrams use [Mermaid](https://mermaid.js.org/) syntax. To render them:

- **GitHub/GitLab**: Diagrams render automatically in markdown preview
- **VS Code**: Install "Markdown Preview Mermaid Support" extension
- **CLI**: Use `mmdc` (Mermaid CLI) to generate images
- **Online**: Paste into [mermaid.live](https://mermaid.live)

## Diagram Types Used

- **flowchart**: System components and data flow
- **sequenceDiagram**: Request/response sequences
- **erDiagram**: Database entity relationships
- **C4Context**: System context diagrams
- **mindmap**: Technology stack overview
