import { test, expect } from '@playwright/test';
import {
  login,
  navigate,
  caseId,
  otherCaseId,
  expectClientWorkspace,
  expectLanding,
  signOut,
  openSection,
} from './helpers.mjs';
import { memberRecord, memberPage, assignedMember, caseMemberPage } from '../fixtures/members.mjs';
import { requestCompletion } from './request-completion.mjs';
import {
  setupMembers,
  enterDirectory,
  directory,
  assignments,
  openAssignments,
} from './members-helpers.mjs';

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
      await expectLanding(page, role);
      if (role === 'client') await expectClientWorkspace(page, ['#case-members']);
      else {
        await navigate(page, 'Expedientes');
        await page.getByRole('button', { name: /Defensa inicial/ }).click();
        await expect(page.getByRole('link', { name: 'Asignaciones', exact: true })).toHaveCount(0);
        await page.evaluate(() => {
          location.hash = '#case-members';
        });
      }
      await expect(assignments(page)).toHaveCount(0);
      expect(state.calls).toEqual([]);
    } finally {
      await context.close();
    }
  }
});
