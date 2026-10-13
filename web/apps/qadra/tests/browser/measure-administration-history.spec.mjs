import { test, expect } from '@playwright/test';
import {
  setupMeasureAdministrations,
  openMeasures,
  measurePanel,
  administrationDetail,
  administrativeActions,
  clone,
} from './measure-administration-helpers.mjs';

test('historical measure consultation retains original rectification access without offering another mutation', async ({
  page,
}) => {
  const state = await setupMeasureAdministrations(page);
  const values = state.target.record.capture.result.values;
  const review = state.prepare({
    case_id: state.caseId,
    operation_id: 'c5000000-0000-4000-8000-000000000001',
    target: clone(state.target.reference),
    context: clone(state.context.expectation),
    reason: 'Correccion registrada antes de consultar la historia',
    action: {
      kind: 'correct',
      values: {
        conditions: 'Condiciones actuales rectificadas',
        validity: clone(values.validity),
        supervision_text: values.supervision.reason,
      },
    },
  });
  const receipt = state.commit(review);
  await openMeasures(page);
  const panel = measurePanel(page);
  await panel
    .getByRole('button', { name: `Consultar medida ${state.target.reference.id}`, exact: true })
    .click();
  await expect(
    panel.getByRole('button', { name: 'Rectificar registro', exact: true }),
  ).toBeEnabled();
  await panel.getByText('Historia de la medida', { exact: true }).click();
  await panel.getByRole('button', { name: 'Consultar medida revision 1', exact: true }).click();
  for (const name of Object.values(administrativeActions))
    await expect(panel.getByRole('button', { name, exact: true })).toBeDisabled();
  await expect(
    panel.getByRole('region', { name: 'Registro exacto de medida', exact: true }),
  ).toContainText(values.conditions);
  await panel
    .getByRole('button', {
      name: `Consultar rectificacion ${receipt.origin.operation_id}`,
      exact: true,
    })
    .click();
  await expect(administrationDetail(page)).toContainText(review.command.reason);
  await expect(administrationDetail(page)).toContainText(receipt.origin.capture_digest);
  expect(state.calls.filter((call) => call.method !== 'GET')).toEqual([]);
});
