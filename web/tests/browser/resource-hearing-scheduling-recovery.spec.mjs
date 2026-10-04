import { test, expect } from '@playwright/test';
import {
  setupResourceHearing,
  openHearingForm,
  fillHearingForm,
  prepareHearing,
  submitHearing,
  hearingEditor,
  hearingDetail,
  hearingPanel,
  clone,
} from './resource-hearing-scheduling-helpers.mjs';

test('reconciles the original atomic creation after response loss and later unlink without another write', async ({
  page,
}) => {
  const state = await setupResourceHearing(page);
  let original;
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/submit')) return false;
    state.submissions.push(clone(call.body));
    original = state.commit(state.prepare(call.body.command));
    const unlink = {
      case_id: original.hearing.case_id,
      resource_id: original.hearing.resource_id,
      association_id: original.association.id,
      operation_id: 'f0000000-0000-4000-8000-000000000099',
      expected_resource_revision: 3,
      change: { action: 'unlink', expected_revision: 1, reason: 'Organizacion posterior' },
    };
    state.activities.commit(state.activities.prepare(unlink));
    await route.abort('failed');
    return true;
  };
  await openHearingForm(page, state);
  await fillHearingForm(page);
  await prepareHearing(page);
  await submitHearing(page);
  await expect(
    hearingEditor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  await hearingEditor(page)
    .getByRole('button', { name: 'Consultar resultado', exact: true })
    .click();
  await expect(hearingEditor(page)).toHaveCount(0);
  await expect(hearingDetail(page)).toContainText('Sala propia del recurso');
  expect(state.submissions).toHaveLength(1);
  expect(state.calls.filter((call) => call.path.endsWith('/prepare'))).toHaveLength(1);
  expect(
    state.calls.some(
      (call) =>
        call.method === 'GET' && call.path === `${state.base}/${original.hearing.id}/revisions/1`,
    ),
  ).toBe(true);
  expect(state.activities.records.get(original.association.id).at(-1).status).toBe('unlinked');
  await hearingDetail(page)
    .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
    .click();
  await hearingPanel(page)
    .getByRole('button', { name: 'Actualizar audiencias del recurso', exact: true })
    .click();
  await hearingPanel(page)
    .getByRole('button', {
      name: `Consultar audiencia de recurso ${original.hearing.id}`,
      exact: true,
    })
    .click();
  await expect(hearingDetail(page)).toContainText('Sala propia del recurso');
});

test('keeps a missing exact result uncertain until an explicit identical submission', async ({
  page,
}) => {
  const state = await setupResourceHearing(page);
  let first;
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/submit') || first) return false;
    first = clone(call.body);
    state.submissions.push(first);
    await route.abort('failed');
    return true;
  };
  await openHearingForm(page, state);
  await fillHearingForm(page);
  await prepareHearing(page);
  await submitHearing(page);
  const editor = hearingEditor(page);
  await editor.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(editor).toContainText('El resultado sigue incierto');
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.committed.size).toBe(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.calls.filter((call) => call.path.endsWith('/prepare'))).toHaveLength(1);
  await editor.getByRole('button', { name: 'Reintentar envio exacto', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(hearingDetail(page)).toBeVisible();
  expect(state.submissions).toHaveLength(2);
  expect(state.submissions[1]).toEqual(first);
  expect(state.committed.size).toBe(1);
  expect(state.calls.filter((call) => call.path.endsWith('/prepare'))).toHaveLength(1);
});
