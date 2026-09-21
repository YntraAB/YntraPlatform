# Yntra Platform

A modular workspace application for workforce coordination, team scheduling, time tracking, internal messaging, and compliance reporting. Built with React 19, TypeScript, Vite, and Tailwind CSS.

## Overview

Yntra provides operational tools for organizations that need an integrated internal workspace without running heavy ERP suites. Data access uses a local-first pattern through TanStack Query and IndexedDB (`idb-keyval`), giving instant UI updates while syncing changes in the background.

Hardware-specific capabilities (notifications, haptics, local storage, network monitoring) are separated through a Platform Abstraction Layer (`src/platform/`), allowing the same React interface to run as a web application (PWA), desktop app (Tauri), or mobile app (Capacitor).

## Core Modules

All functional modules are packaged as autonomous blocks registered in `src/lib/blocks/registry.ts`:

- **Scheduling**: Calendar views (day, week, month, agenda) with shift assignments, team filtering, and drag-and-drop planning.
- **Time Tracking**: Shift reporting, attest flows, break calculations, and SIE4 export for Swedish accounting software.
- **Messaging**: Team channels, direct messages, unread tracking, and shift handover notes.
- **Directory**: Staff profiles, team structuring, client assignments, and role-based permissions (`admin`, `user`, `assistant`, `client`).
- **Notes**: Operational logs and shift handovers with versioning and markdown support.
- **Reporting**: Compliance statistics, working hours summaries, and incident tracking.
- **Settings**: Workspace configuration, feature toggles per organization, two-factor authentication, and account preferences.

## Tech Stack

- **Frontend**: React 19, TypeScript, Vite, Tailwind CSS, Radix UI primitives
- **Data & Caching**: TanStack Query v5, IndexedDB (`idb-keyval`)
- **Backend**: Supabase (PostgreSQL with Row Level Security)
- **Localization**: i18next (Swedish and English)
- **Testing**: Vitest, React Testing Library, Playwright

## Quick Start

### Prerequisites

- Node.js 20+ or Bun 1.1+
- Git

### Installation

```bash
git clone https://github.com/YntraAB/YntraPlatform.git
cd YntraPlatform
npm install
npm run dev
```

The application runs on `http://localhost:5173`.

## Commands

| Command | Description |
| :--- | :--- |
| `npm run dev` | Starts the Vite development server |
| `npm run build` | Compiles TypeScript and builds production bundle |
| `npm run test:run` | Runs the Vitest test suite |
| `npm run check` | Runs linter and TypeScript typechecking |
| `npm run preview` | Previews the production build locally |

## Project Structure

```text
YntraPlatform/
├── docs/           Technical documentation and specifications
├── public/         Static assets and web manifest
├── src/
│   ├── components/ Shared UI primitives and layouts
│   ├── contexts/   Application contexts (auth, workspace, breadcrumbs)
│   ├── features/   Functional feature blocks (scheduler, time, messages, notes, etc.)
│   ├── hooks/      Shared React hooks
│   ├── i18n/       Swedish and English translation files
│   ├── lib/        Block registry, query client, Supabase client, utilities
│   ├── platform/   Platform Abstraction Layer (Web, Desktop, Mobile)
│   ├── services/   Data access and API services
│   └── types/      TypeScript type definitions and database schemas
├── supabase/       Database migrations and schema definitions
├── tests/          Test suites
├── CHANGELOG.md    Version history and release notes
└── LICENSE         MIT License
```

## Documentation

- [Architecture Specification](docs/ARCHITECTURE.md)
- [UI Design System & Tokens](docs/UI_DESIGN_SYSTEM.md)
- [Security & Multi-Tenancy](docs/SECURITY_AND_COMPLIANCE.md)
- [Local-First & Offline Sync](docs/LOCAL_FIRST_AND_SYNC.md)
- [Cross-Platform Strategy](docs/CROSS_PLATFORM_STRATEGY.md)
- [Developer Workflow](docs/DEVELOPER_WORKFLOW.md)
- [Changelog](CHANGELOG.md)

## License

MIT License. See [LICENSE](LICENSE) for details.
