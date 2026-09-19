import { test, expect } from '@playwright/test';
import { login, navigate, caseId } from './helpers.mjs';
import { clone, alertCursor } from '../fixtures/alerts.mjs';
import {
  setupAlerts,
  openAlerts,
  alertCard,
  filterAlerts,
  alertPageFor,
} from './alerts-helpers.mjs';

for (const width of [1440, 390])
  test(`the personal inbox preserves Qadra and separates captured reasons and email at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupAlerts(page);
    await openAlerts(page);
    const inbox = page.getByRole('region', { name: 'Mis alertas', exact: true });
    await expect(inbox.locator('[data-alert-id]')).toHaveCount(5);
    for (const text of [
      'Audiencia pr\u00f3xima',
      'Plazo pr\u00f3ximo',
      'Revisi\u00f3n requerida',
      'Vencido sin atenci\u00f3n declarada',
      'Fecha pr\u00f3xima modificada',
    ])
      await expect(inbox.getByText(text, { exact: true })).toBeVisible();
    const hearing = state.rows.find((row) => row.subject.kind === 'hearing');
    await expect(alertCard(page, hearing)).toContainText('24 horas antes');
    await expect(alertCard(page, hearing)).toContainText('Correo deshabilitado');
    await expect(alertCard(page, hearing)).toContainText('Sin leer');
    await expect(
      inbox.getByText(/La fecha capturada no acredita la vigencia actual/),
    ).toBeVisible();
    expect(state.calls.filter((call) => call.method !== 'GET')).toHaveLength(0);
    await page.screenshot({ path: test.info().outputPath(`alerts-${width}.png`), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });

test('a lost read response is checked explicitly without a repeated mutation or deadline attention', async ({
  page,
}) => {
  const state = await setupAlerts(page);
  state.loseReadResponse = true;
  await openAlerts(page);
  const row = state.rows.find((value) => value.kind.kind === 'overdue_unattended');
  const card = alertCard(page, row);
  await card.getByRole('button', { name: 'Marcar como le\u00edda', exact: true }).click();
  await expect(card.getByRole('alert')).toBeVisible();
  await expect(card.getByText('Sin leer', { exact: true })).toBeVisible();
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(1);
  await card.getByRole('button', { name: 'Comprobar lectura', exact: true }).click();
  await expect(card.getByText('Le\u00edda', { exact: true })).toBeVisible();
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(1);
  expect(state.deadlines.calls.filter((call) => call.method !== 'GET')).toHaveLength(0);
  await filterAlerts(page, 'unread');
  await expect(card).toHaveCount(0);
});

test('personal preference conflicts preserve the draft and require comparison before saving again', async ({
  page,
}) => {
  const state = await setupAlerts(page);
  state.preferenceConflict = true;
  await openAlerts(page);
  await page.getByRole('button', { name: 'Preferencias de alertas', exact: true }).click();
  const form = page.getByRole('region', { name: 'Preferencias de alertas', exact: true });
  const hours = form.getByLabel('Anticipaciones de audiencias (horas)', { exact: true });
  await expect(hours).toHaveValue('48,24');
  await expect(form).toContainText('Para mi cuenta');
  await hours.fill('72,24');
  await form.getByRole('button', { name: 'Guardar preferencias', exact: true }).click();
  await expect(form.getByRole('alert')).toBeVisible();
  await expect(hours).toHaveValue('72,24');
  await expect(
    form.getByRole('button', { name: 'Guardar preferencias', exact: true }),
  ).toBeDisabled();
  expect(state.calls.filter((call) => call.method === 'PUT')).toHaveLength(1);
  await form.getByRole('button', { name: 'Consultar preferencias actuales', exact: true }).click();
  await expect(
    form.getByRole('region', { name: 'Preferencias actuales guardadas', exact: true }),
  ).toContainText('Revisi\u00f3n 1');
  await expect(hours).toHaveValue('72,24');
  await form.getByRole('button', { name: 'Guardar mis preferencias', exact: true }).click();
  await expect(page.getByText('Preferencias guardadas.', { exact: true })).toBeVisible();
  const writes = state.calls.filter((call) => call.method === 'PUT');
  expect(writes).toHaveLength(2);
  expect(writes[1].body.expected_revision).toBe(1);
  expect(writes[1].body.values.hearing_upcoming.lead_hours).toEqual([72, 24]);
  expect(writes[1].body.operation_id).not.toBe(writes[0].body.operation_id);
});

for (const kind of ['hearing', 'deadline'])
  test(`an alert opens its exact ${kind} origin once and returns to the same inbox filters`, async ({
    page,
  }) => {
    const state = await setupAlerts(page);
    await openAlerts(page);
    await filterAlerts(page, 'unread', 'all');
    const row = state.rows.find(
      (value) => value.subject.kind === kind && value.kind.kind === 'upcoming',
    );
    await alertCard(page, row)
      .getByRole('button', {
        name: kind === 'hearing' ? 'Abrir audiencia' : 'Abrir plazo',
        exact: true,
      })
      .click();
    const detail = page.getByRole('region', {
      name: kind === 'hearing' ? 'Detalle de audiencia' : 'Detalle de plazo',
      exact: true,
    });
    await expect(detail).toContainText(/consultada exactamente/i);
    const calls =
      kind === 'hearing' ? state.deadlines.facts.results.scheduling.calls : state.deadlines.calls;
    const exact = () =>
      calls.filter((call) =>
        call.path.endsWith(
          `/${kind === 'hearing' ? 'hearings' : 'deadlines'}/${row.subject.id}/revisions/1`,
        ),
      );
    expect(exact()).toHaveLength(1);
    expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(0);
    await page.getByRole('button', { name: 'Volver a Alertas', exact: true }).click();
    await expect(page.getByRole('combobox', { name: 'Lectura', exact: true })).toHaveValue(
      'unread',
    );
    await expect(page.getByRole('combobox', { name: 'Estado de alerta', exact: true })).toHaveValue(
      'all',
    );
    await expect(alertCard(page, row)).toBeVisible();
    expect(exact()).toHaveLength(1);
  });

test('empty partial scans keep continuation and late pages cannot restore a changed filter', async ({
  page,
}) => {
  const state = await setupAlerts(page);
  const oldest = clone(state.rows.at(-1));
  let release,
    started = false,
    ended = false;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  state.handle = async (route, url) => {
    if (!url.pathname.endsWith('/alerts')) return false;
    if (url.searchParams.get('read') === 'unread') {
      await route.fulfill({ json: alertPageFor([]) });
      return true;
    }
    if (!url.searchParams.get('cursor')) {
      const cursorRow = clone(state.rows[0]);
      cursorRow.created_at.unix_seconds++;
      await route.fulfill({ json: alertPageFor([], true, alertCursor(cursorRow)) });
      return true;
    }
    started = true;
    await gate;
    await route.fulfill({ json: alertPageFor([oldest]) });
    ended = true;
    return true;
  };
  await openAlerts(page);
  await expect(
    page.getByText('Consulta parcial: faltan alertas por consultar.', { exact: true }),
  ).toBeVisible();
  await expect(page.getByText('No hay alertas en esta consulta.', { exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Cargar m\u00e1s alertas', exact: true }).click();
  await expect.poll(() => started).toBe(true);
  await filterAlerts(page, 'unread');
  await expect(page.getByText('No hay alertas en esta consulta.', { exact: true })).toBeVisible();
  release();
  await expect.poll(() => ended).toBe(true);
  await expect(page.locator('[data-alert-id]')).toHaveCount(0);
});

test('revoked case access removes its captured alerts instead of opening protected content', async ({
  page,
}) => {
  const state = await setupAlerts(page);
  await openAlerts(page);
  await page.route(`**/api/v1/cases/${caseId}/administration`, (route) =>
    route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } }),
  );
  const row = state.rows.find((value) => value.subject.kind === 'hearing');
  await alertCard(page, row).getByRole('button', { name: 'Abrir audiencia', exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'Mis alertas', exact: true }).getByRole('alert'),
  ).toBeVisible();
  await expect(page.locator('[data-alert-id]')).toHaveCount(0);
});

test('Client has no alert navigation or private preference and inbox requests', async ({
  page,
}) => {
  const state = await setupAlerts(page, { role: 'client' });
  await login(page, false, false);
  await expect(page.getByRole('button', { name: 'Alertas', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = '#alerts';
  });
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  expect(state.calls).toHaveLength(0);
  await navigate(page, 'Expedientes');
  expect(state.calls).toHaveLength(0);
});
