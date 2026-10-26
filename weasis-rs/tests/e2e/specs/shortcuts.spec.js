// Keyboard shortcuts (step 2.3): Help > Shortcuts page and the docking shortcuts.
const { test, expect } = require('@playwright/test');

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await expect(page.getByRole('menubar')).toBeVisible();
});

test('Help > Shortcuts opens the page of the bindings in a new tab', async ({ page, context }) => {
  await page.getByRole('button', { name: 'Help' }).click();
  const [shortcuts] = await Promise.all([
    context.waitForEvent('page'),
    page.getByRole('menuitem', { name: 'Keyboard Shortcuts' }).click(),
  ]);
  await expect(shortcuts).toHaveTitle('Weasis Keyboard Shortcuts');
  await expect(shortcuts.getByRole('heading', { name: 'Pan' })).toBeVisible();
  const zoomIn = shortcuts.getByRole('row', { name: /Zoom in/ });
  await expect(zoomIn).toContainText('Ctrl+NumPad +');
  await shortcuts.screenshot({ path: 'test-results/shortcuts-page.png' });
});

test('Ctrl+M maximizes the viewer area and restores it', async ({ page }) => {
  await expect(page.locator('.w-dock')).toHaveCount(2);
  await page.keyboard.press('Control+m');
  await expect(page.locator('.w-main-window')).toHaveClass(/w-maximized/);
  await expect(page.locator('.w-dock').first()).toBeHidden();
  await page.keyboard.press('Control+m');
  await expect(page.locator('.w-dock').first()).toBeVisible();
});

test('Ctrl+Shift+E lists the panels and shows the chosen one', async ({ page }) => {
  await page.locator('.w-dock-left').getByTitle('Close').click();
  await expect(page.locator('.w-dock-left')).toHaveCount(0);
  await page.keyboard.press('Control+Shift+e');
  const list = page.getByRole('menu', { name: 'Open the docking panel list for selection' });
  await expect(list.getByRole('menuitem')).toHaveText(['DICOM Explorer', 'Image Tools', 'Display']);
  await list.getByRole('menuitem', { name: 'DICOM Explorer' }).click();
  await expect(list).toBeHidden();
  await expect(page.locator('.w-dock-left').getByRole('tab')).toHaveText(['DICOM Explorer']);
});
