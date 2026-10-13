import { test, expect } from '@playwright/test';
import {
  setupMeasureAdministrations,
  openMeasures,
  openAdministration,
  fillAdministration,
  prepareAdministration,
  submitAdministration,
  administrationEditor,
  administrationDetail,
  measurePanel,
  administrativeActions,
  clone,
} from './measure-administration-helpers.mjs';

for (const action of ['correct', 'entered_in_error', 'replace_entered_in_error'])
  test(`${action} records an explicit administrative change while retaining its original judicial origin`, async ({
    page,
  }) => {
    const state = await setupMeasureAdministrations(page);
    const target = state.target;
    await openAdministration(page, state, action);
    await fillAdministration(page, action);
    const editor = administrationEditor(page);
    if (action !== 'correct')
      await expect(editor.getByLabel('Condiciones', { exact: true })).toHaveCount(0);
    await prepareAdministration(page);
    await expect(editor).toContainText('Rectificacion administrativa declarada en soporte');
    if (action === 'correct')
      await expect(editor).toContainText('Texto de condiciones rectificado expresamente');
    if (action === 'replace_entered_in_error')
      await expect(editor).toContainText('Persona sustituta declarada');
    await submitAdministration(page);
    await expect(editor).toHaveCount(0);
    const detail = administrationDetail(page);
    await expect(detail).toContainText('Rectificacion administrativa declarada en soporte');
    await expect(detail).toContainText(target.reference.id);
    await expect(detail).toContainText(target.judicial_origin.operation_id);
    await expect(detail).toContainText(target.judicial_origin.decision_id);
    const sent = state.submissions[0],
      command = sent.command;
    expect(command.target).toEqual(target.reference);
    expect(command.action.kind).toBe(action);
    expect(sent.expected_submission_digest).toBe(state.preparations[0].submission_digest);
    expect(sent.expected_review_digest).toBe(state.preparations[0].review_digest);
    const original = target.record.capture.result;
    const changed = state.records.get(target.reference.id).at(-1);
    expect(changed.family).toBe('c1');
    expect(changed.reference.revision).toBe(2);
    expect(changed.judicial_origin).toEqual(target.judicial_origin);
    expect(changed.last_judicial).toEqual(target.last_judicial);
    expect(changed.last_action).toBe(target.last_action);
    expect(changed.record.capture.result.values.subject).toEqual(original.values.subject);
    expect(changed.record.capture.result.values.kind).toBe(original.values.kind);
    const operation = state.operations.get(command.operation_id);
    expect(operation.record_history).toEqual(target.record_history);
    if (action === 'correct') {
      expect(command.action.values).toEqual({
        conditions: 'Texto de condiciones rectificado expresamente',
        validity: {
          ...original.values.validity,
          statement: 'Vigencia textual rectificada sin completar el horario',
        },
        supervision_text: 'Texto de supervision rectificado sin nueva ficha',
      });
      expect(command.action.values.validity.start).toEqual({
        precision: 'date',
        year: 2026,
        month: 1,
        day: 2,
        offset_seconds: null,
      });
      expect(command.action.values.validity.end).toBeNull();
      expect(changed.validity).toBe('valid');
      expect(changed.record.capture.result.values.conditions).toBe(
        command.action.values.conditions,
      );
      expect(operation.capture.replacement_link).toBeNull();
      expect(operation.capture.records).toHaveLength(1);
    } else {
      expect(changed.validity).toBe('entered_in_error');
      expect(changed.record.capture.result.values).toEqual(original.values);
      if (action === 'entered_in_error') {
        expect(command.action).toEqual({ kind: action });
        expect(operation.capture.records).toHaveLength(1);
        expect(operation.capture.replacement_link).toBeNull();
      } else {
        const replacement = state.records.get(command.action.replacement_id)[0];
        expect(command.action.replacement_id).not.toBe(target.reference.id);
        expect(replacement.validity).toBe('valid');
        expect(replacement.reference.revision).toBe(1);
        expect(replacement.record_root).toEqual({
          kind: 'administrative',
          operation_id: command.operation_id,
          measure_id: command.action.replacement_id,
        });
        expect(replacement.judicial_origin).toEqual(target.judicial_origin);
        expect(replacement.last_judicial).toEqual(target.last_judicial);
        expect(replacement.record.capture.result.values.subject).toEqual(command.action.subject);
        expect(replacement.record.capture.result.sources.subject).toEqual(state.replacementSubject);
        expect(operation.capture.records).toHaveLength(2);
        expect(operation.capture.replacement_link).toEqual({
          entered_in_error: changed.reference,
          replacement: replacement.reference,
        });
        await expect(detail).toContainText('Persona sustituta declarada');
        await expect(detail).toContainText(command.action.replacement_id);
      }
    }
    for (const capture of operation.capture.records)
      expect(capture.operation_id).toBe(command.operation_id);
    expect(state.preparations).toHaveLength(1);
    expect(state.submissions).toHaveLength(1);
    expect(state.decisions.submissions).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });

test('an uncertain rectification recovers its original operation after another correction without posting again', async ({
  page,
}) => {
  const state = await setupMeasureAdministrations(page);
  state.loseResponse = true;
  await openAdministration(page, state, 'correct');
  await fillAdministration(page, 'correct');
  await prepareAdministration(page);
  await submitAdministration(page);
  const editor = administrationEditor(page);
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  const sent = clone(state.submissions[0]);
  const receipt = clone(state.operations.get(sent.command.operation_id));
  const current = clone(state.records.get(state.target.reference.id).at(-1));
  const later = clone(sent.command);
  later.operation_id = 'b8000000-0000-4000-8000-000000000003';
  later.target = clone(current.reference);
  later.reason = 'Rectificacion posterior ajena al recibo original';
  later.action.values.conditions = 'Condiciones posteriores ajenas al recibo original';
  state.commit(state.prepare(later));
  await editor.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(administrationDetail(page)).toContainText(
    'Texto de condiciones rectificado expresamente',
  );
  await expect(administrationDetail(page)).toContainText(sent.command.operation_id);
  await expect(administrationDetail(page)).not.toContainText(later.reason);
  await expect(administrationDetail(page)).not.toContainText(later.action.values.conditions);
  expect(
    state.calls.filter(
      (call) => call.method === 'GET' && call.path === `${state.base}/${sent.command.operation_id}`,
    ),
  ).toHaveLength(1);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toEqual([sent]);
  expect(state.operations.get(sent.command.operation_id)).toEqual(receipt);
  expect(state.records.get(state.target.reference.id).at(-1).reference.revision).toBe(3);
  expect(state.decisions.submissions).toEqual([]);
  expect(state.unexpected).toEqual([]);
});

for (const [role, closed] of [
  ['paralegal', false],
  ['owner', true],
])
  test(`${role} reads a measure with closed=${closed} without enabling administrative mutations`, async ({
    page,
  }) => {
    const state = await setupMeasureAdministrations(page, { role, closed });
    await openMeasures(page);
    const panel = measurePanel(page);
    await panel
      .getByRole('button', { name: `Consultar medida ${state.target.reference.id}`, exact: true })
      .click();
    await expect(panel).toContainText('Persona declarada');
    await expect(panel).toContainText(state.target.record.capture.result.values.conditions);
    for (const name of Object.values(administrativeActions))
      await expect(
        page.getByRole('button', { name, exact: true }).and(page.locator(':enabled')),
      ).toHaveCount(0);
    await expect(administrationEditor(page)).toHaveCount(0);
    expect(state.calls.filter((call) => call.method !== 'GET')).toEqual([]);
    expect(state.preparations).toEqual([]);
    expect(state.submissions).toEqual([]);
    expect(state.decisions.submissions).toEqual([]);
  });
