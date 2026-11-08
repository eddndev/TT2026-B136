import { test, expect } from '@playwright/test';
import {
  setupHearings,
  openHearings,
  hearingEditor,
  fillHearing,
  confirmHearing,
} from './hearing-helpers.mjs';
import { typed } from './typed-participant-helpers.mjs';
import { document } from './helpers.mjs';
for (const [stage, kind] of [
  ['intermediate', 'intermediate'],
  ['trial', 'oral_trial'],
]) {
  test(`scheduling ${kind} uses the declared stage and keeps the type immutable`, async ({
    page,
  }) => {
    const { state } = await setupHearings(page, { stage });
    await openHearings(page);
    await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
    await fillHearing(page);
    await expect(
      hearingEditor(page).getByRole('combobox', { name: 'Tipo de audiencia', exact: true }),
    ).toHaveValue(kind);
    await confirmHearing(page);
    await expect(hearingEditor(page)).toHaveCount(0);
    expect(state.submissions[0].change.values.kind).toBe(kind);
    await page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
    await expect(
      hearingEditor(page).getByRole('combobox', { name: 'Tipo de audiencia', exact: true }),
    ).toBeDisabled();
  });
}
test('sentencing requires an explicit declaration and exact support without inferring a verdict', async ({
  page,
}) => {
  const { state } = await setupHearings(page, { stage: 'trial' });
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  const editor = hearingEditor(page);
  await editor
    .getByRole('combobox', { name: 'Tipo de audiencia', exact: true })
    .selectOption('sentencing');
  await editor.getByRole('button', { name: 'Revisar registro', exact: true }).click();
  await expect(editor.getByRole('alert')).toBeVisible();
  expect(state.calls.filter((row) => row.path.endsWith('/prepare'))).toHaveLength(0);
  await editor
    .getByLabel('Declaraci\u00f3n del antecedente', { exact: true })
    .fill('Condena declarada en el soporte seleccionado');
  await editor.getByRole('button', { name: 'Elegir soporte del antecedente', exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1 \/ contrato.pdf/ }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await confirmHearing(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.values.conviction_basis).toEqual({
    statement: 'Condena declarada en el soporte seleccionado',
    support: { document_id: document.id, version: 1, digest: document.digest },
  });
  await expect(
    page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
  ).toContainText('Antecedente de condena declarado');
});
test('same-name participants expose distinct exact references in the picker and bound history', async ({
  page,
}) => {
  const { state } = await setupHearings(page);
  const first = state.participants[0],
    second = { ...structuredClone(first), id: '99999999-9999-4999-8999-999999999999' };
  state.participants.push(second);
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  const editor = hearingEditor(page);
  await editor.getByRole('button', { name: 'Elegir participante', exact: true }).click();
  const picker = editor.getByRole('region', {
    name: 'Seleccionar participante exacto',
    exact: true,
  });
  const choices = picker.locator('.hearing-pick-reference');
  await expect(choices).toHaveCount(2);
  await choices.nth(1).getByText('Referencia exacta', { exact: true }).click();
  await expect(choices.nth(1)).toContainText(second.id);
  await choices
    .nth(1)
    .getByRole('button', { name: `Consultar ficha: ${first.display_name}`, exact: true })
    .click();
  await picker.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
  await editor.getByText('Referencia exacta del participante', { exact: true }).click();
  await expect(editor.locator('.hearing-participants')).toContainText(second.id);
  await confirmHearing(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.values.participants).toEqual([
    { participant_id: second.id, revision: 1 },
  ]);
  await page.setViewportSize({ width: 390, height: 1000 });
  const detail = page.getByRole('region', { name: 'Detalle de audiencia', exact: true });
  await detail.getByText('Referencia exacta del participante', { exact: true }).click();
  await expect(detail).toContainText(second.id);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  await page.screenshot({
    path: '../../../output/hearings-implementation/hearing-reference-390.png',
    fullPage: true,
  });
});
