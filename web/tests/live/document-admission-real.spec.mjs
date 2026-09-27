import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  png,
  wav,
  gif,
  values,
  metadataCard,
  history,
  responseTo,
  openDocuments,
  fillMetadata,
  retainedFile,
  downloadExact,
  accountAction,
} from './document-admission-real-helpers.mjs';

test('real format admission preserves rejected drafts and exact classified PNG/WAV versions', async ({
  page,
}, testInfo) => {
  expect(accounts?.admission?.case).toBeDefined();
  const record = accounts.admission.case;
  const base = `/cases/${record.id}/documents`;
  const errors = [],
    writes = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('request', (request) => {
    if (request.method() === 'POST' && new URL(request.url()).pathname.startsWith(`/api/v1${base}`))
      writes.push(request);
  });
  await page.goto('/');
  await loginAs(page, accounts.owner, 2);
  await openDocuments(page, record);
  await accountAction(accounts.owner, 3, async (call) => {
    const empty = await call('GET', base);
    expect(empty.documents).toEqual([]);
    await page.getByRole('button', { name: 'Subir documento', exact: true }).first().click();
    const upload = page.getByRole('dialog', { name: 'Subir documento', exact: true });
    const input = upload.getByLabel('Archivo', { exact: true });
    const truncated = png.subarray(0, png.length - 1);
    await input.setInputFiles({ name: 'truncated.png', mimeType: 'image/png', buffer: truncated });
    await upload.getByLabel('Nombre del documento', { exact: true }).fill('reviewed.png');
    await fillMetadata(upload);
    const rejected = responseTo(page, `${base}/with-metadata`);
    await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
    const rejection = await rejected;
    expect(rejection.status()).toBe(422);
    expect((await rejection.json()).error.code).toBe('document_format_invalid');
    await expect(upload.getByRole('alert')).toContainText(/archivo|contenido|estructura/i);
    await expect(
      upload.getByRole('button', { name: 'Cargar documento', exact: true }),
    ).toBeEnabled();
    await retainedFile(input, 'truncated.png', truncated);
    await expect(upload.getByLabel('Nombre del documento', { exact: true })).toHaveValue(
      'reviewed.png',
    );
    await expect(upload.getByLabel('Tipo de documento (opcional)', { exact: true })).toHaveValue(
      values.document_type,
    );
    await expect(upload.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
      values.classification,
    );
    await expect(
      upload.getByRole('button', { name: 'Quitar etiqueta: admission, exact', exact: true }),
    ).toBeVisible();
    await expect(page.getByRole('heading', { name: 'reviewed.png', exact: true })).toHaveCount(0);
    expect(await call('GET', base)).toEqual(empty);
    expect(writes).toHaveLength(1);
    const originalViewport = page.viewportSize();
    try {
      for (const width of [1440, 390]) {
        await page.setViewportSize({ width, height: 1000 });
        await upload.getByRole('alert').scrollIntoViewIfNeeded();
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
          true,
        );
        expect(await upload.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(
          true,
        );
        await page.screenshot({
          path: testInfo.outputPath(`admission-rejected-draft-${width}.png`),
          fullPage: true,
        });
      }
    } finally {
      await page.setViewportSize(originalViewport);
    }

    await input.setInputFiles({ name: 'admitted.png', mimeType: 'image/png', buffer: png });
    const created = responseTo(page, `${base}/with-metadata`);
    await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
    const creation = await created;
    expect(creation.status()).toBe(201);
    const first = await creation.json();
    expect(first.version).toBe(1);
    expect(first.current_metadata).toEqual({ metadata_revision: 1, ...values });
    await expect(page.getByRole('heading', { name: 'admitted.png', exact: true })).toBeVisible();
    await expect(history(page)).toHaveAttribute('aria-busy', 'false');
    await expect(metadataCard(page)).toContainText('Reservado');
    await downloadExact(page, first, png);
    const path = `${base}/${first.id}`;
    const originalHistory = await call('GET', `${path}/versions`);
    const originalMetadata = await call('GET', `${path}/metadata/history`);

    await page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true }).click();
    const append = page.getByRole('dialog', { name: 'Agregar versi\u00f3n', exact: true });
    const versionFile = append.getByLabel('Archivo de la nueva versi\u00f3n', { exact: true });
    await versionFile.setInputFiles({
      name: 'unsupported.gif',
      mimeType: 'image/gif',
      buffer: gif,
    });
    const denied = responseTo(page, `${path}/versions`);
    await append.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
    const denial = await denied;
    expect(denial.status()).toBe(422);
    expect((await denial.json()).error.code).toBe('document_format_unsupported');
    expect(new URL(denial.url()).searchParams.get('expected_version')).toBe('1');
    await expect(append.getByRole('alert')).toContainText(/formato/i);
    await expect(
      append.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }),
    ).toBeEnabled();
    await retainedFile(versionFile, 'unsupported.gif', gif);
    await expect(append.getByText('Versi\u00f3n de partida: 1', { exact: true })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'unsupported.gif', exact: true })).toHaveCount(
      0,
    );
    expect(await call('GET', path)).toEqual(first);
    expect(await call('GET', `${path}/versions`)).toEqual(originalHistory);
    expect(await call('GET', `${path}/metadata/history`)).toEqual(originalMetadata);
    expect(writes).toHaveLength(3);

    await versionFile.setInputFiles({
      name: 'admitted-v2.wav',
      mimeType: 'audio/wav',
      buffer: wav,
    });
    const appended = responseTo(page, `${path}/versions`);
    await append.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
    const accepted = await appended;
    expect(accepted.status()).toBe(201);
    const second = await accepted.json();
    expect(second.id).toBe(first.id);
    expect(second.version).toBe(2);
    await expect(page.getByRole('heading', { name: 'admitted-v2.wav', exact: true })).toBeVisible();
    await expect(history(page)).toHaveAttribute('aria-busy', 'false');
    await expect(metadataCard(page)).toContainText('Reservado');
    expect((await call('GET', path)).current_metadata).toEqual(first.current_metadata);
    expect(await call('GET', `${path}/metadata/history`)).toEqual(originalMetadata);
    await downloadExact(page, second, wav);
    const historical = responseTo(page, `${path}/versions/1`, 'GET');
    await history(page)
      .getByRole('button')
      .filter({ has: page.getByText('Versi\u00f3n 1 / admitted.png', { exact: true }) })
      .click();
    expect((await historical).status()).toBe(200);
    await expect(page.getByRole('heading', { name: 'admitted.png', exact: true })).toBeVisible();
    await downloadExact(page, first, png);
    expect(writes).toHaveLength(4);
  });
  expect(errors).toEqual([]);
});
