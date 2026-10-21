import { expect, test } from '@playwright/test';

// A module that throws at load time leaves the window blank. Fail on any page error.
test.beforeEach(({ page }) => {
  page.on('pageerror', (error) => {
    throw error;
  });
});

test('Overview renders in both colour schemes', async ({ page }) => {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    await page.goto('/');
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('Playbook');
    await expect(page.getByRole('navigation', { name: 'Workspaces' })).toBeVisible();
    await expect(page.getByRole('complementary', { name: 'What’s what' })).toBeVisible();
    await page.screenshot({ path: `test-results/overview-${colorScheme}.png` });
  }
});

test('Lauf sidebar nests projects under groups, with Knowledge', async ({ page }) => {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    await page.goto('/');
    await page.locator('main').click();
    await page.keyboard.press('Meta+2');
    const clients = page.getByRole('region', { name: 'Clients' });
    await expect(clients.getByText('stoc', { exact: true })).toBeVisible();
    await clients.getByText('stoc', { exact: true }).hover();
    await page.waitForTimeout(300);
    await page.screenshot({ path: `test-results/sidebar-lauf-${colorScheme}.png` });
    await page.getByRole('button', { name: 'Knowledge' }).click();
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('Knowledge');
    await page.screenshot({ path: `test-results/knowledge-${colorScheme}.png` });
  }
});

test('⌘2 switches to Lauf', async ({ page }) => {
  await page.goto('/');
  await page.locator('main').click();
  await page.keyboard.press('Meta+2');
  await expect(page.getByTestId('window')).toHaveAttribute('data-workspace', 'lauf');
  await page.waitForTimeout(400); // let the tint transition settle for the screenshot
  await page.screenshot({ path: 'test-results/hello-lauf.png' });
});

test('⌘K palette finds and runs a command', async ({ page }) => {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    await page.goto('/');
    await page.locator('main').click();
    await page.keyboard.press('Meta+k');
    const input = page.getByRole('combobox');
    await expect(input).toBeFocused();
    await page.waitForTimeout(300);
    await page.screenshot({ path: `test-results/palette-${colorScheme}.png` });
    await input.fill('lauf');
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('window')).toHaveAttribute('data-workspace', 'lauf');
    await expect(page.getByRole('combobox')).toHaveCount(0);
  }
});
