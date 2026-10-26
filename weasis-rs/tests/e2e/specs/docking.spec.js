// Docked panels around the viewers (step 2.2.b).
const { test, expect } = require('@playwright/test');

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await expect(page.getByRole('menubar')).toBeVisible();
});

const panel = (page, side) => page.locator(`.w-dock-${side}`);

test('the explorer is docked left and the tools right', async ({ page }) => {
  await expect(panel(page, 'left').getByRole('tab')).toHaveText(['DICOM Explorer']);
  const tabs = panel(page, 'right').getByRole('tab');
  await expect(tabs).toHaveText(['Image Tools', 'Display']);
  await expect(tabs.first()).toHaveAttribute('aria-selected', 'true');
  await tabs.last().click();
  await expect(tabs.last()).toHaveAttribute('aria-selected', 'true');
  await page.screenshot({ path: 'test-results/docking.png' });
});

test('minimize puts a panel on the strip, a click docks it back', async ({ page }) => {
  await panel(page, 'left').getByTitle('Minimize').click();
  await expect(panel(page, 'left')).toHaveCount(0);
  await page.locator('.w-dock-strip-left').getByRole('button', { name: 'DICOM Explorer' }).click();
  await expect(panel(page, 'left').getByRole('tab')).toHaveText(['DICOM Explorer']);
});

test('the View menu closes and shows a panel', async ({ page }) => {
  await page.getByRole('button', { name: 'View' }).click();
  await page.getByRole('menuitem', { name: 'Tools' }).hover();
  const display = page.getByRole('menuitemcheckbox', { name: 'Display' });
  await expect(display).toHaveAttribute('aria-checked', 'true');
  await display.click();
  await expect(panel(page, 'right').getByRole('tab')).toHaveText(['Image Tools']);
  await page.getByRole('button', { name: 'View' }).click();
  await page.getByRole('menuitem', { name: 'Tools' }).hover();
  await expect(display).toHaveAttribute('aria-checked', 'false');
  await display.click();
  await expect(panel(page, 'right').getByRole('tab', { selected: true })).toHaveText('Display');
});

test('a tab dragged to the bottom moves there, and the layout is kept', async ({ page }) => {
  // The drop zones appear only once the drag started, so the drag is driven step by step.
  await panel(page, 'right').getByRole('tab', { name: 'Display' }).hover();
  await page.mouse.down();
  await page.mouse.move(600, 300, { steps: 5 });
  const zone = await page.locator('.w-drop-zone-bottom').boundingBox();
  await page.mouse.move(zone.x + zone.width / 2, zone.y + zone.height / 2, { steps: 5 });
  await page.mouse.up();
  await expect(panel(page, 'bottom').getByRole('tab')).toHaveText(['Display']);
  await page.reload();
  await expect(panel(page, 'bottom').getByRole('tab')).toHaveText(['Display']);
});

test('the splitter resizes a side within its limits', async ({ page }) => {
  const left = panel(page, 'left');
  const box = await left.locator('.w-dock-splitter').boundingBox();
  const y = box.y + box.height / 2;
  await page.mouse.move(box.x + 2, y);
  await page.mouse.down();
  await page.mouse.move(box.x + 102, y, { steps: 5 });
  await page.mouse.up();
  expect(Math.round((await left.boundingBox()).width)).toBe(400);
  await page.mouse.move(box.x + 102, y);
  await page.mouse.down();
  await page.mouse.move(0, y, { steps: 5 });
  await page.mouse.up();
  expect(Math.round((await left.boundingBox()).width)).toBe(160);
});
