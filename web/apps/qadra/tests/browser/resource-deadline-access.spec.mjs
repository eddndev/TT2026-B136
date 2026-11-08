import { test, expect } from '@playwright/test';
import { openResourceActivities, activityPanel } from './resource-activities-helpers.mjs';
import {
  setupResourceDeadline,
  openResourceDeadline,
  editor,
} from './resource-deadline-helpers.mjs';
for (const [name, options] of [
  ['paralegal', { role: 'paralegal' }],
  ['closed case', { closed: true }],
]) {
  test(`${name} cannot open contextual deadline creation`, async ({ page }) => {
    const state = await setupResourceDeadline(page, options);
    await openResourceActivities(page, state.activities);
    const button = activityPanel(page).getByRole('button', { name: 'Crear plazo', exact: true });
    if (await button.count()) await expect(button).toBeDisabled();
    else await expect(button).toHaveCount(0);
    await expect(editor(page)).toHaveCount(0);
    expect(state.calls).toHaveLength(0);
  });
}
