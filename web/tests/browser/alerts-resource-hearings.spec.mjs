import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  setupOwnAlerts,
  openAlerts,
  alertCard,
  filterAlerts,
  ownDetail,
  openOwn,
} from './alerts-resource-hearings-helpers.mjs';

for (const width of [1440, 390])
  test(`own hearing alert opens captured R1 without current association at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupOwnAlerts(page, { closed: true });
    await openAlerts(page);
    await filterAlerts(page, 'unread', 'all');
    const card = alertCard(page, state.own);
    await expect(card).toContainText('Audiencia de recurso pr\u00f3xima');
    await expect(card).toContainText('Correo deshabilitado');
    await expect(card).toContainText('Fecha de actividad capturada');
    await openOwn(page, state);
    const detail = ownDetail(page),
      hearing = state.creation.hearing;
    await expect(detail).toBeVisible();
    expect(state.order).toEqual(['alert', 'administration', 'exact']);
    expect(state.ownCalls).toHaveLength(1);
    expect(state.ownCalls[0].method()).toBe('GET');
    expect(state.unwanted).toEqual([]);
    await expect(detail).toContainText('Expediente cerrado administrativamente');
    await expect(detail).toContainText('18:00:00');
    await expect(detail).toContainText('-06:00');
    await expect(detail).toContainText(hearing.sources.support.name);
    await expect(detail).toContainText(hearing.sources.act.act.values.statement);
    await detail.getByText('Autor y recibo original', { exact: true }).click();
    await expect(detail).toContainText(hearing.capture_digest);
    await expect(detail).toContainText(hearing.association_id);
    await expect(detail).toContainText('No declara su estado actual.');
    await expect(
      page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
    ).toHaveCount(0);
    await expect(page.getByRole('region', { name: 'Detalle de plazo', exact: true })).toHaveCount(
      0,
    );
    expect(state.calls.filter((call) => call.method !== 'GET')).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    const shots = resolve(
      process.env.TT_RESOURCE_HEARING_ALERT_SHOTS ||
        fileURLToPath(new URL('../../../output/resource-hearings/alerts/shots/', import.meta.url)),
    );
    await mkdir(shots, { recursive: true });
    await page.evaluate(async () => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    });
    await page.screenshot({
      path: resolve(shots, `resource-hearing-alert-${width}.png`),
      fullPage: true,
    });
    await detail.getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true }).click();
    await expect(detail).toHaveCount(0);
    await expect(page.getByRole('combobox', { name: 'Lectura', exact: true })).toHaveValue(
      'unread',
    );
    await expect(page.getByRole('combobox', { name: 'Estado de alerta', exact: true })).toHaveValue(
      'all',
    );
    await expect(card).toBeVisible();
    expect(state.ownCalls).toHaveLength(1);
  });

test('revoked case access clears own alerts before any exact protected read', async ({ page }) => {
  const state = await setupOwnAlerts(page);
  await openAlerts(page);
  await expect(alertCard(page, state.own)).toBeVisible();
  state.denied = true;
  await openOwn(page, state);
  await expect(
    page.getByRole('region', { name: 'Mis alertas', exact: true }).getByRole('alert'),
  ).toBeVisible();
  expect(state.order).toEqual(['alert', 'administration']);
  expect(state.ownCalls).toEqual([]);
  await expect(page.locator('[data-alert-id]')).toHaveCount(0);
  await expect(ownDetail(page)).toHaveCount(0);
});

test('a different valid own capture cannot replace the origin selected in an alert', async ({
  page,
}) => {
  const state = await setupOwnAlerts(page);
  state.own.origin.evidence_digest = '1'.repeat(64);
  await openAlerts(page);
  await openOwn(page, state);
  await expect(
    page.getByRole('region', { name: 'Mis alertas', exact: true }).getByRole('alert'),
  ).toBeVisible();
  await expect(ownDetail(page)).toHaveCount(0);
  expect(state.ownCalls).toHaveLength(1);
  expect(state.unwanted).toEqual([]);
});

test('filter changes invalidate a pending own alert detail without restoring it from a late response', async ({
  page,
}) => {
  const state = await setupOwnAlerts(page);
  let release,
    started = false,
    ended = false;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  state.onExact = async (route) => {
    started = true;
    await gate;
    await route.fulfill({ json: state.creation });
    ended = true;
  };
  await openAlerts(page);
  await openOwn(page, state);
  try {
    await expect.poll(() => started).toBe(true);
    await filterAlerts(page, 'unread', 'all');
    release();
    await expect.poll(() => ended).toBe(true);
    await expect(alertCard(page, state.own)).toBeVisible();
    await expect(ownDetail(page)).toHaveCount(0);
    expect(state.ownCalls).toHaveLength(1);
  } finally {
    release();
  }
});
