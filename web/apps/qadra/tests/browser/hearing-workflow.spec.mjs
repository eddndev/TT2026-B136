import { test, expect } from '@playwright/test';
import {
  setupHearings,
  openHearings,
  hearingEditor,
  fillHearing,
  confirmHearing,
  participant,
} from './hearing-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';

test('programs a hearing with exact participants and preserves local time through replacement and cancellation', async ({
  page,
}) => {
  const { state } = await setupHearings(page);
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  const editor = hearingEditor(page);
  await editor.getByRole('button', { name: 'Elegir participante', exact: true }).click();
  await editor
    .getByRole('button', { name: `Consultar ficha: ${participant.display_name}`, exact: true })
    .click();
  await editor.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
  await confirmHearing(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.submissions[0].change.values.participants).toEqual([
    { participant_id: participant.id, revision: 1 },
  ]);
  await expect(
    page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
  ).toContainText('09:02:03 / UTC-06:00');
  await page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
  await editor.getByLabel('Fecha', { exact: true }).fill('2026-10-02');
  await editor.getByLabel('Motivo del cambio', { exact: true }).fill('Nueva fecha comunicada');
  await confirmHearing(page);
  await expect(editor).toHaveCount(0);
  state.context.stage = 'trial';
  state.context.stage_revision = 2;
  await page.getByRole('button', { name: 'Cancelar audiencia', exact: true }).click();
  await editor
    .getByLabel('Motivo de cancelaci\u00f3n', { exact: true })
    .fill('Cancelacion comunicada');
  await confirmHearing(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[2].change).toEqual({
    action: 'cancel',
    expected_revision: 2,
    reason: 'Cancelacion comunicada',
  });
  await expect(
    page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
  ).toContainText('Cancelada');
  await page.getByRole('button', { name: 'Ver historial de audiencias', exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'Historial de audiencia', exact: true }),
  ).toContainText('Nueva fecha comunicada');
});

test('a historical date stays scheduled and list summaries do not expose venue or notes', async ({
  page,
}) => {
  const record = hearingRecord();
  record.values.scheduled_at = '1900-01-01T10:00:00-06:00';
  await setupHearings(page, { records: [record] });
  await openHearings(page);
  const list = page.getByRole('region', { name: 'Audiencias registradas', exact: true });
  await expect(list).toContainText('Programada');
  await expect(list).not.toContainText('Celebrada');
  await expect(list).not.toContainText(record.values.venue);
  await expect(list).not.toContainText(record.values.note);
});

test('case pagination uses the applied status until the operator applies a new filter', async ({
  page,
}) => {
  const first = hearingRecord(),
    second = { ...structuredClone(first), id: '30000000-0000-4000-8000-000000000003' };
  const { state } = await setupHearings(page, { records: [first, second] });
  state.pageSize = 1;
  await openHearings(page);
  await page
    .getByRole('combobox', { name: 'Estado de audiencia', exact: true })
    .selectOption('cancelled');
  await page.getByRole('button', { name: 'Siguiente', exact: true }).click();
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${second.id}`, exact: true }),
  ).toBeVisible();
  expect(state.calls.filter((row) => row.search.includes('after_id')).at(-1).search).toContain(
    'status=all',
  );
  await page.getByRole('button', { name: 'Aplicar estado', exact: true }).click();
  await expect(
    page.getByText('No hay audiencias en esta consulta.', { exact: true }),
  ).toBeVisible();
  expect(state.calls.at(-1).search).toContain('status=cancelled');
  expect(state.calls.at(-1).search).not.toContain('after_id');
});

test('history follows the descending exclusive revision cursor', async ({ page }) => {
  const first = hearingRecord(),
    { state } = await setupHearings(page, { records: [first] });
  state.records
    .get(first.id)
    .push({ ...structuredClone(first), revision: 2, reason: 'Registro posterior' });
  state.pageSize = 1;
  await openHearings(page);
  await page.getByRole('button', { name: `Consultar audiencia ${first.id}`, exact: true }).click();
  await page.getByRole('button', { name: 'Ver historial de audiencias', exact: true }).click();
  const history = page.getByRole('region', { name: 'Historial de audiencia', exact: true });
  await expect(history).toContainText('Revisi\u00f3n 2');
  await history.getByRole('button', { name: 'Cargar revisiones anteriores', exact: true }).click();
  await expect(history).toContainText('Revisi\u00f3n 1');
  expect(state.calls.filter((row) => row.path.endsWith('/history')).at(-1).search).toContain(
    'before_revision=2',
  );
});
