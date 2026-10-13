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
test('matching receipts need explicit same-envelope confirmation of their joint origin', async ({
  page,
}) => {
  const state = await setupResourceDeadline(page);
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/submit') || state.submissions.length) return false;
    state.submissions.push(structuredClone(call.body));
    state.commit(state.prepare(call.body.command));
    await route.abort('failed');
    return true;
  };
  await openResourceDeadline(page, state);
  await prepareResourceDeadline(page);
  await submitResourceDeadline(page);
  await editor(page).getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(
    editor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  const confirmOrigin = editor(page).getByRole('button', {
    name: 'Confirmar origen del env\u00edo',
    exact: true,
  });
  await expect(confirmOrigin).toBeVisible();
  await confirmOrigin.click();
  await expect(editor(page)).toHaveCount(0);
  await expect(activityDetail(page)).toContainText('Plazo contextual');
  expect(state.submissions).toHaveLength(2);
  expect(state.submissions[1]).toEqual(state.submissions[0]);
  expect(state.committed.size).toBe(1);
});
test('requires explicit same-command replay after both exact records are absent', async ({
  page,
}) => {
  const state = await setupResourceDeadline(page);
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/submit') || state.submissions.length) return false;
    state.submissions.push(structuredClone(call.body));
    await route.abort('failed');
    return true;
  };
  await openResourceDeadline(page, state);
  await prepareResourceDeadline(page);
  await submitResourceDeadline(page);
  await editor(page).getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  expect(state.submissions).toHaveLength(1);
  await editor(page).getByRole('button', { name: 'Reintentar envio exacto', exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(2);
  expect(state.submissions[1]).toEqual(state.submissions[0]);
});

test('an ordinary matching pair cannot be accepted when joint-origin verification rejects it', async ({
  page,
}) => {
  const state = await setupResourceDeadline(page);
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/submit')) return false;
    state.submissions.push(structuredClone(call.body));
    if (state.submissions.length === 1) {
      const draft = state.prepare(call.body.command);
      state.commit(draft);
      state.committed.clear();
      await route.abort('failed');
    } else {
      await route.fulfill({
        status: 409,
        json: { error: { code: 'resource_activity_operation_conflict' } },
      });
    }
    return true;
  };
  await openResourceDeadline(page, state);
  await prepareResourceDeadline(page);
  await submitResourceDeadline(page);
  await editor(page).getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(
    editor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  const confirmOrigin = editor(page).getByRole('button', {
    name: 'Confirmar origen del env\u00edo',
    exact: true,
  });
  await expect(confirmOrigin).toBeVisible();
  await confirmOrigin.click();
  await expect(editor(page).getByRole('alert')).toBeVisible();
  await expect(
    editor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  await expect(activityDetail(page)).toHaveCount(0);
  await expect(page.getByText('Vinculo guardado.', { exact: true })).toHaveCount(0);
  expect(state.submissions).toHaveLength(2);
  expect(state.submissions[1]).toEqual(state.submissions[0]);
  expect(state.committed.size).toBe(0);
});
