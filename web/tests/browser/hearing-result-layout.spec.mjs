import { test, expect } from '@playwright/test';
import {
  setupResults,
  openResults,
  resultPanel,
  resultDetail,
  resultEditor,
  fillResult,
} from './hearing-result-helpers.mjs';
import { resultRecord } from '../fixtures/hearing-results.mjs';
import { participant } from './participant-helpers.mjs';
import { typed } from './typed-participant-helpers.mjs';

async function capturePanel(page, locator, name) {
  const box = await locator.boundingBox();
  const x = Math.floor(box.x),
    y = Math.floor(box.y);
  await page.screenshot({
    path: `../output/hearing-results-implementation/${name}.png`,
    fullPage: true,
    clip: {
      x,
      y,
      width: Math.ceil(box.x + box.width) - x,
      height: Math.ceil(box.y + box.height) - y,
    },
  });
}

for (const width of [1440, 390])
  test(`Qadra result detail and form stay readable at ${width}px with exact identity references`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const row = resultRecord();
    row.values.attendees = [
      {
        participant_id: typed.id,
        revision: typed.revision,
        capacity: 'Compareciente declarado',
        observation: 'Entrada informada\nSin deducir notificacion',
      },
    ];
    row.attendees = [
      {
        ...structuredClone(typed),
        profile: 'typed',
        kind: typed.profile.kind,
        subject_digest: typed.subject.values_digest,
        capacity: row.values.attendees[0].capacity,
        observation: row.values.attendees[0].observation,
      },
    ];
    row.values.agreements = [
      {
        id: '90000000-0000-4000-8000-000000000009',
        text: 'Texto declarado de acuerdo\nCon alcance explicado por el operador.',
      },
    ];
    await setupResults(page, { results: [row] });
    await openResults(page);
    await resultPanel(page)
      .getByRole('button', { name: `Consultar resultado ${row.id}`, exact: true })
      .click();
    await resultDetail(page)
      .getByText('Referencia exacta del participante', { exact: true })
      .click();
    await expect(resultDetail(page)).toContainText(typed.subject.values_digest);

    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../output/hearing-results-implementation/result-detail-${width}.png`,
      fullPage: true,
    });
    await capturePanel(page, resultDetail(page), `result-detail-panel-final-${width}`);
    await resultPanel(page)
      .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
      .click();
    await fillResult(page);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../output/hearing-results-implementation/result-form-${width}.png`,
      fullPage: true,
    });
    await capturePanel(page, resultEditor(page), `result-form-panel-final-${width}`);
    await expect(
      resultEditor(page).getByRole('button', { name: 'Revisar resultado', exact: true }),
    ).toBeVisible();
  });

test('same-name historical fiches stay distinguishable by exact identifiers', async ({ page }) => {
  const second = { ...structuredClone(participant), id: '99999999-9999-4999-8999-999999999999' };
  const state = await setupResults(page);
  state.directory.set(second.id, [second]);
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  const editor = resultEditor(page);
  await editor.getByText('Comparecencias informadas (0/32)', { exact: true }).click();
  await editor.getByRole('button', { name: 'Agregar comparecencia', exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Elegir ficha hist\u00f3rica', exact: true });
  await picker
    .getByRole('button', { name: `Consultar historia de ficha ${second.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Consultar ficha revisi\u00f3n 1', exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Informar comparecencia de esta revisi\u00f3n', exact: true })
    .click();
  const group = editor.getByRole('group', { name: `Comparecencia ${second.id}`, exact: true });
  await group.getByText('Referencia exacta del participante', { exact: true }).click();
  await expect(group).toContainText(second.id);
});
