import { test, expect } from '@playwright/test';
import { setupHearings, openHearings, hearingEditor, fillHearing } from './hearing-helpers.mjs';
import { hearingRecord, hearingPrepared } from '../fixtures/hearings.mjs';
import { login, navigate, caseId } from './helpers.mjs';
import { setupHearingAgenda, queryHearingAgenda } from './combined-agenda-helpers.mjs';
for (const operation of ['prepare', 'detail']) {
  test(`late ${operation} cannot restore a hearing view after leaving the case`, async ({
    page,
  }) => {
    const record = hearingRecord(),
      { state } = await setupHearings(page, { records: [record] });
    let release;
    state.handle = async (route, call) => {
      const matches =
        operation === 'prepare'
          ? call.path.endsWith('/prepare')
          : call.path.endsWith(`/hearings/${record.id}`);
      if (!matches) return false;
      await new Promise((resolve) => {
        release = resolve;
      });
      await route.fulfill({ json: operation === 'prepare' ? hearingPrepared(call.body) : record });
      return true;
    };
    await openHearings(page);
    if (operation === 'prepare') {
      await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
      await fillHearing(page);
      await hearingEditor(page)
        .getByRole('button', { name: 'Revisar registro', exact: true })
        .click();
    } else
      await page
        .getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true })
        .click();
    await expect.poll(() => !!release).toBeTruthy();
    await navigate(page, 'Inicio');
    const late = page.waitForResponse((response) =>
      response.url().endsWith(operation === 'prepare' ? '/prepare' : `/hearings/${record.id}`),
    );
    release();
    await late;
    await expect(
      page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
    ).toBeVisible();
    await expect(hearingEditor(page)).toHaveCount(0);
    await expect(
      page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
    ).toHaveCount(0);
    expect(state.submissions).toHaveLength(0);
  });
}
test('a late agenda case validation cannot navigate after leaving Agenda', async ({ page }) => {
  const record = hearingRecord(),
    { state } = await setupHearingAgenda(page, [record]);
  await login(page, false, false);
  await navigate(page, 'Agenda');
  await queryHearingAgenda(page);
  let release;
  await page.route(`**/api/v1/cases/${caseId}/administration`, async (route) => {
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({ json: state.admin });
  });
  await page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }).click();
  await expect.poll(() => !!release).toBeTruthy();
  await navigate(page, 'Inicio');
  const late = page.waitForResponse((response) => response.url().endsWith('/administration'));
  release();
  await late;
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  expect(state.calls.some((row) => row.path.includes('/revisions/'))).toBeFalsy();
});
