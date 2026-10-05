import { expect } from '@playwright/test';
import { navigate } from '../case-administration-workflow.mjs';
import {
  responseTo,
  hearingExact,
  hearingDetail,
  capture,
} from './precautionary-hearings-real-helpers.mjs';

export async function hearingAlerts(call, id) {
  const rows = [];
  let cursor;
  for (let index = 0; index < 10; index++) {
    const params = new URLSearchParams({ limit: 100, read: 'all', state: 'all' });
    if (cursor) params.set('cursor', cursor);
    const page = await call('GET', `/alerts?${params}`);
    rows.push(
      ...page.alerts.filter(
        (row) => row.subject.kind === 'precautionary_hearing' && row.subject.id === id,
      ),
    );
    if (!page.has_more) return rows;
    expect(page.next_cursor).not.toBeNull();
    expect(page.next_cursor).not.toBe(cursor);
    cursor = page.next_cursor;
  }
  throw new Error('Precautionary alert scan exceeded the bounded fixture volume');
}

export async function agendaAndAlert(page, call, scenario, actor, scheduled, testInfo, name) {
  const hearing = scheduled.operation,
    review = hearing.capture.review;
  const id = review.command.hearing_id,
    values = review.resolved_values;
  await navigate(page, 'Agenda');
  await expect(page.getByRole('region', { name: 'Agenda combinada', exact: true })).toHaveAttribute(
    'aria-busy',
    'false',
  );
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption('day');
  await page
    .getByLabel('Fecha de referencia', { exact: true })
    .fill(new Date(scheduled.seconds * 1000).toISOString().slice(0, 10));
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('+00:00');
  await page
    .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
    .selectOption('precautionary_hearing');
  const waiting = responseTo(page, '/api/v1/agenda');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  const response = await waiting;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  const pageResult = await response.json();
  expect(pageResult.complete).toBe(true);
  const found = pageResult.items.filter(
    (row) => row.kind === 'precautionary_hearing' && row.precautionary_hearing.id === id,
  );
  expect(found).toHaveLength(1);
  expect(found[0].precautionary_hearing).toEqual({
    case_id: scenario.case.id,
    id,
    revision: 1,
    purpose: values.purpose,
    scheduled_at: values.scheduled_at,
    modality: values.modality,
    status: 'scheduled',
    participant_count: values.participants.length,
    capture_digest: hearing.capture.capture_digest,
  });
  expect(found[0].at).toEqual({
    unix_seconds: scheduled.seconds,
    nanosecond: 0,
    offset_seconds: 0,
  });
  const reading = responseTo(page, hearingExact(scenario, hearing));
  await page
    .getByRole('button', { name: `Consultar audiencia cautelar ${id}`, exact: true })
    .click();
  expect(await (await reading).json()).toEqual(hearing);
  await expect(hearingDetail(page)).toBeVisible();
  await capture(page, testInfo, `precautionary-agenda-${name}`);
  await hearingDetail(page)
    .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
    .click();
  let alerts;
  await expect
    .poll(
      async () => {
        alerts = await hearingAlerts(call, id);
        return alerts.length;
      },
      { timeout: 60000, intervals: [250, 500, 1000] },
    )
    .toBe(1);
  const alert = alerts[0];
  expect(alert).toMatchObject({
    recipient_id: actor.id,
    subject: { kind: 'precautionary_hearing', case_id: scenario.case.id, id },
    origin: { revision: 1, evidence_digest: hearing.capture.capture_digest },
    kind: {
      kind: 'upcoming',
      lead_hours: 48,
      activity_at: { unix_seconds: scheduled.seconds, nanosecond: 0, offset_seconds: 0 },
    },
    state: { kind: 'active' },
    email: { kind: 'disabled' },
    read_at: null,
  });
  await navigate(page, 'Alertas');
  await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
  const card = page.locator(`[data-alert-id="${alert.id}"]`);
  for (let index = 0; (await card.count()) === 0 && index < 10; index++) {
    const next = page.getByRole('button', { name: 'Cargar m\u00e1s alertas', exact: true });
    await expect(next).toBeVisible();
    const loading = responseTo(page, '/api/v1/alerts');
    await next.click();
    expect((await loading).status()).toBe(200);
    await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
  }
  await expect(card).toContainText('48 horas antes');
  const original = responseTo(page, hearingExact(scenario, hearing));
  await card.getByRole('button', { name: 'Abrir audiencia cautelar', exact: true }).click();
  const exact = await original;
  expect(exact.status()).toBe(200);
  expect(exact.headers()['cache-control']).toBe('no-store');
  expect(await exact.json()).toEqual(hearing);
  await hearingDetail(page).getByText('Autor y recibo original', { exact: true }).click();
  await expect(hearingDetail(page)).toContainText(hearing.capture.capture_digest);
  await capture(page, testInfo, `precautionary-alert-${name}`);
  await hearingDetail(page)
    .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
    .click();
  const marking = responseTo(page, `/api/v1/alerts/${alert.id}/read`, 'POST');
  await card.getByRole('button', { name: 'Marcar como le\u00edda', exact: true }).click();
  const marked = await marking;
  expect(marked.status()).toBe(200);
  const receipt = await marked.json();
  expect(receipt.alert.read_at).not.toBeNull();
  expect(receipt.alert).toEqual({ ...alert, read_at: receipt.alert.read_at });
  await expect(card.getByText('Le\u00edda', { exact: true })).toBeVisible();
  return { agenda: found[0], alert, receipt };
}
