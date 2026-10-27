// DICOM explorer (step 2.5): series selection with the mouse and the keyboard, series menu.
const path = require('path');
const { test, expect } = require('@playwright/test');

const testdata = (name) => path.join(__dirname, '../../../testdata', name);

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByRole('button', { name: 'File' }).click();
  await page.getByRole('menuitem', { name: 'Open' }).hover();
  const [chooser] = await Promise.all([
    page.waitForEvent('filechooser'),
    page.getByRole('menuitem', { name: 'DICOM' }).click(),
  ]);
  // One study: a CT series, then CR, US, and a PDF report shown last.
  await chooser.setFiles([
    testdata('synthetic/ct-series.zip'),
    testdata('synthetic/cr-monochrome1.dcm'),
    testdata('synthetic/us-regions.dcm'),
    testdata('synthetic/encapsulated-pdf.dcm'),
  ]);
  await expect(series(page)).toHaveCount(4);
});

const series = (page) => page.locator('.w-series-pane');
const pane = (page, name) => series(page).filter({ hasText: name });
const selected = (page) => page.locator('.w-series-pane[aria-selected="true"]');

test('series are shown in order and selected with the mouse', async ({ page }) => {
  await expect(series(page)).toHaveText(['CT sphere phantom', 'CR MONOCHROME1 gradient', 'US regions', 'PDF report']);
  await pane(page, 'CR').click();
  await expect(selected(page)).toHaveText(['CR MONOCHROME1 gradient']);
  await pane(page, 'US').click({ modifiers: ['Shift'] });
  await expect(selected(page)).toHaveCount(2);
  await pane(page, 'CT').click({ modifiers: ['Control'] });
  await expect(selected(page)).toHaveCount(3);
  await pane(page, 'CR').click({ modifiers: ['Control'] });
  await expect(selected(page)).toHaveText(['CT sphere phantom', 'US regions']);
  await page.screenshot({ path: 'test-results/explorer-selection.png' });
});

test('the explorer shortcuts move the selection', async ({ page }) => {
  await pane(page, 'CR').click();
  await page.keyboard.press('ArrowDown');
  await expect(selected(page)).toHaveText(['US regions']);
  await page.keyboard.press('End');
  await expect(selected(page)).toHaveText(['PDF report']);
  await page.keyboard.press('ArrowDown');
  await expect(selected(page)).toHaveText(['CT sphere phantom'], { message: 'goes round' });
  await page.keyboard.press('Control+a');
  await expect(selected(page)).toHaveCount(4);
});

test('the series menu removes the selected series', async ({ page }) => {
  await pane(page, 'US').click();
  await pane(page, 'US').click({ button: 'right' });
  const menu = page.getByRole('menu');
  await expect(menu.getByRole('menuitem')).toHaveText([
    'Select related Series',
    'Select related Series (same axis)',
    'Remove selected Series',
    'Remove this Study',
    'Remove this Patient',
  ]);
  await menu.getByRole('menuitem', { name: 'Remove selected Series' }).click();
  await expect(series(page)).toHaveText(['CT sphere phantom', 'CR MONOCHROME1 gradient', 'PDF report']);
  await pane(page, 'CR').click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Remove this Patient' }).click();
  await expect(series(page)).toHaveCount(0);
});
