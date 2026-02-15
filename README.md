# Household Manager

A full-stack Rust application for household task management with role-based access control, points/rewards system, and real-time updates.

## Tech Stack

- **Backend**: Rust with Actix-web, SQLx, SQLite
- **Frontend**: Rust with Leptos (WASM)
- **Authentication**: JWT tokens with refresh token rotation
- **Real-time**: WebSocket support for live updates

## Features

- Multi-household support with invitations
- Role-based permissions (Owner, Admin, Member)
- Customizable role labels per household
- Task scheduling (daily, weekly, monthly, custom dates)
- Points and rewards system
- Announcements and chat
- Activity logging
- PWA support with offline capabilities
- Internationalization (English, German)

## Development Setup

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled

### Development Shell

```bash
nix develop
```

This provides all necessary tools:
- Rust toolchain with WASM target
- SQLx CLI
- Trunk (frontend dev server)
- wasm-bindgen-cli
- binaryen (wasm-opt)

### Running Locally

**Backend:**
```bash
cargo run -p backend
```

**Frontend (dev server with hot reload):**
```bash
cd frontend && trunk serve
```

The frontend dev server proxies API requests to `http://127.0.0.1:8080`.

### Running Tests

```bash
cargo test --workspace
```

## Building for Production

### Using Nix Flakes

**Build backend:**
```bash
nix build .#backend
# or
nix build .  # default is backend
```

**Build frontend:**
```bash
nix build .#frontend
```

### Using nix-build

**Backend:**
```bash
nix-build
```

**Frontend:**
```bash
nix-build frontend/default.nix
```

### Build Outputs

**Backend** (`result/`):
- `bin/backend` - The server binary
- `bin/start.sh` - Start script with automatic DB migration
- `migrations/` - Database migration files

**Frontend** (`result/`):
- `index.html` - Entry point
- `frontend.js` - WASM loader
- `frontend_bg.wasm` - Compiled WebAssembly
- `styles.css` - Stylesheet
- `manifest.json`, `sw.js` - PWA files

## Deployment

### Backend

```bash
# Set environment variables
export DATABASE_URL="sqlite:///path/to/household.db"
export JWT_SECRET="your-secret-key"

# Run with automatic migration
./result/bin/start.sh

# Or run directly (requires manual migration)
./result/bin/backend
```

### Frontend

Serve the frontend build output with any static file server (nginx, caddy, etc.). Configure reverse proxy for `/api/` requests to the backend.

Example nginx configuration:
```nginx
server {
    listen 80;
    root /path/to/frontend/build;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

## Project Structure

```
.
├── backend/           # Actix-web server
│   ├── src/
│   │   ├── handlers/  # HTTP request handlers
│   │   ├── models/    # Database models
│   │   ├── services/  # Business logic
│   │   └── middleware/# Auth, rate limiting
│   └── migrations/    # SQLx migrations
├── frontend/          # Leptos WASM app
│   ├── src/
│   │   ├── pages/     # Page components
│   │   ├── components/# Reusable components
│   │   ├── api/       # Backend API client
│   │   └── i18n/      # Translations
│   └── index.html
├── shared/            # Shared types between frontend/backend
├── default.nix        # Backend nix build
├── flake.nix          # Nix flake configuration
└── Cargo.toml         # Workspace configuration
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | SQLite database path | `sqlite://household.db` |
| `JWT_SECRET` | Secret for JWT signing | Required |
| `HOST` | Server bind address | `127.0.0.1` |
| `PORT` | Server port | `8080` |

## License

MIT
