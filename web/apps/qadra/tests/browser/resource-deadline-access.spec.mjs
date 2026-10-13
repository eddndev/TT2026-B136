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
test('revoked access clears contextual deadline fields after preparation is denied', async ({
  page,
}) => {
  const state = await setupResourceDeadline(page, { role: 'litigator' });
  state.handle = async (route) => {
    await route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } });
    return true;
  };
  await openResourceDeadline(page, state);
  await editor(page).getByRole('button', { name: 'Preparar plazo y vinculo', exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  await expect(activityPanel(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(0);
});
