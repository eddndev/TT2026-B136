import { test, expect } from '@playwright/test';
import {
  setupHearings,
  openHearings,
  hearingEditor,
  confirmHearing,
  fillHearing,
} from './hearing-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';

test('a concurrent revision requires comparison while retaining the operators changed fields', async ({
  page,
}) => {
  const record = hearingRecord(),
    { state } = await setupHearings(page, { records: [record] });
  await openHearings(page);
  await page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }).click();
  await page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
  const editor = hearingEditor(page);
  await editor.getByLabel('Sede o conexi\u00f3n', { exact: true }).fill('Borrador del operador');
  await editor.getByLabel('Motivo del cambio', { exact: true }).fill('Correccion comunicada');
  await editor.getByRole('button', { name: 'Revisar registro', exact: true }).click();
  state.records.get(record.id).push({
    ...structuredClone(record),
    revision: 2,
    values: { ...record.values, venue: 'Cambio concurrente' },
  });
  state.context.case_revision = 2;
  state.context.stage_revision = 2;
  await editor.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(editor.getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
    'Borrador del operador',
  );
  await editor
    .getByRole('button', { name: 'Consultar audiencia y contexto actuales', exact: true })
    .click();
  await expect(
    editor.getByRole('region', { name: 'Registro consultado para comparar' }),
  ).toContainText('Cambio concurrente');
  await editor
    .getByRole('button', { name: 'Usar base consultada y revisar borrador', exact: true })
    .click();
  await confirmHearing(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.submissions[0].change.expected_revision).toBe(2);
  expect(state.submissions[0].change.expected_case_revision).toBe(2);
  expect(state.submissions[0].change.expected_stage_revision).toBe(2);
  expect(state.submissions[0].change.values.venue).toBe('Borrador del operador');
});

test('closure preserves the draft and disables modifications without calling cancellation', async ({
  page,
}) => {
  const { state } = await setupHearings(page);
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  state.context.administrative_status = 'closed';
  state.admin.administration.administrative_status = 'closed';
  await hearingEditor(page).getByRole('button', { name: 'Revisar registro', exact: true }).click();
  await expect(hearingEditor(page)).toContainText('Tu formulario se conserva');
  await expect(hearingEditor(page).getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
    'Sala privada declarada',
  );
  await expect(
    hearingEditor(page).getByRole('button', { name: 'Revisar registro', exact: true }),
  ).toBeDisabled();
  expect(state.submissions).toHaveLength(0);
});

test('a stage change after preparation preserves the draft and requires a new explicit context', async ({
  page,
}) => {
  const { state } = await setupHearings(page);
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  const editor = hearingEditor(page);
  await editor.getByRole('button', { name: 'Revisar registro', exact: true }).click();
  state.context.stage_revision = 2;
  state.handle = async (route, call) => {
    if (
      call.method === 'POST' &&
      call.path.endsWith('/hearings') &&
      call.body.command.change.expected_stage_revision === 1
    ) {
      await route.fulfill({ status: 409, json: { error: { code: 'hearing_context_conflict' } } });
      return true;
    }
    return false;
  };
  await editor.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await editor
    .getByRole('button', { name: 'Consultar audiencia y contexto actuales', exact: true })
    .click();
  await editor
    .getByRole('button', { name: 'Usar base consultada y revisar borrador', exact: true })
    .click();
  await confirmHearing(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.expected_stage_revision).toBe(2);
});

test('an exhausted revision keeps the draft and cannot prepare another write', async ({ page }) => {
  const record = hearingRecord(),
    { state } = await setupHearings(page, { records: [record] });
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/prepare')) return false;
    await route.fulfill({ status: 422, json: { error: { code: 'hearing_revision_exhausted' } } });
    return true;
  };
  await openHearings(page);
  await page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }).click();
  await page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
  const editor = hearingEditor(page);
  await editor.getByLabel('Motivo del cambio', { exact: true }).fill('Correccion pendiente');
  await editor.getByRole('button', { name: 'Revisar registro', exact: true }).click();
  await expect(editor).toContainText('Se agotaron las revisiones');
  await expect(editor.getByLabel('Motivo del cambio', { exact: true })).toHaveValue(
    'Correccion pendiente',
  );
  await expect(
    editor.getByRole('button', { name: 'Revisar registro', exact: true }),
  ).toBeDisabled();
  expect(state.submissions).toHaveLength(0);
});
