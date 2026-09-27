import { test, expect } from '@playwright/test';
import {
  setupRelated,
  openRelatedFromAlert,
  related,
  item,
  targetDetail,
  assocId,
  navigate,
  login,
  caseId,
} from './activity-resources-helpers.mjs';

for (const kind of ['hearing', 'deadline'])
  test(`alert ${kind} opens exact linked resource and returns to distinct activity origin without reading it`, async ({
    page,
  }) => {
    const state = await setupRelated(page, kind);
    await openRelatedFromAlert(page, state);
    await expect(related(page)).toContainText(/estado actual.*v[i\u00ed]nculos/i);
    await expect(related(page)).toContainText(/no.*emisi[o\u00f3]n.*alerta/i);
    await expect(related(page)).toContainText('Revisi\u00f3n consultada de la actividad: 2');
    await expect(item(page, state.association.id)).toContainText(
      'Revisi\u00f3n vinculada de la actividad: 1',
    );
    await expect(item(page, state.association.id)).toContainText('Recurso revisi\u00f3n 1');
    await item(page, state.association.id)
      .getByRole('button', { name: `Abrir recurso vinculado ${state.association.id}`, exact: true })
      .click();
    const resource = page.getByRole('region', { name: 'Detalle de recurso', exact: true });
    const association = page.getByRole('region', {
      name: 'Detalle de actividad vinculada',
      exact: true,
    });
    await expect(resource).toContainText(/revisi[o\u00f3]n 1.*consultada exactamente/i);
    await expect(association).toContainText(
      /v[i\u00ed]nculo.*revisi[o\u00f3]n 1.*consulta historica exacta/i,
    );
    const calls = state.activities.resources.calls.map((call) => call.path);
    expect(calls).toContain(
      `/api/v1/cases/${caseId}/procedural-resources/${state.activities.resource.id}/revisions/1`,
    );
    expect(state.activities.calls.map((call) => call.path)).toContain(
      `/api/v1/cases/${caseId}/procedural-resources/${state.activities.resource.id}/activities/${state.association.id}/revisions/1`,
    );
    await page.getByRole('button', { name: 'Volver a actividad', exact: true }).click();
    await expect(targetDetail(page, kind)).toContainText(
      /revisi[o\u00f3]n 2.*consultada exactamente/i,
    );
    await expect(related(page)).toBeVisible();
    await page.getByRole('button', { name: 'Volver a Alertas', exact: true }).click();
    await expect(page.getByRole('combobox', { name: 'Lectura', exact: true })).toHaveValue(
      'unread',
    );
    await expect(page.getByRole('combobox', { name: 'Estado de alerta', exact: true })).toHaveValue(
      'all',
    );
    await expect(page.locator(`[data-alert-id="${state.alert.id}"]`)).toContainText('Sin leer');
    expect(state.alertCalls.every((call) => call.method === 'GET')).toBe(true);
    expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
    expect(state.alert.read_at).toBeNull();
  });

test('related resources page linked heads and explicitly request unlinked or all without mixing cursors', async ({
  page,
}) => {
  const state = await setupRelated(page);
  state.seed(21, true);
  await openRelatedFromAlert(page, state);
  await expect(related(page).locator('[data-association-id]')).toHaveCount(20);
  expect(new URLSearchParams(state.calls[0].search).get('status')).toBe('linked');
  await related(page)
    .getByRole('button', { name: 'Siguientes recursos relacionados', exact: true })
    .click();
  await expect(related(page).locator('[data-association-id]')).toHaveCount(1);
  await expect(item(page, assocId(21))).toBeVisible();
  expect(new URLSearchParams(state.calls.at(-1).search).get('after_id')).toBe(assocId(20));
  await related(page)
    .getByRole('combobox', { name: 'Estado del v\u00ednculo', exact: true })
    .selectOption('unlinked');
  await related(page)
    .getByRole('button', { name: 'Consultar recursos relacionados', exact: true })
    .click();
  await expect(item(page, assocId(22))).toBeVisible();
  await expect(related(page).locator('[data-association-id]')).toHaveCount(1);
  expect(new URLSearchParams(state.calls.at(-1).search).get('after_id')).toBeNull();
  await related(page)
    .getByRole('combobox', { name: 'Estado del v\u00ednculo', exact: true })
    .selectOption('all');
  await related(page)
    .getByRole('button', { name: 'Consultar recursos relacionados', exact: true })
    .click();
  await expect(related(page).locator('[data-association-id]')).toHaveCount(20);
  expect(new URLSearchParams(state.calls.at(-1).search).get('status')).toBe('all');
});

test('an empty related query is an ordinary authorized state', async ({ page }) => {
  const state = await setupRelated(page, 'deadline');
  state.seed(0);
  await openRelatedFromAlert(page, state);
  await expect(related(page)).toContainText('No hay recursos relacionados en esta consulta.');
  await expect(related(page).getByRole('alert')).toHaveCount(0);
  await expect(targetDetail(page, 'deadline')).toBeVisible();
});

for (const [status, code] of [
  [403, 'permission_denied'],
  [404, 'case_not_found'],
  [401, 'invalid_session'],
])
  test(`related read ${code} removes protected activity and resource information`, async ({
    page,
  }) => {
    const state = await setupRelated(page);
    await openRelatedFromAlert(page, state);
    await expect(item(page, state.association.id)).toBeVisible();
    state.handle = async (route) => {
      await route.fulfill({ status, json: { error: { code } } });
      return true;
    };
    await related(page)
      .getByRole('button', { name: 'Consultar recursos relacionados', exact: true })
      .click();
    await expect(related(page)).toHaveCount(0);
    await expect(targetDetail(page, 'hearing')).toHaveCount(0);
    if (status === 401) await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
    else
      await expect(
        page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
      ).toBeVisible();
    expect(state.alertCalls.every((call) => call.method === 'GET')).toBe(true);
  });

for (const destination of ['case', 'session'])
  test(`late related response cannot restore data after changing ${destination}`, async ({
    page,
  }) => {
    const state = await setupRelated(page);
    await openRelatedFromAlert(page, state);
    let release,
      started = false;
    const gate = new Promise((resolve) => {
      release = resolve;
    });
    state.handle = async (route, url) => {
      started = true;
      await gate;
      await route.fulfill({ json: state.page(url) });
      return true;
    };
    await related(page)
      .getByRole('button', { name: 'Consultar recursos relacionados', exact: true })
      .click();
    await expect.poll(() => started).toBe(true);
    if (destination === 'session') {
      await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
      await login(page, false, false);
      await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
    } else {
      await navigate(page, 'Expedientes');
      await page.getByRole('button', { name: /Otro expediente/ }).click();
      await expect(
        page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
      ).toBeVisible();
    }
    const response = page.waitForResponse((value) =>
      new URL(value.url()).pathname.endsWith('/resource-associations'),
    );
    release();
    await (await response).finished();
    await expect(related(page)).toHaveCount(0);
    await expect(page.getByRole('region', { name: 'Detalle de recurso', exact: true })).toHaveCount(
      0,
    );
    await expect(page.getByRole('button', { name: 'Volver a actividad', exact: true })).toHaveCount(
      0,
    );
  });
