import { test, expect } from '@playwright/test';
import {
  setupPrecautionaryScheduling,
  openHearings,
  openPrecautionaryForm,
  fillPrecautionaryForm,
  preparePrecautionary,
  submitPrecautionary,
  precautionaryEditor,
  precautionaryPanel,
  precautionaryDetail,
  clone,
} from './precautionary-hearing-scheduling-helpers.mjs';

test('case hearings schedule an imposition, replace its appointment and retain it when cancelled', async ({
  page,
}) => {
  const state = await setupPrecautionaryScheduling(page);
  await openPrecautionaryForm(page);
  await expect(
    page.getByRole('button', { name: 'Programar audiencia', exact: true }),
  ).toBeDisabled();
  await fillPrecautionaryForm(page);
  await preparePrecautionary(page);
  await expect(precautionaryEditor(page)).toContainText('Sala cautelar declarada');
  await expect(precautionaryEditor(page)).toContainText('contrato.pdf');
  await submitPrecautionary(page);
  await expect(precautionaryEditor(page)).toHaveCount(0);
  await expect(precautionaryDetail(page)).toContainText(/Revisi[o\u00f3]n exacta consultada: 1/);
  const first = state.submissions[0],
    id = first.command.hearing_id,
    r1 = clone(state.records.get(id)[0]);
  expect(first.command.change.action).toBe('schedule');
  expect(first.command.change.values).toMatchObject({
    purpose: 'imposition',
    scheduled_at: '2026-10-10T09:02:03-06:00',
    participants: [],
    review_targets: [],
    scheduling_basis: { statement: 'Senalamiento comunicado', locator: 'Pagina 2' },
  });
  await page.getByRole('button', { name: 'Reprogramar audiencia cautelar', exact: true }).click();
  await expect(precautionaryEditor(page).getByLabel('Sede o enlace', { exact: true })).toHaveValue(
    'Sala cautelar declarada',
  );
  await precautionaryEditor(page).getByLabel('Fecha', { exact: true }).fill('2026-10-11');
  await precautionaryEditor(page)
    .getByLabel('Sede o enlace', { exact: true })
    .fill('Sala cautelar reprogramada');
  await precautionaryEditor(page)
    .getByLabel('Motivo del cambio', { exact: true })
    .fill('Nuevo senalamiento comunicado');
  await preparePrecautionary(page);
  await submitPrecautionary(page);
  await expect(precautionaryDetail(page)).toContainText(/Revisi[o\u00f3]n exacta consultada: 2/);
  const second = state.submissions[1],
    r2 = clone(state.records.get(id)[1]);
  expect(second.command.change).toMatchObject({
    action: 'replace',
    expected_revision: 1,
    expected_capture_digest: r1.capture.capture_digest,
    reason: 'Nuevo senalamiento comunicado',
  });
  expect(second.command.change.values.scheduling_basis).toEqual(
    first.command.change.values.scheduling_basis,
  );
  await page.getByRole('button', { name: 'Cancelar audiencia cautelar', exact: true }).click();
  await precautionaryEditor(page)
    .getByLabel('Motivo del cambio', { exact: true })
    .fill('Convocatoria cancelada expresamente');
  await preparePrecautionary(page);
  await submitPrecautionary(page);
  const detail = precautionaryDetail(page);
  await expect(detail).toContainText(/Revisi[o\u00f3]n exacta consultada: 3/);
  await expect(detail).toContainText('Cancelada');
  await expect(detail).toContainText('Sala cautelar reprogramada');
  expect(state.submissions[2].command.change).toEqual({
    action: 'cancel',
    expected_revision: 2,
    expected_capture_digest: r2.capture.capture_digest,
    reason: 'Convocatoria cancelada expresamente',
  });
  const history = state.records.get(id).at(-1).history.captures;
  expect(history).toHaveLength(3);
  expect(history[2].review.resolved_values).toEqual(history[1].review.resolved_values);
  expect(history[2].review.sources).toEqual(history[1].review.sources);
  expect(new Set(state.submissions.map((row) => row.command.operation_id)).size).toBe(3);
  for (let index = 0; index < 3; index++) {
    expect(state.submissions[index].expected_submission_digest).toBe(
      state.preparations[index].submission_digest,
    );
    expect(state.submissions[index].expected_review_digest).toBe(
      state.preparations[index].review_digest,
    );
  }
  await detail.getByText('Historia de la convocatoria', { exact: true }).click();
  await expect(detail).toContainText('Nuevo senalamiento comunicado');
  await expect(detail).toContainText('Convocatoria cancelada expresamente');
  expect(state.submissions).toHaveLength(3);
  expect(state.unexpected).toEqual([]);
  expect(state.hearings.state.submissions).toEqual([]);
});

test('a lost precautionary submit response is recovered by its exact operation without another POST', async ({
  page,
}) => {
  const state = await setupPrecautionaryScheduling(page);
  state.loseResponse = true;
  await openPrecautionaryForm(page);
  await fillPrecautionaryForm(page);
  await preparePrecautionary(page);
  await submitPrecautionary(page);
  const editor = precautionaryEditor(page);
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  const sent = clone(state.submissions[0]),
    first = state.operations.get(sent.command.operation_id);
  const later = clone(sent.command);
  later.operation_id = 'f9000000-0000-4000-8000-000000000002';
  later.change = {
    action: 'replace',
    expected_revision: 1,
    expected_capture_digest: first.capture.capture_digest,
    context: clone(state.context.expectation),
    values: clone(sent.command.change.values),
    reason: 'Senalamiento posterior independiente',
  };
  later.change.values.venue = 'Sala posterior que no pertenece al recibo';
  state.commit(state.prepare(later));
  await editor.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(precautionaryDetail(page)).toContainText(/Revisi[o\u00f3]n exacta consultada: 1/);
  await expect(precautionaryDetail(page)).toContainText('Sala cautelar declarada');
  await expect(precautionaryDetail(page)).not.toContainText(
    'Sala posterior que no pertenece al recibo',
  );
  expect(
    state.calls.filter(
      (call) =>
        call.path === `${state.base}/operations/${sent.command.operation_id}` &&
        call.method === 'GET',
    ),
  ).toHaveLength(1);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toEqual([sent]);
  expect(state.records.get(sent.command.hearing_id).at(-1).capture.review.result_revision).toBe(2);
});

for (const [role, closed] of [
  ['paralegal', false],
  ['owner', true],
])
  test(`${role} reads precautionary history with closed=${closed} without enabling mutations`, async ({
    page,
  }) => {
    const state = await setupPrecautionaryScheduling(page, { role, closed, seeded: true });
    await openHearings(page);
    const panel = precautionaryPanel(page),
      id = state.seed.capture.review.command.hearing_id;
    await expect(panel).toBeVisible();
    await panel
      .getByRole('button', { name: `Consultar audiencia cautelar ${id}`, exact: true })
      .click();
    await expect(precautionaryDetail(page)).toContainText(/Revisi[o\u00f3]n exacta consultada: 1/);
    await expect(precautionaryDetail(page)).toContainText('contrato.pdf');
    if (closed)
      await expect(precautionaryDetail(page)).toContainText(
        'Expediente cerrado administrativamente',
      );
    for (const name of [
      'Programar audiencia cautelar',
      'Reprogramar audiencia cautelar',
      'Cancelar audiencia cautelar',
    ])
      await expect(
        page.getByRole('button', { name, exact: true }).and(page.locator(':enabled')),
      ).toHaveCount(0);
    await precautionaryDetail(page)
      .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
      .click();
    await expect(precautionaryDetail(page)).toHaveCount(0);
    await expect(
      panel.getByRole('button', { name: `Consultar audiencia cautelar ${id}`, exact: true }),
    ).toBeVisible();
    expect(state.calls.filter((call) => call.method !== 'GET')).toEqual([]);
    expect(state.submissions).toEqual([]);
  });
