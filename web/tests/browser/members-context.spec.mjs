import { test, expect } from '@playwright/test';
import { login, navigate, caseId, otherCaseId } from './helpers.mjs';
import { memberRecord, memberPage, assignedMember, caseMemberPage } from '../fixtures/members.mjs';
import { requestCompletion } from './request-completion.mjs';
import {
  setupMembers,
  enterDirectory,
  directory,
  assignments,
  openAssignments,
} from './members-helpers.mjs';

test('late member rows cannot enter another case after changing the selected expediente', async ({
  page,
}) => {
  const state = await setupMembers(page);
  await login(page, false, false);
  let release;
  state.handle = (route, call) => {
    if (call.path !== `/api/v1/cases/${caseId}/members`) return false;
    return new Promise((resolve) => {
      release = async () => {
        await route.fulfill({ json: caseMemberPage([assignedMember(2)]) });
        resolve(true);
      };
    });
  };
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Asignaciones', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await openAssignments(page, 'Otro expediente');
  await expect(assignments(page)).toContainText('person6@example.test');
  const completed = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === `/api/v1/cases/${caseId}/members`,
  );
  await release();
  const result = await completed;
  if (result.failed) expect(result.request.failure()?.errorText).toMatch(/abort|cancel/i);
  await expect(assignments(page)).not.toContainText('person2@example.test');
  await expect(
    assignments(page).getByRole('button', { name: 'Retirar person6@example.test', exact: true }),
  ).toBeVisible();
  expect(
    state.calls
      .filter((call) => call.path.startsWith(`/api/v1/cases/${otherCaseId}/members`))
      .every((call) => call.method === 'GET'),
  ).toBe(true);
});

test('late directory response cannot repopulate data after session changes', async ({ page }) => {
  const state = await setupMembers(page);
  await enterDirectory(page);
  let release;
  state.handle = (route, call) => {
    if (call.path !== '/api/v1/users' || call.method !== 'GET') return false;
    return new Promise((resolve) => {
      release = async () => {
        await route.fulfill({ json: memberPage([memberRecord(2)]) });
        resolve(true);
      };
    });
  };
  const received = page.waitForResponse(
    (response) => new URL(response.url()).pathname === '/api/v1/users',
  );
  await directory(page).getByRole('button', { name: 'Buscar cuentas', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await release();
  expect((await received).status()).toBe(200);
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  await expect(directory(page)).toHaveCount(0);
  await expect(page.locator('[data-user-id]')).toHaveCount(0);
});

test('non-Owner roles cannot open account administration or case assignments', async ({
  browser,
}, testInfo) => {
  for (const role of ['litigator', 'paralegal', 'client']) {
    const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
    try {
      const page = await context.newPage(),
        state = await setupMembers(page, { role });
      await login(page, false, false);
      await expect(
        page.getByRole('navigation').getByRole('button', { name: 'Equipo', exact: true }),
      ).toHaveCount(0);
      await page.evaluate(() => {
        location.hash = '#team';
      });
      await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
      await navigate(page, 'Expedientes');
      await page.getByRole('button', { name: /Defensa inicial/ }).click();
      await expect(page.getByRole('link', { name: 'Asignaciones', exact: true })).toHaveCount(0);
      await page.evaluate(() => {
        location.hash = '#case-members';
      });
      await expect(assignments(page)).toHaveCount(0);
      expect(state.calls).toEqual([]);
    } finally {
      await context.close();
    }
  }
});
