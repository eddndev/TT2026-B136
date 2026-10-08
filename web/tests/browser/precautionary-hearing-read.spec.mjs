import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  setupPrecautionaryAgenda,
  openPrecautionaryAgenda,
  precautionaryCard,
  precautionaryDetail,
  closePrecautionaryDetail,
  setupPrecautionaryAlerts,
  openAlerts,
  openPrecautionaryAlert,
  alertCard,
  filterAlerts,
  queryAgenda,
} from './precautionary-hearing-read-helpers.mjs';

for (const width of [1440, 390])
  test(`precautionary Agenda opens cancelled R3 and its exact history at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupPrecautionaryAgenda(page, { closed: true });
    await openPrecautionaryAgenda(page, state);
    expect(
      await page
        .locator('[data-agenda-kind]')
        .evaluateAll((rows) => rows.map((row) => row.dataset.agendaKind)),
    ).toEqual(['hearing', 'deadline', 'precautionary_hearing']);
    await page
      .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
      .selectOption('precautionary_hearing');
    await page
      .getByRole('combobox', { name: 'Estado de audiencia', exact: true })
      .selectOption('cancelled');
    await queryAgenda(page, 'day', '2026-01-04');
    const card = precautionaryCard(page, state);
    await expect(card).toContainText('Audiencia cautelar');
    await expect(card).toContainText(/Imposici[o\u00f3]n/);
    await expect(card).toContainText('Cancelada');
    await expect(page.locator('[data-agenda-kind]')).toHaveCount(1);
    expect(state.calls.at(-1).searchParams.get('kind')).toBe('precautionary_hearing');
    expect(state.calls.at(-1).searchParams.get('hearing_status')).toBe('cancelled');
    await card.click();
    const detail = precautionaryDetail(page),
      capture = state.selected.capture,
      values = capture.review.resolved_values;
    await expect(detail).toBeVisible();
    await expect(
      detail.getByRole('heading', { name: /Imposici[o\u00f3]n de medidas cautelares/ }),
    ).toBeVisible();
    await expect(detail).toContainText(/Revisi[o\u00f3]n exacta consultada: 3/);
    await expect(detail).toContainText('Cancelada');
    await expect(detail).toContainText('Expediente cerrado administrativamente');
    await expect(detail).toContainText('09:00:00');
    await expect(detail).toContainText('-06:00');
    await expect(detail).toContainText(values.venue);
    await expect(detail).toContainText(values.scheduling_basis.statement);
    await expect(detail).toContainText(values.scheduling_basis.locator);
    await expect(detail).toContainText(capture.review.sources.support.name);
    for (const person of capture.review.participants)
      await expect(detail).toContainText(person.overview.display_name);
    await detail.getByText('Historia de la convocatoria', { exact: true }).click();
    for (const revision of [1, 2, 3])
      await expect(detail).toContainText(new RegExp(`Revisi[o\u00f3]n ${revision}`));
    await expect(detail).toContainText('Nuevo senalamiento declarado');
    await expect(detail).toContainText('Cancelacion declarada');
    await expect(detail).toContainText(capture.capture_digest);
    await expect(
      detail.getByRole('button', { name: /Guardar|Confirmar|Cancelar audiencia/ }),
    ).toHaveCount(0);
    await expect(page.getByRole('region', { name: 'Detalle de plazo', exact: true })).toHaveCount(
      0,
    );
    expect(state.order).toEqual(['administration', 'exact']);
    expect(state.detailCalls).toHaveLength(1);
    expect(state.detailCalls[0].method()).toBe('GET');
    expect(state.headCalls).toEqual([]);
    expect(state.unwanted).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    const shots = fileURLToPath(
      new URL('../../../output/precautionary-measures/qadra/visual/', import.meta.url),
    );
    await mkdir(shots, { recursive: true });
    await page.evaluate(async () => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    });
    await page.screenshot({
      path: resolve(shots, `precautionary-agenda-${width}.png`),
      fullPage: true,
    });
    await closePrecautionaryDetail(page);
    await expect(detail).toHaveCount(0);
    await expect(card).toBeVisible();
    await expect(page.getByLabel('Fecha de referencia', { exact: true })).toHaveValue('2026-01-04');
    await expect(
      page.getByRole('combobox', { name: 'Estado de audiencia', exact: true }),
    ).toHaveValue('cancelled');
    expect(state.detailCalls).toHaveLength(1);
  });

test('precautionary alert opens its captured R2 while the available head is cancelled R3', async ({
  page,
}) => {
  const state = await setupPrecautionaryAlerts(page, { closed: true });
  await openAlerts(page);
  await filterAlerts(page, 'unread', 'all');
  const card = alertCard(page, state.own);
  await expect(card).toContainText('Audiencia cautelar pr\u00f3xima');
  await expect(card).toContainText('Fecha de actividad capturada');
  await expect(card).toContainText('Correo deshabilitado');
  await openPrecautionaryAlert(page, state);
  const detail = precautionaryDetail(page);
  await expect(detail).toBeVisible();
  await expect(detail).toContainText(/Revisi[o\u00f3]n exacta consultada: 2/);
  await expect(detail).toContainText('Programada');
  await expect(detail).not.toContainText('Cancelacion declarada');
  await expect(detail).toContainText(state.selected.capture.review.resolved_values.venue);
  await expect(detail).toContainText('09:00:00');
  await expect(detail).toContainText('-06:00');
  await detail.getByText('Historia de la convocatoria', { exact: true }).click();
  await expect(detail).toContainText(/Revisi[o\u00f3]n 1/);
  await expect(detail).toContainText(/Revisi[o\u00f3]n 2/);
  await expect(detail).not.toContainText(/Revisi[o\u00f3]n 3/);
  await expect(detail).toContainText(state.own.origin.evidence_digest);
  expect(state.head.capture.review.result_revision).toBe(3);
  expect(state.order).toEqual(['alert', 'administration', 'exact']);
  expect(state.detailCalls).toHaveLength(1);
  expect(new URL(state.detailCalls[0].url()).pathname).toBe(state.exactPath);
  expect(new URL(state.detailCalls[0].url()).search).toBe('');
  expect(state.headCalls).toEqual([]);
  expect(state.unwanted).toEqual([]);
  expect(state.calls.filter((call) => call.method !== 'GET')).toEqual([]);
  await expect(page.getByRole('region', { name: 'Detalle de plazo', exact: true })).toHaveCount(0);
  await closePrecautionaryDetail(page);
  await expect(detail).toHaveCount(0);
  await expect(card).toBeVisible();
  await expect(page.getByRole('combobox', { name: 'Lectura', exact: true })).toHaveValue('unread');
  await expect(page.getByRole('combobox', { name: 'Estado de alerta', exact: true })).toHaveValue(
    'all',
  );
});
