import { test, expect } from '@playwright/test';
import {
  setupMeasureDecisions,
  openMeasures,
  newDecision,
  fillDecisionCommon,
  prepareDecision,
  decisionEditor,
  clone,
} from './measure-decision-helpers.mjs';

async function chooseExact(effect, reference) {
  await effect.getByRole('button', { name: 'Elegir medida', exact: true }).click();
  const picker = effect.getByRole('region', { name: 'Seleccionar medida exacta', exact: true });
  await picker
    .getByRole('button', { name: `Consultar medida ${reference.id}`, exact: true })
    .click();
  await expect(picker).toContainText(reference.capture_digest);
  await picker.getByRole('button', { name: 'Vincular esta revision', exact: true }).click();
  await expect(picker).toHaveCount(0);
}

test('explicitly replacing a modification base resets displayed raw dates to the chosen revision', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page, { seeded: true }),
    original = state.seed.detail;
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  const effect = decisionEditor(page).getByRole('group', { name: 'Efecto 1', exact: true });
  await effect
    .getByRole('combobox', { name: 'Accion de medida 1', exact: true })
    .selectOption('modify');
  await chooseExact(effect, original.reference);
  const date = effect.getByLabel('Fecha de inicio de vigencia', { exact: true });
  await date.fill('2026-02-03');
  await expect(date).toHaveValue('2026-02-03');

  const later = clone(state.seed.operation.group.review.command),
    values = clone(original.record.capture.result.values);
  later.operation_id = 'b9000000-0000-4000-8000-000000000011';
  later.decision_id = 'b9000000-0000-4000-8000-000000000012';
  values.validity.start = {
    precision: 'date',
    year: 2026,
    month: 2,
    day: 15,
    offset_seconds: null,
  };
  later.outcome = {
    kind: 'changes',
    effects: [{ action: 'modify', previous: original.reference, values }],
  };
  state.commit(state.prepare(later));
  const current = state.records.get(original.reference.id).at(-1);
  expect(current.reference.revision).toBe(2);
  await chooseExact(effect, current.reference);
  await expect(date).toHaveValue('2026-02-15');
  expect(state.calls.filter((call) => call.method === 'POST')).toEqual([]);
  await prepareDecision(page);
  expect(state.preparations[0].review.command.outcome.effects[0]).toMatchObject({
    action: 'modify',
    previous: current.reference,
    values: { validity: { start: values.validity.start } },
  });
  expect(state.submissions).toEqual([]);
  expect(state.unexpected).toEqual([]);
});
