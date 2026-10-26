// Base components (step 2.1.b), on the gallery page.
const { test, expect } = require('@playwright/test');

test.beforeEach(async ({ page }) => {
  await page.goto('/#gallery');
  await expect(page.getByTestId('state')).toBeVisible();
});

test('the theme colors the page', async ({ page }) => {
  const background = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(background).toBe('rgb(45, 42, 46)');
});

test('buttons and toggle buttons change the state', async ({ page }) => {
  await page.getByRole('button', { name: 'Press' }).click();
  await page.getByRole('button', { name: 'Toggle' }).click();
  await expect(page.getByTestId('state')).toHaveText('pressed 1 | toggle true | layout 1x1');
  await expect(page.getByRole('button', { name: 'Toggle' })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('button', { name: 'Disabled' })).toBeDisabled();
});

test('the drop-down button selects a choice and closes', async ({ page }) => {
  await page.getByRole('button', { name: '1x1' }).click();
  await expect(page.getByRole('menu')).toBeVisible();
  await page.getByRole('menuitemradio', { name: '2x2' }).click();
  await expect(page.getByRole('menu')).toBeHidden();
  await expect(page.getByTestId('state')).toContainText('layout 2x2');
});

test('the collapsible panel folds its slider', async ({ page }) => {
  const slider = page.getByRole('slider', { name: 'Zoom' });
  await slider.fill('200');
  await expect(page.getByText('Zoom: 200')).toBeVisible();
  await page.getByRole('button', { name: 'Zoom' }).click();
  await expect(slider).toBeHidden();
});

test('the dialog opens and closes', async ({ page }) => {
  await page.getByRole('button', { name: 'Open dialog' }).click();
  const dialog = page.getByRole('dialog', { name: 'About Weasis' });
  await expect(dialog).toBeVisible();
  await page.screenshot({ path: 'test-results/gallery.png' });
  await dialog.getByRole('button', { name: 'OK' }).click();
  await expect(dialog).toBeHidden();
  // The title bar button closes it too.
  await page.getByRole('button', { name: 'Open dialog' }).click();
  await dialog.getByRole('button', { name: 'Close' }).click();
  await expect(dialog).toBeHidden();
});
