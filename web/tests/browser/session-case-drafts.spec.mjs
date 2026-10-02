import { test, expect } from '@playwright/test';
import { login, navigate, caseRecord } from './helpers.mjs';
import { administration, profile } from './case-administration-helpers.mjs';
import { expire, signInOther, casePath } from './session-inactivity-helpers.mjs';
import {
  caseDraftSetup,
  checkCaseDraftRequests,
  holdRead,
  editor,
  createEditor,
  fillPartialCase,
  expectBlankCase,
  openCaseSummary,
  titleLabel,
  offenseLabel,
  rawTitle,
  rawReference,
  rawNuc,
  pendingOffense,
  caseIndexPath,
} from './session-case-drafts-helpers.mjs';

test.afterEach(async ({ page }) => checkCaseDraftRequests(page));

test('same-account MFA restores partial case creation only after explicit authorized re-entry', async ({
  page,
}) => {
  const state = await caseDraftSetup(page);
  await login(page, false, false);
  const form = await createEditor(page);
  await expire(page, state, await fillPartialCase(form));
  await login(page, true, false);
  expect(state.grants.at(-1)).toMatchObject({
    factor: 'recovery',
    user: state.grants[0].user,
  });
  await expect(editor(page)).toHaveCount(0);
  const gate = holdRead(state, caseIndexPath);
  await navigate(page, 'Expedientes');
  await expect.poll(() => gate.entered).toBe(true);
  await expect(editor(page)).toHaveCount(0);
  await expect(
    page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }),
  ).toBeDisabled();
  gate.release();
  state.gate = null;
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  const restored = editor(page);
  for (const [label, value] of [
    [titleLabel, rawTitle],
    ['Referencia interna', rawReference],
    ['NUC', rawNuc],
    [offenseLabel, pendingOffense],
  ])
    await expect(restored.getByLabel(label, { exact: true })).toHaveValue(value);
  await expect(restored.getByRole('button', { name: /^Quitar descripci/ })).toHaveCount(0);
  expect(
    state.calls.some(
      (call) =>
        call.path === caseIndexPath &&
        call.headers.authorization === `Bearer ${state.current.token}`,
    ),
  ).toBe(true);
  expect(
    state.calls.filter((call) => call.method === 'POST' && call.path === '/api/v1/penal-cases'),
  ).toEqual([]);
  const storage = await page.evaluate(() =>
    JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)]),
  );
  expect(storage).not.toContain(rawTitle);
  expect(storage).not.toContain(pendingOffense);
});

test('changing account and explicit logout discard suspended case creation drafts', async ({
  page,
}) => {
  const state = await caseDraftSetup(page);
  await login(page, false, false);
  await expire(page, state, await fillPartialCase(await createEditor(page)));
  await signInOther(page);
  await expectBlankCase(await createEditor(page));
  await editor(page).getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await login(page, false, false);
  const form = await createEditor(page);
  await expectBlankCase(form);
  await expire(page, state, await fillPartialCase(form));
  await login(page, false, false);
  await expect(editor(page)).toHaveCount(0);
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await login(page, false, false);
  await expectBlankCase(await createEditor(page));
  expect(state.confirmedWrites).toEqual([]);
});

test('resumed case editing compares a fresh revision and requires an explicit replacement decision', async ({
  page,
}) => {
  const state = await caseDraftSetup(page);
  await login(page, false, false);
  await openCaseSummary(page);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  const rawEdit = '  Cambio pendiente del expediente  ';
  await editor(page).getByLabel(titleLabel, { exact: true }).fill(rawEdit);
  await editor(page).getByLabel(offenseLabel, { exact: true }).fill(pendingOffense);
  await expire(page, state, await editor(page).elementHandle());
  state.record = administration({ ...caseRecord, title: 'Cambio concurrente' }, 2, {
    ...profile,
    general_information: 'Informacion vigente modificada por otra persona',
  });
  await login(page, false, false);
  await openCaseSummary(page, 'Cambio concurrente');
  await expect(editor(page)).toHaveCount(0);
  const gate = holdRead(state, casePath);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  await expect.poll(() => gate.entered).toBe(true);
  expect(
    await page.locator('input').evaluateAll((nodes) => nodes.map((node) => node.value)),
  ).not.toContain(rawEdit);
  gate.release();
  state.gate = null;
  const restored = editor(page);
  await expect(restored.getByLabel(titleLabel, { exact: true })).toHaveValue(rawEdit);
  await expect(restored.getByLabel(offenseLabel, { exact: true })).toHaveValue(pendingOffense);
  await expect(restored.getByRole('button', { name: /^Quitar descripci/ })).toHaveCount(1);
  const comparison = restored.getByRole('region', { name: 'Valores actuales guardados' });
  await expect(comparison).toContainText('Revisi\u00f3n 2');
  await expect(comparison).toContainText('Cambio concurrente');
  await expect(comparison).toContainText('Informacion vigente modificada por otra persona');
  const save = restored.getByRole('button', { name: 'Guardar mis cambios', exact: true });
  await expect(save).toBeDisabled();
  expect(state.confirmedWrites).toEqual([]);
  await restored.getByRole('checkbox', { name: /He comparado los valores actuales/ }).check();
  expect(state.confirmedWrites).toEqual([]);
  state.allowReplacement = true;
  await save.click();
  await expect(editor(page)).toHaveCount(0);
  expect(state.confirmedWrites).toHaveLength(1);
  expect(state.confirmedWrites[0].values).toEqual({
    expected_revision: 2,
    title: rawEdit.trim(),
    reference: caseRecord.reference,
    profile: { ...profile, offenses: [...profile.offenses, pendingOffense.trim()] },
  });
  expect(state.confirmedWrites[0].headers.authorization).toBe(`Bearer ${state.current.token}`);
});
