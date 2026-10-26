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

const groups = (page) => page.locator('.w-tab-group');

test('a tab split to the right opens a new group, which can be resized', async ({ page }) => {
  await tab(page, 'Viewer 2').click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Split Right' }).click();
  await expect(groups(page)).toHaveCount(2);
  await expect(groups(page).first().getByRole('tab')).toHaveText(['Viewer 1', 'Viewer 3']);
  await expect(groups(page).last().getByRole('tab')).toHaveText(['Viewer 2']);
  await expect(groups(page).last()).toHaveClass(/w-focused/);
  // A group of a single tab cannot be split again.
  await tab(page, 'Viewer 2').click({ button: 'right' });
  await expect(page.getByRole('menuitem', { name: 'Split Down' })).toHaveAttribute('aria-disabled', 'true');
  await page.locator('.w-backdrop').click();
  const divider = await page.locator('.w-split-divider').boundingBox();
  const before = (await groups(page).first().boundingBox()).width;
  await page.mouse.move(divider.x + 2, divider.y + 50);
  await page.mouse.down();
  await page.mouse.move(divider.x - 98, divider.y + 50, { steps: 5 });
  await page.mouse.up();
  expect((await groups(page).first().boundingBox()).width).toBeLessThan(before - 80);
  await page.screenshot({ path: 'test-results/viewer-groups.png' });
});

test('moving the last tab of a group into another removes the split', async ({ page }) => {
  await tab(page, 'Viewer 3').click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Split Down' }).click();
  await tab(page, 'Viewer 3').click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Move to Other Group' }).hover();
  await page.getByRole('menuitem', { name: 'Viewer 1, Viewer 2' }).click();
  await expect(groups(page)).toHaveCount(1);
  await expect(page.locator('.w-split')).toHaveCount(0);
  await expect(page.getByRole('tab')).toHaveText(['Viewer 1', 'Viewer 2', 'Viewer 3']);
  await expect(tab(page, 'Viewer 3')).toHaveAttribute('aria-selected', 'true');
});
