import { test, expect } from '@playwright/test';
import { resultDetail } from './hearing-result-helpers.mjs';
import {
  setupDerivedDeadline,
  openDerivedDeadline,
  fillDerivedDeadline,
  prepareDerivedDeadline,
  submitDerivedDeadline,
  derivedEditor,
} from './hearing-derived-deadline-helpers.mjs';
import { hearingCaseId, hearingId } from '../fixtures/hearings.mjs';

const endpoint = `/api/v1/cases/${hearingCaseId}/hearings/${hearingId}/results/derived-deadline`;

for (const width of [1440, 390])
  test(`creates one result and its blocked deadline through the compound route at ${width}px`, async ({
    page,
  }, info) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupDerivedDeadline(page);
    await openDerivedDeadline(page);
    await fillDerivedDeadline(page);
    await prepareDerivedDeadline(page);
    await expect(derivedEditor(page)).toContainText('Resultado con plazo declarado');
    await expect(derivedEditor(page)).toContainText('Respuesta al resultado');
    expect(
      state.results.calls.filter((call) => /\/results\/[^/]+\/revisions\/1$/.test(call.path)),
    ).toHaveLength(0);
    await page.screenshot({ path: info.outputPath('derived-deadline-review.png'), fullPage: true });
    await submitDerivedDeadline(page);
    await expect(derivedEditor(page)).toHaveCount(0);
    await expect(resultDetail(page)).toContainText('Resultado con plazo declarado');
    expect(state.calls.map((call) => [call.method, call.path])).toEqual([
      ['POST', `${endpoint}/prepare`],
      ['POST', `${endpoint}/submit`],
    ]);
    expect(state.submissions).toHaveLength(1);
    expect(state.results.submissions).toHaveLength(0);
    expect(state.deadlines.submissions).toHaveLength(0);
    expect(state.results.records.size).toBe(1);
    expect(state.deadlines.records.size).toBe(1);
    const command = state.submissions[0].command;
    const source = command.deadline.change.definition.input.selection.source;
    expect(source).toEqual({
      kind: 'known',
      value: {
        family: 'hearing_result',
        hearing_id: hearingId,
        result_id: command.result.result_id,
        revision: 1,
        agreement_id: null,
      },
    });
    const record = state.committed.get(command.result.operation_id);
    expect(record.result.id).toBe(command.result.result_id);
    expect(record.deadline.id).toBe(command.deadline.deadline_id);
    expect(record.origin.result_operation_id).toBe(command.result.operation_id);
    expect(record.origin.deadline_operation_id).toBe(command.deadline.operation_id);
    expect(record.deadline.calculation.result.due_at).toBeNull();
    expect(record.deadline.calculation.result.blocks).toHaveLength(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({
      path: info.outputPath('derived-deadline-created.png'),
      fullPage: true,
    });
  });

test('recovers a lost compound response by explicit prepare Replay without a second submit', async ({
  page,
}) => {
  const state = await setupDerivedDeadline(page);
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/submit')) return false;
    state.submissions.push(structuredClone(call.body));
    state.commit(state.prepare(call.body.command));
    await route.abort('failed');
    return true;
  };
  await openDerivedDeadline(page);
  await fillDerivedDeadline(page);
  await prepareDerivedDeadline(page);
  await submitDerivedDeadline(page);
  const form = derivedEditor(page);
  await expect(
    form.getByRole('button', { name: 'Consultar envio exacto', exact: true }),
  ).toBeVisible();
  expect(state.calls.map((call) => call.path)).toEqual([
    `${endpoint}/prepare`,
    `${endpoint}/submit`,
  ]);
  expect(state.submissions).toHaveLength(1);
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(form).toHaveCount(0);
  await expect(resultDetail(page)).toContainText('Resultado con plazo declarado');
  expect(state.calls.map((call) => call.path)).toEqual([
    `${endpoint}/prepare`,
    `${endpoint}/submit`,
    `${endpoint}/prepare`,
  ]);
  expect(state.calls[2].body).toEqual(state.calls[0].body);
  expect(state.submissions).toHaveLength(1);
  expect(state.committed.size).toBe(1);
  expect(state.results.records.size).toBe(1);
  expect(state.deadlines.records.size).toBe(1);
  expect(state.results.submissions).toHaveLength(0);
  expect(state.deadlines.submissions).toHaveLength(0);
});
