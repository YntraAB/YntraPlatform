# Developer Workflow & Command Reference

This document outlines local environment setup, everyday development workflows, testing, and contribution protocols for Yntra Platform.

---

## 1. Prerequisites

- **Node.js**: `v20.0.0+` (or Bun `v1.1+`)
- **Package Manager**: `npm` or `bun`
- **Git**

---

## 2. Common Commands

| Command | Bun Equivalent | Description |
| :--- | :--- | :--- |
| `npm run dev` | `bun run dev` | Starts the Vite development server with HMR on `http://localhost:5173` |
| `npm run build` | `bun run build` | Runs TypeScript compilation (`tsc -b`) and generates optimized bundle in `dist/` |
| `npm run check` | `bun run check` | Runs linter and TypeScript typecheck without emitting files |
| `npm run test` | `bun run test` | Runs Vitest unit and integration test suite |
| `npm run test:e2e`| `bun run test:e2e` | Runs Playwright end-to-end browser tests |
| `npm run format`| `bun run format`| Formats codebase using Prettier with Tailwind class sorting |

---

## 3. Environment Configuration

Copy `.env.example` to `.env`:
```bash
cp .env.example .env
```

Key environment variables:
- `VITE_SUPABASE_URL`: Remote or local Supabase project URL.
- `VITE_SUPABASE_ANON_KEY`: Public anonymous client key for row-level security queries.
- `VITE_APP_ENV`: `development` | `staging` | `production`.

---

## 4. Git & Contribution Workflow

Follow standard Git branching and Conventional Commits for code contributions:

1. Create a descriptive feature branch from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. Format and verify your changes:
   ```bash
   npm run format
   npm run check
   npm run test
   ```

3. Commit changes using imperative, concise messages:
   ```bash
   git commit -m "feat(scheduler): handle daylight saving offset in roster"
   ```

4. Push your branch and open a Pull Request against `main`.

