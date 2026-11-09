import { test, expect } from '@playwright/test';
import {
  setupMeasureDecisions,
  openMeasures,
  newDecision,
  fillDecisionCommon,
  fillImposition,
  prepareDecision,
  submitDecision,
  decisionEditor,
  decisionDetail,
} from './measure-decision-helpers.mjs';

test('a declared imposition retains its exact subject and support without inventing unknown times', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page);
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  await fillImposition(page);
  await prepareDecision(page);
  await expect(decisionEditor(page)).toContainText('Persona declarada');
  await expect(decisionEditor(page)).toContainText('contrato.pdf');
  await expect(decisionEditor(page)).toContainText('No consta el momento de decision');
  await submitDecision(page);
  await expect(decisionEditor(page)).toHaveCount(0);
  await expect(decisionDetail(page)).toContainText('Persona declarada');
  await expect(decisionDetail(page)).toContainText('Presentarse cada viernes segun soporte');
  const sent = state.submissions[0],
    command = sent.command;
  expect(command.values.declared_at).toEqual({
    precision: 'unknown',
    reason: 'No consta el momento de decision',
  });
  expect(command.anchor).toBeNull();
  expect(command.outcome.effects).toHaveLength(1);
  const effect = command.outcome.effects[0];
  expect(effect.action).toBe('impose');
  expect(effect.proposal.values).toMatchObject({
    subject: { id: state.subject.id, revision: 1, values_digest: state.subject.values_digest },
    kind: 'periodic_appearance',
    validity: {
      start: { precision: 'unknown', reason: 'No consta inicio de vigencia' },
      end: null,
    },
    supervision: { kind: 'unknown', reason: 'No consta autoridad supervisora' },
  });
  expect(sent.expected_submission_digest).toBe(state.preparations[0].review.submission_digest);
  expect(sent.expected_review_digest).toBe(state.preparations[0].review.review_digest);
  expect(state.operations.get(command.operation_id).family).toBe('g2');
  expect(state.submissions).toHaveLength(1);
  expect(state.unexpected).toEqual([]);
});

async function choosePriorMeasure(page, reference, action) {
  const effect = decisionEditor(page).getByRole('group', { name: 'Efecto 1', exact: true });
  await effect.getByLabel(/Acci[o\u00f3]n de medida 1/, { exact: true }).selectOption(action);
  await effect.getByRole('button', { name: 'Elegir medida', exact: true }).click();
  const picker = effect.getByRole('region', { name: 'Seleccionar medida exacta', exact: true });
  await picker
    .getByRole('button', { name: `Consultar medida ${reference.id}`, exact: true })
    .click();
  await expect(picker).toContainText(reference.capture_digest);
  await picker.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
  await expect(picker).toHaveCount(0);
  return effect;
}

test('modification and revocation retain the selected current predecessor and its judicial origin', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page, { seeded: true });
  const original = state.seed.detail,
    id = original.reference.id;
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  const effect = await choosePriorMeasure(page, original.reference, 'modify');
  await expect(effect.getByLabel('Condiciones', { exact: true })).toHaveValue(
    original.record.capture.result.values.conditions,
  );
  await effect
    .getByLabel('Condiciones', { exact: true })
    .fill('Comparecer cada lunes segun nueva decision');
  await prepareDecision(page);
  await submitDecision(page);
  await expect(decisionDetail(page)).toContainText('Comparecer cada lunes segun nueva decision');
  const changed = state.records.get(id).at(-1),
    modify = state.submissions[0].command.outcome.effects[0];
  expect(modify.action).toBe('modify');
  expect(modify.previous).toEqual(original.reference);
  expect(modify.values.subject).toEqual(original.record.capture.result.values.subject);
  expect(modify.values.kind).toBe(original.record.capture.result.values.kind);
  expect(changed.reference.revision).toBe(2);
  expect(changed.judicial_origin).toEqual(original.judicial_origin);
  expect(changed.record_history.records.judicial.groups).toEqual(
    original.record_history.records.judicial.groups,
  );
  await newDecision(page);
  await fillDecisionCommon(page);
  await choosePriorMeasure(page, changed.reference, 'revoke');
  await prepareDecision(page);
  await submitDecision(page);
  await expect(decisionDetail(page)).toContainText(/Revocaci[o\u00f3]n/);
  const revoked = state.records.get(id).at(-1);
  expect(state.submissions[1].command.outcome).toEqual({
    kind: 'changes',
    effects: [{ action: 'revoke', previous: changed.reference }],
  });
  expect(revoked.reference.revision).toBe(3);
  expect(revoked.record.capture.result.values).toEqual(changed.record.capture.result.values);
  expect(revoked.judicial_origin).toEqual(original.judicial_origin);
  for (const reference of [original.reference, changed.reference]) {
    expect(
      state.calls.some(
        (call) =>
          call.path.endsWith(`/measures/${id}/revisions/${reference.revision}`) &&
          new URLSearchParams(call.search).get('capture_digest') === reference.capture_digest,
      ),
    ).toBe(true);
  }
  expect(state.submissions).toHaveLength(2);
  expect(state.unexpected).toEqual([]);
});

test('an explicit decision without measure changes records its statement without creating a measure', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page);
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  const editor = decisionEditor(page);
  await editor
    .getByRole('combobox', { name: 'Resultado', exact: true })
    .selectOption('no_measure_change');
  await editor
    .getByLabel(/Declaraci[o\u00f3]n sin cambios/, { exact: true })
    .fill('Se resolvio expresamente sin modificar medidas');
  await expect(editor.getByRole('group', { name: 'Efecto 1', exact: true })).toHaveCount(0);
  await prepareDecision(page);
  await submitDecision(page);
  await expect(decisionDetail(page)).toContainText(
    'Se resolvio expresamente sin modificar medidas',
  );
  const command = state.submissions[0].command;
  expect(command.outcome).toEqual({
    kind: 'no_measure_change',
    statement: 'Se resolvio expresamente sin modificar medidas',
  });
  expect(state.operations.get(command.operation_id).group.measures).toEqual([]);
  expect(state.records.size).toBe(0);
  expect(state.submissions).toHaveLength(1);
});

for (const [role, closed] of [
  ['paralegal', false],
  ['owner', true],
])
  test(`${role} reads the original judicial decision with closed=${closed} without enabling changes`, async ({
    page,
  }) => {
    const state = await setupMeasureDecisions(page, { role, closed, seeded: true });
    await openMeasures(page);
    const id = state.seed.operation.origin.decision_id;
    await page
      .getByRole('button', { name: `Consultar decisi\u00f3n cautelar ${id}`, exact: true })
      .click();
    await expect(decisionDetail(page)).toContainText('Persona declarada');
    await expect(decisionDetail(page)).toContainText('contrato.pdf');
    await expect(decisionDetail(page)).toContainText(state.seed.operation.origin.operation_id);
    await expect(
      page
        .getByRole('button', { name: 'Registrar decisi\u00f3n cautelar', exact: true })
        .and(page.locator(':enabled')),
    ).toHaveCount(0);
    await expect(decisionEditor(page)).toHaveCount(0);
    expect(state.calls.filter((call) => call.method !== 'GET')).toEqual([]);
    expect(state.submissions).toEqual([]);
  });
