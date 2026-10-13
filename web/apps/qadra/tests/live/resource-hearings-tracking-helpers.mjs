import { expect } from '@playwright/test';
import { navigate } from '../case-administration-workflow.mjs';
import { responseTo } from './procedural-resources-helpers.mjs';
import { ownExact, ownDetail, captureOwn } from './resource-hearings-real-helpers.mjs';

export async function ownAlerts(call, creation) {
  const rows = [];
  let cursor;
  for (let pageIndex = 0; pageIndex < 10; pageIndex++) {
    const query = new URLSearchParams({ limit: 100, read: 'all', state: 'all' });
    if (cursor) query.set('cursor', cursor);
    const result = await call('GET', `/alerts?${query}`);
    rows.push(
      ...result.alerts.filter(
        (row) => row.subject.kind === 'resource_hearing' && row.subject.id === creation.hearing.id,
      ),
    );
    if (!result.has_more) return rows;
    expect(result.next_cursor).not.toBeNull();
    expect(result.next_cursor).not.toBe(cursor);
    cursor = result.next_cursor;
  }
  throw new Error('Own hearing alert scan exceeded the bounded fixture volume');
}

export async function waitOwnAlert(call, actor, creation, seconds) {
  let rows;
  await expect
    .poll(
      async () => {
        rows = await ownAlerts(call, creation);
        return rows.length;
      },
      { timeout: 60000, intervals: [250, 500, 1000] },
    )
    .toBe(1);
  const alert = rows[0],
    hearing = creation.hearing;
  expect(alert).toMatchObject({
    recipient_id: actor.id,
    subject: {
      kind: 'resource_hearing',
      case_id: hearing.case_id,
      resource_id: hearing.resource_id,
      id: hearing.id,
    },
    origin: { revision: 1, evidence_digest: hearing.capture_digest },
    kind: {
      kind: 'upcoming',
      lead_hours: 48,
      activity_at: { unix_seconds: seconds, nanosecond: 0, offset_seconds: 0 },
    },
    state: { kind: 'active' },
    email: { kind: 'disabled' },
    read_at: null,
  });
  return alert;
}

export async function checkOwnAgenda(page, scenario, creation, seconds) {
  await navigate(page, 'Agenda');
  const agenda = page.getByRole('region', { name: 'Agenda combinada', exact: true });
  await expect(agenda).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption('day');
  await page
    .getByLabel('Fecha de referencia', { exact: true })
    .fill(new Date(seconds * 1000).toISOString().slice(0, 10));
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('+00:00');
  await page
    .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
    .selectOption('resource_hearing');
  const querying = responseTo(page, '/api/v1/agenda');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  const response = await querying;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  const result = await response.json();
  expect(result.complete).toBe(true);
  const found = result.items.filter(
    (row) => row.kind === 'resource_hearing' && row.resource_hearing.id === creation.hearing.id,
  );
  expect(found).toHaveLength(1);
  const hearing = creation.hearing;
  expect(found[0].resource_hearing).toEqual({
    case_id: hearing.case_id,
    resource_id: hearing.resource_id,
    id: hearing.id,
    revision: 1,
    kind: hearing.values.kind,
    scheduled_at: hearing.values.scheduled_at,
    modality: hearing.values.modality,
    participant_count: hearing.values.participants.length,
    association_id: hearing.association_id,
    capture_digest: hearing.capture_digest,
  });
  expect(found[0].at).toEqual({ unix_seconds: seconds, nanosecond: 0, offset_seconds: 0 });
  const reading = responseTo(page, ownExact(scenario, creation));
  await page
    .getByRole('button', { name: `Consultar audiencia de recurso ${hearing.id}`, exact: true })
    .click();
  const exact = await reading;
  expect(exact.status()).toBe(200);
  expect(await exact.json()).toEqual(creation);
  await expect(ownDetail(page)).toBeVisible();
  await ownDetail(page)
    .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
    .click();
  return found[0];
}

export async function checkOwnInbox(page, scenario, creation, alert, testInfo, name) {
  await navigate(page, 'Alertas');
  await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
  const card = page.locator(`[data-alert-id="${alert.id}"]`);
  for (let index = 0; (await card.count()) === 0 && index < 10; index++) {
    const next = page.getByRole('button', { name: 'Cargar m\u00e1s alertas', exact: true });
    await expect(next).toBeVisible();
    const querying = responseTo(page, '/api/v1/alerts');
    await next.click();
    expect((await querying).status()).toBe(200);
    await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
  }
  await expect(card).toContainText('48 horas antes');
  await expect(card).toContainText('Correo deshabilitado');
  const reading = responseTo(page, ownExact(scenario, creation));
  await card.getByRole('button', { name: 'Abrir audiencia de recurso', exact: true }).click();
  const exact = await reading;
  expect(exact.status()).toBe(200);
  expect(exact.headers()['cache-control']).toBe('no-store');
  expect(await exact.json()).toEqual(creation);
  await ownDetail(page).getByText('Autor y recibo original', { exact: true }).click();
  await expect(ownDetail(page)).toContainText(creation.hearing.capture_digest);
  await captureOwn(page, testInfo, `resource-hearing-alert-${name}`);
  await ownDetail(page)
    .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
    .click();
  const marking = responseTo(page, `/api/v1/alerts/${alert.id}/read`, 'POST');
  await card.getByRole('button', { name: 'Marcar como le\u00edda', exact: true }).click();
  const response = await marking;
  expect(response.status()).toBe(200);
  const receipt = await response.json();
  expect(receipt.alert).toEqual({ ...alert, read_at: receipt.alert.read_at });
  expect(receipt.alert.read_at).not.toBeNull();
  await expect(card.getByText('Le\u00edda', { exact: true })).toBeVisible();
  return receipt;
}
