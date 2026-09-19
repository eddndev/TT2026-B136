import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
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
test('revoked membership clears linked private captures after a protected refresh', async ({
  page,
}) => {
  const state = await setupResourceActivities(page, { role: 'litigator' });
  const linked = state.seed();
  await openResourceActivities(page, state);
  await activityPanel(page)
    .getByRole('button', { name: `Consultar v\u00ednculo ${linked.id}`, exact: true })
    .click();
  await expect(activityDetail(page)).toContainText('Sala historica uno');
  state.resources.facts.results.scheduling.denied = true;
  const denied = page.waitForResponse(
    (response) => response.url().includes('/activities') && response.status() === 403,
  );
  await activityPanel(page)
    .getByRole('button', { name: 'Actualizar actividades', exact: true })
    .click();
  await denied;
  await expect(activityPanel(page)).toHaveCount(0);
  await expect(activityDetail(page)).toHaveCount(0);
  await expect(activityEditor(page)).toHaveCount(0);
  await expect(page.getByText('Sala historica uno', { exact: true })).toHaveCount(0);
  expect(state.submissions).toHaveLength(0);
});
test('client never loads resource associations through navigation or a protected hash', async ({
  page,
}) => {
  const state = await setupResourceActivities(page, { role: 'client' });
  state.seed();
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Recursos', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'resources';
  });
  await expect(activityPanel(page)).toHaveCount(0);
  await expect(activityEditor(page)).toHaveCount(0);
  expect(state.calls).toHaveLength(0);
});
