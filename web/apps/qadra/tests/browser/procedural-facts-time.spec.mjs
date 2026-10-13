import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  fillResolution,
  confirmFact,
} from './procedural-facts-helpers.mjs';

for (const precision of ['date', 'minute', 'second'])
  test(`${precision} capture preserves its exact components and distinguishes absent offset from UTC`, async ({
    page,
  }) => {
    const state = await setupFacts(page);
    await openFacts(page);
    await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
    await fillResolution(page);
    const editor = factEditor(page);
    await editor
      .getByRole('combobox', { name: 'Precisi\u00f3n de emisi\u00f3n', exact: true })
      .selectOption(precision);
    await editor.getByLabel('Fecha de emisi\u00f3n', { exact: true }).fill('2026-09-01');
    if (precision !== 'date')
      await editor
        .getByLabel('Hora de emisi\u00f3n', { exact: true })
        .fill(precision === 'minute' ? '12:34' : '12:34:56');
    await confirmFact(page);
    await expect(editor).toHaveCount(0);
    const absent = state.submissions[0].change.values.issued_at;
    expect(absent).toEqual({
      precision,
      year: 2026,
      month: 9,
      day: 1,
      offset_seconds: null,
      ...(precision !== 'date' ? { hour: 12, minute: 34 } : {}),
      ...(precision === 'second' ? { second: 56 } : {}),
    });
    await factDetail(page)
      .getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true })
      .click();
    await expect(
      editor.getByRole('combobox', { name: 'Precisi\u00f3n de emisi\u00f3n', exact: true }),
    ).toHaveValue(precision);
    await editor
      .getByRole('combobox', { name: 'Desfase de emisi\u00f3n', exact: true })
      .selectOption('declared');
    await editor.getByLabel('Desfase UTC de emisi\u00f3n', { exact: true }).fill('+00:00');
    await editor.getByLabel('Motivo', { exact: true }).fill('Desfase comunicado expresamente');
    await confirmFact(page);
    await expect(editor).toHaveCount(0);
    expect(state.submissions[1].change.values.issued_at).toEqual({ ...absent, offset_seconds: 0 });
  });

test('lowering temporal precision removes hidden seconds instead of inventing a midnight instant', async ({
  page,
}) => {
  const state = await setupFacts(page);
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  const editor = factEditor(page);
  await editor
    .getByRole('combobox', { name: 'Precisi\u00f3n de emisi\u00f3n', exact: true })
    .selectOption('second');
  await editor.getByLabel('Fecha de emisi\u00f3n', { exact: true }).fill('2026-09-01');
  await editor.getByLabel('Hora de emisi\u00f3n', { exact: true }).fill('12:34:56');
  await editor
    .getByRole('combobox', { name: 'Precisi\u00f3n de emisi\u00f3n', exact: true })
    .selectOption('date');
  await expect(editor.getByLabel('Hora de emisi\u00f3n', { exact: true })).toHaveCount(0);
  await confirmFact(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.values.issued_at).toEqual({
    precision: 'date',
    year: 2026,
    month: 9,
    day: 1,
    offset_seconds: null,
  });
});

test('an explicitly entered zero second remains second precision through native time input', async ({
  page,
}) => {
  const state = await setupFacts(page);
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  const editor = factEditor(page);
  await editor
    .getByRole('combobox', { name: 'Precisi\u00f3n de emisi\u00f3n', exact: true })
    .selectOption('second');
  await editor.getByLabel('Fecha de emisi\u00f3n', { exact: true }).fill('2026-09-01');
  await editor.getByLabel('Hora de emisi\u00f3n', { exact: true }).fill('12:34:00');
  await confirmFact(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.values.issued_at).toEqual({
    precision: 'second',
    year: 2026,
    month: 9,
    day: 1,
    hour: 12,
    minute: 34,
    second: 0,
    offset_seconds: null,
  });
});
