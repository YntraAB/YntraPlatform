import { test, expect } from '@playwright/test'

test.describe('Happy Path Flow', () => {
  test.beforeEach(async ({ page }) => {
    // Log all requests for debugging
    page.on('request', (request) => console.log('>>', request.method(), request.url()))
    page.on('response', (response) => console.log('<<', response.status(), response.url()))

    // Mock Supabase Auth Session
    await page.route('**/auth/v1/session*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          access_token: 'fake-token',
          token_type: 'bearer',
          expires_in: 3600,
          refresh_token: 'fake-refresh',
          user: {
            id: 'test-user-uuid',
            email: 'test@example.com',
            user_metadata: { role: 'admin' },
            app_metadata: { provider: 'google' },
          },
        }),
      })
    })

    // Mock Supabase User Data call
    await page.route('**/rest/v1/users*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'test-user-uuid',
          full_name: 'Test Admin',
          role: 'admin',
          email: 'test@example.com',
          workspace_id: 'ws-1',
        }),
      })
    })

    // Mock Supabase Workspace Data call
    await page.route('**/rest/v1/workspaces*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'ws-1',
          name: 'Test Workspace',
          modules_active: { assistance: true, school: true },
          settings: { language: 'en' },
        }),
      })
    })

    // Mock Team Members (Unread Notes/Permissions)
    await page.route('**/rest/v1/team_members*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([{ team_id: 'team-1', notes_last_read_at: new Date().toISOString() }]),
      })
    })

    // Mock Teams
    await page.route('**/rest/v1/teams*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([{ id: 'team-1', name: 'Test Team', workspace_id: 'ws-1' }]),
      })
    })

    // Mock Messages (Unread count)
    await page.route('**/rest/v1/messages*', async (route) => {
      // If it's a HEAD request for count
      if (route.request().method() === 'HEAD') {
        await route.fulfill({
          status: 200,
          headers: { 'content-range': '0-0/0' },
        })
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([]),
        })
      }
    })

    // Mock Notes
    await page.route('**/rest/v1/notes*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      })
    })

    // Navigate to the app
    await page.goto('/')
  })

  test('should show login page initially', async ({ page }) => {
    await expect(page.getByTestId('yntra-logo')).toBeVisible()
  })

  test('should navigate to schedule and inbox after login simulation', async ({ page }) => {
    // Manually set the session to bypass social login redirect
    await page.evaluate(() => {
      const session = {
        access_token: 'fake-token',
        token_type: 'bearer',
        expires_in: 3600,
        refresh_token: 'fake-refresh',
        user: {
          id: 'test-user-uuid',
          email: 'test@example.com',
          user_metadata: { role: 'admin' },
          app_metadata: { provider: 'google' },
          aud: 'authenticated',
          created_at: new Date().toISOString(),
        },
        expires_at: Math.floor(Date.now() / 1000) + 3600,
      }
      // We set both the common key and the project-specific key to be safe
      localStorage.setItem('sb-ileffmdueouhbooesjnv-auth-token', JSON.stringify(session))
      localStorage.setItem('supabase.auth.token', JSON.stringify(session))
    })

    await page.reload()

    // Give it a bit more time to process the session
    await page.waitForURL(/.*schedule/, { timeout: 10000 })

    // Verify we are on the schedule page
    await expect(page).toHaveURL(/.*schedule/)

    // Debug: What's on the page?
    const body = await page.innerHTML('body')
    console.log('Page Body Snapshot:', body.substring(0, 500))

    // Check if user name is displayed in the header
    await expect(page.getByText('Test Admin')).toBeVisible()

    // Navigate to Inbox
    await page.getByTestId('sidebar-item-inbox').click()
    await expect(page).toHaveURL(/.*inbox/)
  })
})
