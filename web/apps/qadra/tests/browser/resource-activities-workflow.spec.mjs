import { test, expect } from '@playwright/test';
import { technicalRecord } from '../fixtures/deadline-v2-unit.mjs';
import {
  setupResourceActivities,
  openResourceActivities,
  selectTarget,
  activityPanel,
  activityEditor,
  activityDetail,
} from './resource-activities-helpers.mjs';

for (const width of [1440, 390]) {
  test(`links an exact hearing and old act then unlinks without changing either at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupResourceActivities(page);
    await openResourceActivities(page, state);
    await activityPanel(page)
      .getByRole('button', { name: 'Vincular actividad', exact: true })
      .click();
    const editor = activityEditor(page);
    await expect(
      editor.getByRole('combobox', { name: 'Tipo de actividad', exact: true }),
    ).toHaveValue('');
    await selectTarget(page, state, 'hearing');
    await editor.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
    await editor.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
    await editor.getByRole('button', { name: 'Usar este acto', exact: true }).click();
    await editor.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
    await editor.getByRole('button', { name: 'Confirmar v\u00ednculo', exact: true }).click();
    await expect(editor).toHaveCount(0);
    expect(state.submissions).toHaveLength(1);
    const command = state.submissions[0];
    expect(command.change.target).toEqual({
      kind: 'hearing',
      id: state.hearing[0].id,
      revision: 1,
      submission_digest: state.hearing[0].receipt.submission_digest,
    });
    expect(command.change.act).toEqual({
      id: state.act.act.id,
      revision: 1,
      resource_revision: 2,
      capture_digest: state.act.receipt.capture_digest,
    });
    expect(command.expected_resource_revision).toBe(3);
    const detail = activityDetail(page);
    await expect(
      detail.getByRole('region', { name: 'Captura vinculada', exact: true }),
    ).toContainText('Sala historica uno');
    await expect(
      detail.getByRole('region', { name: 'Actividad actual', exact: true }),
    ).toContainText('Sala actual dos');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({
      path: testInfo.outputPath('resource-activity-linked.png'),
      fullPage: true,
    });
    await detail.getByRole('button', { name: 'Desvincular actividad', exact: true }).click();
    await editor.getByLabel('Motivo', { exact: true }).fill('Organizacion de la carpeta');
    await editor.getByRole('button', { name: 'Preparar desvinculaci\u00f3n', exact: true }).click();
    await editor
      .getByRole('button', { name: 'Confirmar desvinculaci\u00f3n', exact: true })
      .click();
    await expect(editor).toHaveCount(0);
    expect(state.submissions.map((row) => row.change.action)).toEqual(['link', 'unlink']);
    const [linked, unlinked] = state.records.get(command.association_id);
    expect(unlinked.selection).toEqual(linked.selection);
    expect(unlinked.sources).toEqual(linked.sources);
    expect(state.resources.facts.results.scheduling.submissions).toHaveLength(0);
    expect(state.targetCalls.every((call) => call.method === 'GET')).toBe(true);
    await detail
      .getByRole('button', { name: 'Ver historial de v\u00ednculo', exact: true })
      .click();
    await page
      .getByRole('button', { name: 'Consultar v\u00ednculo revisi\u00f3n 1', exact: true })
      .click();
    await expect(activityDetail(page)).toContainText('Vinculada');
    await expect(
      activityDetail(page).getByRole('region', { name: 'Captura vinculada', exact: true }),
    ).toContainText('Sala historica uno');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({
      path: testInfo.outputPath('resource-activity-history.png'),
      fullPage: true,
    });
  });
}
