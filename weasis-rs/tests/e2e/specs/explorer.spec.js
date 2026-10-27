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
const selected = (page) => page.locator('.w-series-pane[aria-selected="true"] .w-series-label');

test('series are shown in order and selected with the mouse', async ({ page }) => {
  await expect(series(page).locator('.w-series-label')).toHaveText(['CT sphere phantom', 'CR MONOCHROME1 gradient', 'US regions', 'PDF report']);
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
  await expect(series(page).locator('.w-series-label')).toHaveText(['CT sphere phantom', 'CR MONOCHROME1 gradient', 'PDF report']);
  await pane(page, 'CR').click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Remove this Patient' }).click();
  await expect(series(page)).toHaveCount(0);
});

test('series show their thumbnail, image count and kind', async ({ page }) => {
  const ct = pane(page, 'CT');
  await expect(ct.locator('img')).toHaveAttribute('src', /^data:image\/jpeg;base64,/);
  await expect(ct.locator('.w-thumbnail-count')).toHaveText('16');
  await expect(pane(page, 'CR').locator('img')).toHaveCount(1);
  // A document has no image: its kind is shown.
  await expect(pane(page, 'PDF').locator('.w-thumbnail-kind')).toHaveText('PDF');
  // Each series gets its own image: the US image is uniform, the CR image a gradient.
  const grays = (name) => pane(page, name).locator('img').evaluate((img) => {
    const canvas = document.createElement('canvas');
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const context = canvas.getContext('2d');
    context.drawImage(img, 0, 0);
    const row = context.getImageData(0, canvas.height >> 1, canvas.width, 1).data;
    const values = new Set();
    for (let i = 0; i < row.length; i += 4) values.add(row[i] >> 3);
    return values.size;
  });
  await expect(pane(page, 'US').locator('img')).toHaveCount(1);
  expect(await grays('US')).toBeLessThanOrEqual(2);
  expect(await grays('CR')).toBeGreaterThan(10);
  const size = await ct.locator('img').evaluate((img) => [img.naturalWidth, img.naturalHeight]);
  expect(Math.max(...size)).toBeLessThanOrEqual(256);
  await page.screenshot({ path: 'test-results/thumbnails.png' });
});

test('the thumbnail is rebuilt from another image', async ({ page }) => {
  const ct = pane(page, 'CT');
  const image = ct.locator('img');
  await expect(image).toHaveAttribute('src', /^data:image\/jpeg/);
  const middle = await image.getAttribute('src');
  await ct.click();
  await ct.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Rebuild Thumbnail' }).hover();
  await page.getByRole('menuitem', { name: 'From first image' }).click();
  // The first slice of the sphere phantom differs from the middle one.
  await expect(image).not.toHaveAttribute('src', middle);
});
