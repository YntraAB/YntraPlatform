# Developer Guide: Modular Blocks Engine

The Yntra Platform is designed as a **dynamic, composable workspace**. Rather than hardcoded monoliths, applications are composed of autonomous functional blocks.

---

## 1. What is a Functional Block?

A **Block** is a self-contained feature module that provides:
1. **Routes**: View endpoints mapped to React components.
2. **Navigation Items**: Sidebar and header links with badge counters and icons.
3. **Role Permissions**: Which roles (`platform_admin`, `admin`, `user`, etc.) can access the block.
4. **Tenant Configuration**: Whether the block is enabled or disabled in the current workspace.

---

## 2. Core Block Registry

All active blocks are registered statically in `src/lib/blocks/registry.ts`:

```typescript
export const BLOCK_REGISTRY: Record<string, BlockDefinition> = {
  dashboard: dashboardPlugin,
  messaging: messagingPlugin,
  scheduling: schedulingPlugin,
  notes: notesPlugin,
  directory: directoryPlugin,
  assistance: assistancePlugin,
  time: timePlugin,
  reporting: reportingPlugin,
}
```

---

## 3. How to Create a New Block (5 Simple Steps)

### Step 1: Create Feature Directory
Create `src/features/<block-id>/`:
```text
src/features/inventory/
├── components/
│   ├── InventoryPage.tsx
│   └── InventoryList.tsx
└── index.ts
```

### Step 2: Define Feature Routes and Navigation
In `src/features/inventory/index.ts`:
```typescript
import { lazy } from 'react'
import type { BlockDefinition } from '@/lib/blocks/registry'

const InventoryPage = lazy(() => import('./components/InventoryPage'))

export const inventoryPlugin: BlockDefinition = {
  id: 'inventory',
  routes: [
    {
      path: '/inventory',
      component: InventoryPage,
      allowedRoles: ['platform_admin', 'admin', 'user'],
      breadcrumbKey: 'sidebar.sections.inventory',
    },
  ],
  navigation: [
    {
      id: 'inventory',
      labelKey: 'sidebar.sections.inventory',
      path: '/inventory',
      icon: 'archive', // Registered in src/lib/blocks/icons.ts
      section: 'tools', // 'main' | 'tools' | 'admin'
      allowedRoles: ['platform_admin', 'admin', 'user'],
    },
  ],
}
```

### Step 3: Register in `src/lib/blocks/registry.ts`
```typescript
import { inventoryPlugin } from '@/features/inventory'

export const BLOCK_REGISTRY: Record<string, BlockDefinition> = {
  // ...
  inventory: inventoryPlugin,
}
```

### Step 4: Add Translation Strings
Add entries to `src/i18n/locales/en.json` and `src/i18n/locales/sv.json`:
```json
{
  "sidebar": {
    "sections": {
      "inventory": "Lager & Utrustning"
    }
  }
}
```

### Step 5: Test Deactivation & Guards
Test with `BlockGuard`: if the workspace turns off `inventory`, visiting `/inventory` displays a clean deactivated notice rather than an uncaught route error.
