import { test, expect } from '@playwright/test';
import { login, navigate, expectClientWorkspace } from './helpers.mjs';
import {
  setupResourceActivities,
  openResourceActivities,
  activityPanel,
  activityEditor,
  activityDetail,
} from './resource-activities-helpers.mjs';

async function unavailable(scope, name) {
  const button = scope.getByRole('button', { name, exact: true });
  if (await button.count()) await expect(button).toBeDisabled();
  else await expect(button).toHaveCount(0);
}
for (const [name, options] of [
  ['paralegal', { role: 'paralegal' }],
  ['closed case', { closed: true }],
]) {
  test(`${name} retains association history without an available mutation`, async ({ page }) => {
    const state = await setupResourceActivities(page, options);
    const linked = state.seed();
    await openResourceActivities(page, state);
    await unavailable(activityPanel(page), 'Vincular actividad');
    await activityPanel(page)
      .getByRole('button', { name: `Consultar v\u00ednculo ${linked.id}`, exact: true })
      .click();
    await unavailable(activityDetail(page), 'Desvincular actividad');
    await activityDetail(page)
      .getByRole('button', { name: 'Ver historial de v\u00ednculo', exact: true })
      .click();
    await page
      .getByRole('button', { name: 'Consultar v\u00ednculo revisi\u00f3n 1', exact: true })
      .click();
    await expect(activityDetail(page)).toContainText('Sala historica uno');
    expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
    expect(state.submissions).toHaveLength(0);
  });
}
