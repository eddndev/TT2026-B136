import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  deadlineEditorSetup,
  checkDeadlineEditorRequests,
  holdDeadlineRead,
  casePath,
  profilesPath,
  initialDeadline,
} from './session-deadline-editor-fixtures.mjs';
import {
  editor,
  rawDeadline,
  titleField,
  prepareButton,
  confirmButton,
  enterDeadlines,
  beginDeadline,
  fillOrdinaryDeadline,
  prepareDeadline,
} from './session-deadline-editor-ui.mjs';

test.afterEach(async ({ page }) => checkDeadlineEditorRequests(page));

test('an ordinary deadline restores raw source declarations and a partial offset only after fresh case authority', async ({
  page,
}) => {
  const state = await deadlineEditorSetup(page);
  await login(page);
  await enterDeadlines(page);
  let form = await beginDeadline(page);
  await fillOrdinaryDeadline(page);
  await form
    .getByLabel('Motivo de fuente no identificada', { exact: true })
    .fill(rawDeadline.source);
  await form
    .getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })
    .fill(rawDeadline.statement);
  await form.getByLabel('Localizador de aplicabilidad', { exact: true }).fill(rawDeadline.locator);
  await form.getByLabel('Declarar un inicio calificado', { exact: true }).check();
  await form
    .getByRole('combobox', { name: 'Finalidad del inicio', exact: true })
    .selectOption('ordered_period_start');
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de inicio calificado', exact: true })
    .selectOption('minute');
  await form.getByLabel('Fecha de inicio calificado', { exact: true }).fill('2026-10-02');
  await form.getByLabel('Hora de inicio calificado', { exact: true }).fill('11:23');
  await form
    .getByRole('combobox', { name: 'Desfase de inicio calificado', exact: true })
    .selectOption('declared');
  await form.getByLabel('Desfase UTC de inicio calificado', { exact: true }).fill('-0');
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterDeadlines(page);
  const reads = state.calls.length,
    fresh = holdDeadlineRead(state, 'GET', casePath);
  form = await beginDeadline(page);
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(titleField(form)).toHaveValue('');
  await expect(prepareButton(form)).toBeDisabled();
  fresh.release();
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(form.getByLabel('Motivo de fuente no identificada', { exact: true })).toHaveValue(
    rawDeadline.source,
  );
  await expect(form.getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })).toHaveValue(
    rawDeadline.statement,
  );
  await expect(form.getByLabel('Localizador de aplicabilidad', { exact: true })).toHaveValue(
    rawDeadline.locator,
  );
  await expect(form.getByLabel('Fecha de inicio calificado', { exact: true })).toHaveValue(
    '2026-10-02',
  );
  await expect(form.getByLabel('Hora de inicio calificado', { exact: true })).toHaveValue('11:23');
  await expect(form.getByLabel('Desfase UTC de inicio calificado', { exact: true })).toHaveValue(
    '-0',
  );
  await expect(
    form.getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true }),
  ).toHaveValue('fixed');
  await expect(
    form.getByRole('region', { name: 'Responsable seleccionado', exact: true }),
  ).toContainText('staff@example.test');
  await expect
    .poll(
      () =>
        state.calls
          .slice(reads)
          .filter(
            (row) =>
              row.method === 'GET' &&
              row.path === `${profilesPath}/${state.profiles[0].id}/revisions/1`,
          ).length,
    )
    .toBeGreaterThan(0);
  expect(state.deadlinePrepares).toEqual([]);
  expect(state.deadlinePosts).toEqual([]);
});

test('an attention draft retains its exact time inputs and raw explanation without inventing a valid offset', async ({
  page,
}) => {
  const original = initialDeadline(),
    state = await deadlineEditorSetup(page, { deadlines: [original] });
  await login(page);
  await enterDeadlines(page);
  let form = await beginDeadline(page, 'set_attention', original);
  await form
    .getByRole('combobox', { name: 'Estado de atenci\u00f3n', exact: true })
    .selectOption('recorded');
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de atenci\u00f3n', exact: true })
    .selectOption('second');
  await form.getByLabel('Fecha de atenci\u00f3n', { exact: true }).fill('2026-10-02');
  await form.getByLabel('Hora de atenci\u00f3n', { exact: true }).fill('11:23:04');
  await form
    .getByLabel('Declaraci\u00f3n de atenci\u00f3n', { exact: true })
    .fill(rawDeadline.statement);
  await form.getByLabel('Localizador de atenci\u00f3n', { exact: true }).fill(rawDeadline.locator);
  await form.getByLabel('Motivo', { exact: true }).fill(rawDeadline.reason);
  await form
    .getByRole('combobox', { name: 'Desfase de atenci\u00f3n', exact: true })
    .selectOption('declared');
  await form.getByLabel('Desfase UTC de atenci\u00f3n', { exact: true }).fill('-0');
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterDeadlines(page);
  form = await beginDeadline(page, 'set_attention', original);
  await expect(
    form.getByRole('combobox', { name: 'Estado de atenci\u00f3n', exact: true }),
  ).toHaveValue('recorded');
  await expect(
    form.getByRole('combobox', { name: 'Precisi\u00f3n de atenci\u00f3n', exact: true }),
  ).toHaveValue('second');
  await expect(form.getByLabel('Fecha de atenci\u00f3n', { exact: true })).toHaveValue(
    '2026-10-02',
  );
  await expect(form.getByLabel('Hora de atenci\u00f3n', { exact: true })).toHaveValue('11:23:04');
  await expect(form.getByLabel('Desfase UTC de atenci\u00f3n', { exact: true })).toHaveValue('-0');
  await expect(form.getByLabel('Desfase UTC de atenci\u00f3n', { exact: true })).toHaveAttribute(
    'aria-invalid',
    'true',
  );
  await expect(form.getByLabel('Declaraci\u00f3n de atenci\u00f3n', { exact: true })).toHaveValue(
    rawDeadline.statement,
  );
  await expect(form.getByLabel('Localizador de atenci\u00f3n', { exact: true })).toHaveValue(
    rawDeadline.locator,
  );
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawDeadline.reason);
  expect(state.deadlinePrepares).toEqual([]);
  expect(state.deadlinePosts).toEqual([]);
});

test('a prepared correction keeps its original base and raw draft until explicit comparison adopts a concurrent deadline revision', async ({
  page,
}) => {
  const original = initialDeadline(),
    state = await deadlineEditorSetup(page, { deadlines: [original] });
  await login(page);
  await enterDeadlines(page);
  let form = await beginDeadline(page, 'correct', original);
  await titleField(form).fill(rawDeadline.title);
  await form.getByLabel('Motivo', { exact: true }).fill(rawDeadline.reason);
  await prepareDeadline(state, form);
  const first = structuredClone(state.deadlinePrepares[0].values);
  await expire(page, state, await form.elementHandle());
  const other = structuredClone(first);
  other.operation_id = 'f0000000-0000-4000-8000-000000000077';
  other.change.definition.title = 'Titulo de otra revision';
  state.deadlineCommit(state.deadlinePrepare(other));
  await login(page);
  await enterDeadlines(page);
  form = await beginDeadline(page, 'correct', original);
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawDeadline.reason);
  await expect(confirmButton(form)).toHaveCount(0);
  await expect(prepareButton(form)).toBeDisabled();
  expect(state.deadlinePrepares).toHaveLength(1);
  await form.getByRole('button', { name: 'Consultar base actual', exact: true }).click();
  await expect(
    form.getByRole('region', { name: 'Comparar base del plazo', exact: true }),
  ).toContainText('Titulo de otra revision');
  await expect(prepareButton(form)).toBeDisabled();
  await form
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await prepareDeadline(state, form, false);
  const next = state.deadlinePrepares[1].values;
  expect(next.deadline_id).toBe(first.deadline_id);
  expect(next.operation_id).not.toBe(first.operation_id);
  expect(next.change.expected_revision).toBe(2);
  expect(next.change.definition.title).toBe(rawDeadline.title.trim());
  expect(next.change.reason).toBe(rawDeadline.reason.trim());
  expect(state.deadlinePosts).toEqual([]);
});
