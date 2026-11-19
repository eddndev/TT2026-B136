import { test, expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { setupMeasureDecisions, openMeasures, decisionPanel } from './measure-decision-helpers.mjs';
import {
  setupPrecautionaryScheduling,
  openHearings,
  precautionaryPanel,
} from './precautionary-hearing-scheduling-helpers.mjs';

const missingContext = (page, code) =>
  page.route(`**/api/v1/cases/${caseId}/precautionary-context`, (route) =>
    route.fulfill({ status: 409, json: { error: { code, message: 'not ready' } } }),
  );

for (const code of ['case_stage_required', 'case_profile_required'])
  test(`measures stay readable and explain the missing stage on ${code}`, async ({ page }) => {
    const state = await setupMeasureDecisions(page, { seeded: true });
    await missingContext(page, code);
    await openMeasures(page);
    const panel = decisionPanel(page);
    await expect(panel).toContainText(
      'Completa la ficha penal y registra la etapa para registrar decisiones cautelares.',
    );
    await expect(panel.getByRole('alert')).toHaveCount(0);
    await expect(
      panel.getByRole('button', { name: /^Consultar decisi[oó]n cautelar / }),
    ).toHaveCount(1);
    await expect(
      panel.getByRole('button', { name: 'Registrar decisión cautelar', exact: true }),
    ).toBeDisabled();
    expect(state.unexpected).toEqual([]);
  });

test('precautionary hearings stay readable and explain the missing stage', async ({ page }) => {
  const state = await setupPrecautionaryScheduling(page, { seeded: true });
  await missingContext(page, 'case_stage_required');
  await openHearings(page);
  const panel = precautionaryPanel(page);
  await expect(panel).toContainText(
    'Completa la ficha penal y registra la etapa para programar una audiencia cautelar.',
  );
  await expect(panel.getByRole('alert')).toHaveCount(0);
  await expect(
    panel.getByRole('button', { name: 'Programar audiencia cautelar', exact: true }),
  ).toBeDisabled();
  expect(state.calls.some((call) => call.method === 'GET' && call.path === state.base)).toBe(true);
});

test('any other precautionary context failure is still reported', async ({ page }) => {
  await setupMeasureDecisions(page, { seeded: true });
  await page.route(`**/api/v1/cases/${caseId}/precautionary-context`, (route) =>
    route.fulfill({ status: 500, json: { error: { code: 'internal' } } }),
  );
  await openMeasures(page);
  await expect(decisionPanel(page).getByRole('alert')).toBeVisible();
  await expect(decisionPanel(page)).not.toContainText('registra la etapa');
});
