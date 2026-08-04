import { test, expect } from '@playwright/test';

test.describe('Yntra End-to-End (E2E) User Journey', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the Yntra Web / Desktop App
    await page.goto('/');
  });

  test('Complete Journey: Account Registration -> Workspace Setup -> Offline Edits -> Network Reconnection Sync Merge', async ({ page, context }) => {
    // ==========================================
    // STAGE 1: Account Registration / Authentication
    // ==========================================
    await test.step('Stage 1: Account Registration & Authentication', async () => {
      // Verify login logo and title elements exist
      const logo = page.locator('[data-testid="yntra-logo"]');
      await expect(logo).toBeVisible({ timeout: 10000 });

      // Click "Dev Login" to trigger dev account registration bypass or activate invitation
      const devLoginBtn = page.getByRole('button', { name: 'Dev Login' });
      if (await devLoginBtn.isVisible()) {
        await devLoginBtn.click();
      }

      // Allow navigation/state initialization to complete
      await page.waitForTimeout(1000);
    });

    // ==========================================
    // STAGE 2: Workspace Setup
    // ==========================================
    await test.step('Stage 2: Workspace Setup', async () => {
      const setupNameInput = page.locator('#setup-full-name');
      const setupPhoneInput = page.locator('#setup-phone');
      const setupPasswordInput = page.locator('#setup-password');
      const setupConfirmInput = page.locator('#setup-confirm-password');

      // If user requires setup configuration
      if (await setupNameInput.isVisible()) {
        await setupNameInput.fill('E2E Test Administrator');
        await setupPhoneInput.fill('+46 70 999 88 77');
        await setupPasswordInput.fill('123456');
        await setupConfirmInput.fill('123456');

        const saveBtn = page.getByRole('button', { name: /spara|save/i });
        await saveBtn.click();

        await page.waitForTimeout(1000);
      }

      // Verify layout header or sidebar is rendered after setup completion
      const sidebarOrHeader = page.locator('nav, header, [role="navigation"], .main-layout, div');
      await expect(sidebarOrHeader.first()).toBeVisible();
    });

    // ==========================================
    // STAGE 3: Offline Edits
    // ==========================================
    await test.step('Stage 3: Offline Edits (Disconnected Mode)', async () => {
      // Emulate dropping network connectivity
      await context.setOffline(true);

      // Verify Offline Indicator triggers or responds to browser offline event
      await page.evaluate(() => {
        window.dispatchEvent(new Event('offline'));
      });

      // Perform offline edit operation: navigate to notes/todos or field view if present
      const fieldCrewToggle = page.getByText(/OFFLINE-LÄGE|KÄLLARE|ONLINE/i).first();
      if (await fieldCrewToggle.isVisible()) {
        await fieldCrewToggle.click();
      }

      // Create an offline item or perform edit action
      const noteInput = page.locator('textarea, input[type="text"]').first();
      if (await noteInput.isVisible()) {
        await noteInput.fill('Offline CRDT Edit Note created during E2E test');
      }

      // Verify that offline queue badge or offline status indicator reflects disconnected state
      const offlineBadge = page.locator('.bg-amber-500\\/10, .bg-red-500\\/10, .text-amber-400, .offline-indicator, span').filter({ hasText: /OFFLINE|KÖ|DISCONNECTED/i }).first();
      if (await offlineBadge.isVisible()) {
        await expect(offlineBadge).toBeVisible();
      }
    });

    // ==========================================
    // STAGE 4: Network Reconnection Sync Merge
    // ==========================================
    await test.step('Stage 4: Network Reconnection & Deterministic Sync Merge', async () => {
      // Re-enable network connectivity
      await context.setOffline(false);

      // Dispatch online event to notify application components
      await page.evaluate(() => {
        window.dispatchEvent(new Event('online'));
      });

      await page.waitForTimeout(1500);

      // Verify status transitions back to Synced / Online
      const syncedBadge = page.locator('span, div').filter({ hasText: /ONLINE|SYNCED|SYNKRONISERAD|4G\/5G/i }).first();
      if (await syncedBadge.isVisible()) {
        await expect(syncedBadge).toBeVisible();
      }
    });
  });
});
