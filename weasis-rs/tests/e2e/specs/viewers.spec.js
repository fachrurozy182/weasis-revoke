// Viewer tabs (step 2.2.c), on the gallery page until the explorers open viewers.
const { test, expect } = require('@playwright/test');

test.beforeEach(async ({ page }) => {
  await page.goto('/#gallery');
  for (let i = 0; i < 3; i++) {
    await page.getByRole('button', { name: 'Open viewer' }).click();
  }
});

const tab = (page, name) => page.getByRole('tab', { name });

test('an opened viewer is selected and the others keep their content', async ({ page }) => {
  await expect(page.getByRole('tab')).toHaveText(['Viewer 1', 'Viewer 2', 'Viewer 3']);
  await expect(tab(page, 'Viewer 3')).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('tabpanel')).toHaveText('Viewer 3');
  await expect(page.locator('.w-viewer')).toHaveCount(3);
  await page.screenshot({ path: 'test-results/viewers.png' });
});

test('closing a tab shows the previously selected one', async ({ page }) => {
  await tab(page, 'Viewer 1').click();
  await tab(page, 'Viewer 3').click();
  await tab(page, 'Viewer 3').getByTitle('Close').click();
  await expect(page.getByRole('tab')).toHaveText(['Viewer 1', 'Viewer 2']);
  await expect(tab(page, 'Viewer 1')).toHaveAttribute('aria-selected', 'true');
});

test('the tab menu closes the other tabs or all of them', async ({ page }) => {
  await tab(page, 'Viewer 2').click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Close Others' }).click();
  await expect(page.getByRole('tab')).toHaveText(['Viewer 2']);
  await tab(page, 'Viewer 2').click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Close All' }).click();
  await expect(page.getByRole('tab')).toHaveCount(0);
  await expect(page.locator('.w-viewer')).toHaveCount(0);
});
