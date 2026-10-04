import { test, expect } from '@playwright/test';
import { expire } from './session-inactivity-helpers.mjs';
import {
  setupHearingSession,
  login,
  openHearingScope,
  resumeHearing,
} from './resource-hearing-scheduling-session-helpers.mjs';
import {
  openHearingForm,
  fillHearingForm,
  hearingEditor,
} from './resource-hearing-scheduling-helpers.mjs';
import { caseId } from './helpers.mjs';

test('every recovery attempt revalidates retained exact sources before unblocking the draft', async ({
  page,
}) => {
  const state = await setupHearingSession(page);
  await openHearingForm(page, state);
  await fillHearingForm(page);
  await expire(page, state, await hearingEditor(page).elementHandle());
  await login(page, false, false);
  await openHearingScope(page, state);
  const path = `/api/v1/cases/${caseId}/procedural-resources/${state.activities.resource.id}`;
  let exactReads = 0;
  await page.route(`**${path}/revisions/1`, async (route) => {
    exactReads++;
    if (exactReads <= 2)
      await route.fulfill({ status: 503, json: { error: { code: 'service_busy' } } });
    else await route.fallback();
  });
  await resumeHearing(page);
  const editor = hearingEditor(page);
  await expect.poll(() => exactReads).toBe(1);
  await expect(editor).toHaveAttribute('aria-busy', 'false');
  await expect(
    editor.getByRole('button', { name: 'Preparar audiencia y vinculo', exact: true }),
  ).toBeDisabled();
  for (const expected of [2, 3]) {
    const head = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === path && response.request().method() === 'GET',
    );
    await editor
      .getByRole('button', { name: 'Volver a consultar el contexto', exact: true })
      .click();
    await head;
    await expect(editor).toHaveAttribute('aria-busy', 'false');
    expect(exactReads).toBe(expected);
    if (expected === 2)
      await expect(
        editor.getByRole('button', { name: 'Preparar audiencia y vinculo', exact: true }),
      ).toBeDisabled();
  }
  await expect(editor.getByLabel('Sede o enlace', { exact: true })).toHaveValue(
    'Sala propia del recurso',
  );
  await expect(
    editor.getByRole('button', { name: 'Preparar audiencia y vinculo', exact: true }),
  ).toBeEnabled();
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(0);
});
