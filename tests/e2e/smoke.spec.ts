import { expect, test } from '@playwright/test';

test('hello window renders in both colour schemes', async ({ page }) => {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    await page.goto('/');
    await expect(page.getByRole('heading', { level: 1 })).toContainText('Every identity');
    await expect(page.getByRole('navigation', { name: 'Workspaces' })).toBeVisible();
    await page.screenshot({ path: `test-results/hello-${colorScheme}.png` });
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
