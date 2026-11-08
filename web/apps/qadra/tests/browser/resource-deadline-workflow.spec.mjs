import { test, expect } from '@playwright/test';
import { activityDetail } from './resource-activities-helpers.mjs';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import {
  setupResourceDeadline,
  openResourceDeadline,
  prepareResourceDeadline,
  submitResourceDeadline,
  editor,
} from './resource-deadline-helpers.mjs';
for (const width of [1440, 390])
  test(`creates one contextual deadline and exact historical act atomically at ${width}px`, async ({
    page,
  }, info) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupResourceDeadline(page);
    await openResourceDeadline(page, state);
    await editor(page)
      .getByRole('button', { name: 'Elegir acto del recurso', exact: true })
      .click();
    await editor(page)
      .getByRole('combobox', { name: 'Acto del recurso', exact: true })
      .selectOption('2');
    await editor(page).getByRole('button', { name: 'Usar este acto', exact: true }).click();
    await prepareResourceDeadline(page);
    await expect(editor(page)).toContainText('Interposicion original declarada');
    await expect(editor(page)).toContainText('Plazo contextual');
    await page.screenshot({
      path: info.outputPath('resource-deadline-review.png'),
      fullPage: true,
    });
    await submitResourceDeadline(page);
    await expect(editor(page)).toHaveCount(0);
    await expect(activityDetail(page)).toContainText('Plazo contextual');
    expect(state.submissions).toHaveLength(1);
    const command = state.submissions[0].command;
    expect(command.resource.revision).toBe(3);
    expect(command.act.resource_revision).toBe(2);
    expect(command.act.revision).toBe(1);
    expect(state.deadlines.submissions).toHaveLength(0);
    expect(state.activities.submissions).toHaveLength(0);
    const result = state.activities.records.get(command.association_id)[0];
    expect(result.receipt.operation_id).toBe(command.deadline.operation_id);
    expect(result.selection.target.id).toBe(command.deadline.deadline_id);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({
      path: info.outputPath('resource-deadline-created.png'),
      fullPage: true,
    });
  });
test('keeps the declared deadline and old resource capture when accepting a changed current head', async ({
  page,
}) => {
  const state = await setupResourceDeadline(page);
  await openResourceDeadline(page, state);
  const correction = resourceCommandFixture('correct');
  correction.resource_id = state.activities.resource.id;
  correction.change.values = structuredClone(state.activities.resource.values);
  correction.change.values.title = 'Recurso concurrente';
  state.activities.resources.commit(state.activities.resources.prepare(correction));
  await editor(page).getByRole('button', { name: 'Preparar plazo y vinculo', exact: true }).click();
  await editor(page)
    .getByRole('button', { name: 'Comparar con registro actual', exact: true })
    .click();
  await expect(editor(page)).toContainText('Recurso concurrente');
  await editor(page)
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  await expect(editor(page).getByLabel('T\u00edtulo del plazo', { exact: true })).toHaveValue(
    'Plazo contextual',
  );
  await prepareResourceDeadline(page);
  const command = state.calls.at(-1).body;
  expect(command.expected_resource_revision).toBe(4);
  expect(command.resource.revision).toBe(3);
  expect(command.deadline.change.definition.title).toBe('Plazo contextual');
  await submitResourceDeadline(page);
  await expect(editor(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
});
