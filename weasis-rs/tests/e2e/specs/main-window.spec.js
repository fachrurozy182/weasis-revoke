// Main window menus (step 2.2.a).
const { test, expect } = require('@playwright/test');

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('menubar')).toBeVisible();
});

test('the menu bar has the menus of the desktop application', async ({ page }) => {
  const titles = page.getByRole('menubar').locator('.w-menubar-title');
  await expect(titles).toHaveText(['File', 'View', 'Tools', 'Help']);
});

test('a click opens a menu, hovering switches, Escape closes', async ({ page }) => {
  await page.getByRole('button', { name: 'File' }).click();
  const file = page.getByRole('menu', { name: 'File' });
  await expect(file.getByRole('menuitem', { name: /Preferences/ })).toContainText('Alt+P');
  await page.getByRole('button', { name: 'Help' }).hover();
  await expect(page.getByRole('menu', { name: 'Help' })).toBeVisible();
  await expect(file).toBeHidden();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('menu', { name: 'Help' })).toBeHidden();
});

test('the anonymize item is a check box', async ({ page }) => {
  await page.getByRole('button', { name: 'Tools' }).click();
  const item = page.getByRole('menuitemcheckbox', { name: 'Anonymize displayed data' });
  await expect(item).toHaveAttribute('aria-checked', 'false');
  await item.click();
  await page.getByRole('button', { name: 'Tools' }).click();
  await expect(item).toHaveAttribute('aria-checked', 'true');
  await page.screenshot({ path: 'test-results/main-window.png' });
  // A click outside closes the menu.
  await page.mouse.click(600, 500);
  await expect(page.getByRole('menu', { name: 'Tools' })).toBeHidden();
});

test('empty submenus are disabled until explorers fill them', async ({ page }) => {
  await page.getByRole('button', { name: 'File' }).click();
  await expect(page.getByRole('menuitem', { name: 'Import' })).toHaveAttribute('aria-disabled', 'true');
});

test('anonymizing shows the masking badge and the title suffix', async ({ page }) => {
  await expect(page).toHaveTitle(/^Weasis v[\d.]+$/);
  await page.getByRole('button', { name: 'Tools' }).click();
  await page.getByRole('menuitemcheckbox', { name: 'Anonymize displayed data' }).click();
  const badge = page.getByRole('button', { name: 'Anonymized display' });
  await expect(badge).toBeVisible();
  await expect(page).toHaveTitle(/ — Anonymized display$/);
  await badge.click();
  await page.getByRole('menuitem', { name: 'Turn off anonymization' }).click();
  await expect(badge).toBeHidden();
  await page.getByRole('button', { name: 'Tools' }).click();
  await expect(page.getByRole('menuitemcheckbox', { name: 'Anonymize displayed data' })).toHaveAttribute('aria-checked', 'false');
});
