// Local import (step 2.4): File > Open > DICOM reads the files in the import worker.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('@playwright/test');

const testdata = (name) => path.join(__dirname, '../../../testdata', name);

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await expect(page.getByRole('menubar')).toBeVisible();
});

async function openFiles(page, files) {
  await page.getByRole('button', { name: 'File' }).click();
  await page.getByRole('menuitem', { name: 'Open' }).hover();
  const [chooser] = await Promise.all([
    page.waitForEvent('filechooser'),
    page.getByRole('menuitem', { name: 'DICOM' }).click(),
  ]);
  await chooser.setFiles(files);
}

test('opened DICOM files appear in the explorer', async ({ page }) => {
  await openFiles(page, [testdata('pydicom/CT_small.dcm'), testdata('pydicom/MR_small.dcm')]);
  const patients = page.locator('.w-patient-list li');
  await expect(patients).toHaveText([/CompressedSamples\^CT1 — 1 series/, /CompressedSamples\^MR1 — 1 series/]);
  await expect(page.getByRole('status')).toHaveCount(0);
  await page.screenshot({ path: 'test-results/import.png' });
});

test('unreadable files are reported at the end', async ({ page }) => {
  // Paths and buffers cannot be mixed: both files are given as buffers.
  const ct = fs.readFileSync(testdata('pydicom/CT_small.dcm'));
  await openFiles(page, [
    { name: 'CT_small.dcm', mimeType: 'application/dicom', buffer: ct },
    { name: 'broken.dcm', mimeType: 'application/dicom', buffer: Buffer.from('not a DICOM file') },
  ]);
  const report = page.getByRole('dialog', { name: 'Import DICOM' });
  await expect(report).toContainText('1 DICOM file(s) cannot be read');
  await report.getByRole('button', { name: 'OK' }).click();
  await expect(report).toBeHidden();
  await expect(page.locator('.w-patient-list li')).toHaveCount(1);
});

test('File > Import > DICOM imports a folder, recursively when asked', async ({ page }) => {
  const importFolder = async (recursive) => {
    await page.getByRole('button', { name: 'File' }).click();
    await page.getByRole('menuitem', { name: 'Import' }).hover();
    await page.getByRole('menuitem', { name: 'DICOM' }).click();
    const dialog = page.getByRole('dialog', { name: 'Import DICOM' });
    await dialog.locator('fieldset').nth(1).locator('input[type=file]')
      .setInputFiles(testdata('pydicom/dicomdir-tiny-alpha'));
    await expect(dialog.getByRole('textbox', { name: 'Folders' })).toHaveValue('dicomdir-tiny-alpha');
    await dialog.getByRole('checkbox', { name: 'Search recursively' }).setChecked(recursive);
    await page.screenshot({ path: `test-results/import-dialog-${recursive}.png` });
    await dialog.getByRole('button', { name: 'Import and Close' }).click();
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('status')).toHaveCount(0);
  };
  // Only the DICOMDIR and a README are directly in the folder: nothing to import, no message.
  await importFolder(false);
  const report = page.getByRole('dialog', { name: 'Import DICOM' });
  await expect(report).toBeHidden();
  // Its subfolders hold 50 CT objects without pixel data, which no viewer shows.
  await importFolder(true);
  await expect(report).toContainText('50 DICOM file(s) skipped during import.');
  await report.getByRole('checkbox', { name: "Don't show this again" }).check();
  await report.getByRole('button', { name: 'OK' }).click();
  await expect(report).toBeHidden();
  // Not told again.
  await importFolder(true);
  await expect(report).toBeHidden();
});

test('files dropped on the window are imported', async ({ page }) => {
  const bytes = [...fs.readFileSync(testdata('pydicom/MR_small.dcm'))];
  const dataTransfer = await page.evaluateHandle((content) => {
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array(content)], 'MR_small.dcm'));
    return data;
  }, bytes);
  await page.dispatchEvent('.w-viewers', 'dragover', { dataTransfer });
  await page.dispatchEvent('.w-viewers', 'drop', { dataTransfer });
  await expect(page.locator('.w-patient-list li')).toHaveText([/CompressedSamples\^MR1/]);
});

test('a ZIP archive is read, after its password when encrypted', async ({ page }) => {
  await openFiles(page, [testdata('synthetic/ct-series.zip')]);
  await expect(page.locator('.w-patient-list li')).toHaveText([/WEASIS\^SYNTHETIC/]);

  await openFiles(page, [testdata('synthetic/ct-series-encrypted.zip')]);
  const prompt = page.getByRole('dialog', { name: 'DICOM Zip' });
  await prompt.getByLabel('Enter a password').fill('weasis');
  await page.screenshot({ path: 'test-results/zip-password.png' });
  await prompt.getByRole('button', { name: 'OK' }).click();
  await expect(prompt).toBeHidden();
  await expect(page.getByRole('status')).toHaveCount(0);
  // Same patient: the instance joins the series already loaded.
  await expect(page.locator('.w-patient-list li')).toHaveCount(1);
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('the DICOMDIR page reads the files the DICOMDIR references', async ({ page }) => {
  const importDicomdir = async (folder) => {
    await page.getByRole('button', { name: 'File' }).click();
    await page.getByRole('menuitem', { name: 'Import' }).hover();
    await page.getByRole('menuitem', { name: 'DICOM' }).click();
    const dialog = page.getByRole('dialog', { name: 'Import DICOM' });
    await dialog.getByRole('option', { name: 'DICOMDIR' }).click();
    await dialog.locator('.w-item-page:not([hidden]) input[type=file]').setInputFiles(testdata(folder));
    await dialog.getByRole('button', { name: 'Import and Close' }).click();
    await expect(dialog).toBeHidden();
  };
  // The 50 referenced CT objects have no pixel data: they are skipped.
  await importDicomdir('pydicom/dicomdir-tiny-alpha');
  const report = page.getByRole('dialog', { name: 'Import DICOM' });
  await expect(report).toContainText('50 DICOM file(s) skipped during import.');
  await report.getByRole('button', { name: 'OK' }).click();

  // Without DICOMDIR, a plain import of the folder is offered.
  await importDicomdir('synthetic/ct-series');
  const question = page.getByRole('dialog', { name: 'DICOMDIR' });
  await expect(question).toContainText('No new file has been found from DICOMDIR');
  await question.getByRole('button', { name: 'Yes' }).click();
  await expect(page.locator('.w-patient-list li')).toHaveText([/WEASIS\^SYNTHETIC — 1 series/]);
});
