import { test, expect } from '@playwright/test';
import { login, navigate, caseId, expectClientWorkspace } from './helpers.mjs';
import { openResourceActivities } from './resource-activities-helpers.mjs';
import { failFact } from './procedural-facts-helpers.mjs';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import {
  setupResourceHearing,
  openHearingForm,
  fillHearingForm,
  prepareHearing,
  submitHearing,
  activityPanel,
  hearingEditor,
  hearingPanel,
  hearingDetail,
} from './resource-hearing-scheduling-helpers.mjs';

for (const [label, options] of [
  ['paralegal', { role: 'paralegal' }],
  ['closed case', { closed: true }],
  ['archived resource', { archived: true }],
]) {
  test(`${label} retains own hearing history without a creation action`, async ({ page }) => {
    const state = await setupResourceHearing(page, options);
    const saved = state.seed();
    if (options.archived) {
      const command = resourceCommandFixture('archive');
      command.resource_id = state.activities.resource.id;
      state.resources.commit(state.resources.prepare(command));
    }
    await openResourceActivities(page, state.activities);
    const button = activityPanel(page).getByRole('button', {
      name: 'Crear audiencia de recurso',
      exact: true,
    });
    if (await button.count()) await expect(button).toBeDisabled();
    else await expect(button).toHaveCount(0);
    await hearingPanel(page)
      .getByRole('button', {
        name: `Consultar audiencia de recurso ${saved.hearing.id}`,
        exact: true,
      })
      .click();
    await expect(hearingDetail(page)).toContainText(saved.hearing.values.venue);
    await expect(hearingEditor(page)).toHaveCount(0);
    expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
    expect(state.submissions).toHaveLength(0);
  });
}
