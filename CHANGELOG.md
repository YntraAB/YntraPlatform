# Changelog

All notable changes to the **Yntra Platform** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned
- **Tauri 2.0 Desktop Shell**: Native installers (.msi, .dmg, .AppImage) with local file system access and system tray support.
- **Capacitor 6.0 Mobile Shell**: Native iOS & Android applications with push notifications and biometrics.
- **P2P Mesh Sync**: In-room WebRTC data channel synchronization for multi-terminal local networks.

---

## [1.2.0] - 2026-09-21

### Added
- **Obsidian & Zinc High-Contrast Theme Engine**:
  - Added full `<alpha-value>` support to `tailwind.config.js` across all semantic tokens (`background`, `card`, `secondary`, `border`, `primary`, `muted`), enabling precise opacity modulation classes (`border-border/40`, `bg-secondary/40`, `text-muted-foreground/70`).
  - Redesigned the dark theme in `src/index.css` from murky blue-slate to a deep, high-contrast obsidian and zinc palette:
    - Background Canvas: `hsl(240 5% 6%)` (`#0f0f12`)
    - Elevated Card Surfaces: `hsl(240 4% 11.5%)` (`#1c1c20`)
    - Secondary Shelves (Headers/Footers): `hsl(240 4% 16%)` (`#27272c`)
    - Sharp Borders & Hairlines: `hsl(240 5% 22%)` (`#36363d`)
    - Primary Button: Pure high-contrast editorial monochrome (`hsl(0 0% 98%)` with dark text in dark mode).
- **Universal Modal & Popup UI Standard**:
  - **Backdrop Contrast**: Upgraded `DialogOverlay` and `AlertDialogOverlay` to `bg-black/75 backdrop-blur-[3px]` for deep, focused contrast.
  - **Card Geometry**: Restrained `rounded-lg` (8px) card corners with `border border-border bg-card shadow-2xl`. Max `rounded-md` (6px) on inputs and buttons.
  - **Exact `h-12` (48px) Sidhuvud**: All modal headers standardized to `h-12 border-b border-border bg-secondary/40 px-5` with sober `text-xs font-medium text-foreground`.
  - **Avgränsad Sidfot**: Dedicated shelf footer with `border-t border-border bg-secondary/40 px-5 py-2.5` and compact `h-8` action buttons.
  - **Sunk Form Controls**: Inputs and selects embedded with `h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground` for crisp contrast against card background.
- **Zero Colored Circle Dots ("Inga Runda Cirklar")**:
  - Eliminated colored status/category circles (`size-2 rounded-full`) from dialog headers, title bars, and metadata rows.
  - Replaced circular multi-step indicators with sober text counters (e.g. `step / 2`).
- **Balanced Micro-Iconography Standard ("Mindre Ikoner, Inte På Allt")**:
  - Standardized popup header icons to delicate micro `size-3.5` (14px) with subtle `text-muted-foreground/70` opacity.
  - Standardized button action icons to compact `size-3` (12px, `mr-1.5`) for immediate visual orientation (Ta bort, Spara, Redigera, Skicka, Lägg till).
  - Anchored key structural data rows (Tid, Team, Tilldelad personal) with compact `size-3` icons, while keeping descriptions, labels, and text fields clean and free from icon spam.

### Changed
- **All Modals & Popups Refactored to Standard**:
  - `EventModal`: Complete overhaul of event detail popup—clean key-value hierarchy, micro-calendar header, micro-clock on time row, micro-icons for team and assignee, and calm text-first action buttons.
  - `AddEventModal`: Micro-calendar header, sunken form rows, and micro-check save button.
  - `ReportTimeDialog`: Compact `sm:max-w-[360px]` card, micro-clock header, and micro-check save button.
  - `AdminInviteModal`: Micro-shield header, sunken input, and micro-send button.
  - `ClientManagerModal`: Micro-user header, sunken input grid, and micro-check save button.
  - `TimeOffRequestModal`: Micro-calendar header, sunken textarea, and micro-send button.
  - `TeamManagerModal`: Micro-users header, text step counter `1 / 2`, micro chevron navigation, and micro-check submit button.
  - `RoleManagerModal`: Micro-shield header, micro-plus "Skapa ny roll", micro-trash and check action buttons.
  - `TwoFactorSettings`: Micro-key header, sunken QR code shelf, micro-shield check button.
  - `BlockSettings`: Dynamic module micro-icon in header.
  - `ImageCropperDialog`: Micro-crop header, micro-check save button, fixed Slider `onValueChange` prop.
  - `ReportList`: Micro-shield alert in header.

---

## [1.1.0] - 2026-09-21

### Added
- **Dynamic Breadcrumbs & Module-First Navigation**:
  - Eliminated the redundant root "Hem" (Home) prefix across all platform routes. Routes now immediately identify their root domain (e.g. `Team`, `Inkorg`, `Anteckningar`, `Inställningar`, `Tidshantering`, `Schema`).
  - Added smart drill-down breadcrumbs:
    - Directory: `Team > [Teamnamn] > [Medlem]` and `Team > Alla konton i organisationen > [Medlem]`.
    - Messages: `Inkorg > [Ämne]` (reading) and `Inkorg > [Nytt meddelande / Svara]` (composing).
    - Notes: `Anteckningar > [Teamnamn] > [Anteckning]`.
    - Settings: `Inställningar > [Aktiv flik (Allmänt, Konto, Moduler etc.)]`.
    - Time Management: `Tidshantering > [Team / Personal]`.
  - Added interactive breadcrumb navigation event (`yntra:breadcrumb-navigate`): clicking any root or parent step smoothly resets detail panes without page reload.
  - Standardized breadcrumb typography to crisp `text-[13px] font-medium` (active) and `text-[13px] font-normal text-muted-foreground/75` (links) with `size-3.5` separators.

### Changed
- **Platform-Wide UI Control Standardization (Settings Standard)**:
  - Scaled all inputs to compact `h-8` (32px), `px-2.5`, `py-1`, `text-xs`, `rounded-md`.
  - Standardized buttons to `h-8 px-3 text-xs font-medium` (default), `h-7 px-2.5 text-[11px]` (small), and `size-8` with `size-3.5` icons (icon button).
  - Standardized Select components: `SelectTrigger` `h-8 px-2.5 text-xs`, `SelectItem` `py-1.5 text-xs`.
  - Standardized textareas to `px-2.5 py-2 text-xs rounded-md`.
  - Standardized tabs to `TabsList h-8`, `TabsTrigger px-2.5 py-1 text-xs`.
  - Standardized dialogs to `DialogTitle text-base font-medium`, `DialogDescription text-xs`.
- **Topbars & View Headers Standardized to `h-12` (48px)**:
  - Lowered all view headers (Inkorg, Team, Anteckningar, Tidrapporter, Rapportering, Kalender, Inställningar, AppLayout header) from `h-14` (56px) down to exact `h-12` (48px), aligning them with list rows.
  - Replaced oversized titles with refined, light `text-xs font-medium text-foreground`.
- **Sidebar Layout & Ergonomic Spacing**:
  - Standardized sidebar header to `h-12 px-4`, aligning with the main topbar.
  - Grouped Search Trigger and TeamSwitcher into a cohesive container with `gap-2.5 px-3 pt-2.5 pb-2`.
  - Refined TeamSwitcher: removed outer margins, tightened label spacing to `space-y-1.5`, set label to `text-[11px] font-normal` with `h-3 w-3` Users icon.
  - Added subtle divider (`h-px bg-border/40`) before navigation items for clear visual hierarchy.
- **Inbox Reader & Composer Redesign**:
  - Message reader: compact back button (`h-8 text-xs font-medium`), subtle divider, truncated subject indicator, sender card with `size-8` avatar and `text-xs font-medium` name.
  - Message composer: contextual topbar title ("Svara", "Vidarebefordra", "Nytt meddelande"), light `text-xs font-normal` labels, `text-xs font-normal` textarea, compact quote box with remove button, and `Ctrl+Enter` shortcut to send.
- **Directory & Team Row Alignment**:
  - Aligned "Alla konton i organisationen" to exact same column width (`w-36 sm:w-48 md:w-56`) and typography (`text-sm font-medium` and `text-sm font-normal`) as the team rows below it, with subtle tinted background (`bg-secondary/30 dark:bg-muted/30`).
- **Dark Mode Color Refinements & Corner Radii**:
  - Eliminated stark pure white on white and harsh black in dark mode; applied balanced dark charcoal/gray tokens.
  - Restrained all corner radii to `rounded-md` (6px) or `rounded-lg` (8px).
- **Error Boundary Overhaul (Anti-AI Slop)**:
  - Removed garish red neon glowing bubble (`h-16 w-16 bg-destructive/10 text-destructive rounded-full`) and loud `text-2xl font-bold` heading.
  - Implemented quiet, minimalist Apple/Linear design: `size-8 rounded-md` neutral container with `h-4 w-4` Alert icon, `text-xs font-medium` title, `text-xs` description, subtle monospace code block, and compact `h-8` action buttons.

### Fixed
- Fixed runtime `ReferenceError: t is not defined` in `DirectoryPage.tsx` by importing `useTranslation`.
- Fixed runtime `ReferenceError: setCurrentLevel is not defined` in `DirectoryPage.tsx` by destructuring `setCurrentLevel`, `setSelectedTeam`, and `handleBreadcrumbClick` from `useDirectoryData()`.

---

## [1.0.0] - 2026-09-20

### Added
- **Platform Abstraction Layer (PAL)** (`src/platform/`):
  - Standardized cross-platform API for `storage`, `notifications`, `haptics`, `network`, and `appLifecycle`.
  - Native adapters for Web/PWA (`web.ts`), Desktop (`desktop.ts`), and Mobile (`mobile.ts`).
  - Reactive `usePlatform()` hook for UI components to query platform status without DOM coupling.
- **Antigravity `.agents/` Intelligence Environment**:
  - Global invariants and laws in `.agents/AGENTS.md`.
  - Contextual rule set in `.agents/rules/`: `architecture.md`, `cross-platform.md`, `modular-blocks.md`, `offline-and-sync.md`, `ui-design-system.md`, and `security-and-tenancy.md`.
  - Five progressive on-demand skills with YAML frontmatter:
    - `yntra-block-builder`: Runbook for creating and registering functional enterprise blocks.
    - `yntra-cross-platform`: Rules and guidelines for cross-platform safety.
    - `yntra-offline-first`: Optimistic UI mutations and local-first caching runbook.
    - `yntra-ui-crafting`: Aesthetics, dark mode, and micro-animation guidelines.
    - `yntra-i18n-localization`: Multi-language localization and translation runbook.
- **Comprehensive Documentation Suite (`docs/`)**:
  - `docs/ARCHITECTURE.md`: Complete system architecture specification with Mermaid diagrams.
  - `docs/CROSS_PLATFORM_STRATEGY.md`: Roadmap and setup guide for Web -> Desktop (Tauri) -> Mobile (Capacitor).
  - `docs/MODULAR_BLOCKS_GUIDE.md`: Step-by-step developer manual for building custom workspace blocks.
  - `docs/LOCAL_FIRST_AND_SYNC.md`: Local-first caching, optimistic UI, and deterministic conflict resolution.
  - `docs/AGENTS_SYSTEM_GUIDE.md`: Deep research and manual on Antigravity's progressive disclosure customization system.
  - `docs/DEVELOPER_WORKFLOW.md`: Setup commands, testing protocols, and contribution standards.
  - `docs/SECURITY_AND_COMPLIANCE.md`: Multi-tenancy isolation, RBAC role hierarchy, and RLS policies.
- **Modern Project Branding & Root Metadata**:
  - Updated `README.md` with visual architecture diagrams, getting started guide, and directory structure.
  - Updated `package.json` package identifier to `@yntra/platform`.

### Changed
- Refined modular block architecture: isolated plugins into self-contained feature modules with strict role guards.
- Enhanced breadcrumb and navigation resolution to support dynamic multi-lingual routing.

---

## [0.9.0-example] - 2026-04-15 (Archived in `EXAMPEL/`)

### Added
- Experimental Rust core (`yntra-core`) with embedded libSQL and `rkyv` zero-copy serialization.
- Experimental Dioxus 0.7 Web/Desktop client prototype (`yntra-ui`).
- UniFFI bindings generator prototype (`yntra-uniffi-bindgen`) for Swift and Kotlin.
- Initial criterion benchmark suite for quantitative database throughput.
