import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import {
  accounts,
  directory,
  assignments,
  card,
  responseTo,
  findAccount,
  changeAccess,
  openCase,
  searchAssignments,
  capture,
  accountAction,
} from './members-helpers.mjs';

for (const [name, width] of [
  ['desktop', 1440],
  ['mobile', 390],
]) {
  test(`real account lifecycle preserves assignments and requires fresh login at ${width}px`, async ({
    page,
    browser,
  }, testInfo) => {
    const scenario = accounts[name],
      pageErrors = [];
    page.on('pageerror', (error) => pageErrors.push(error.message));
    const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
    try {
      const subject = await context.newPage();
      await subject.goto('/');
      await loginAs(subject, scenario.subject, 0);
      await openCase(subject, scenario.case);
      await page.setViewportSize({ width, height: 1000 });
      await page.goto('/');
      await loginAs(page, scenario.owner, 0);
      await findAccount(page, scenario.subject);
      const inactive = await changeAccess(page, scenario.subject, 'litigator', false);
      await expect(card(page, scenario.subject.id)).toContainText('Inactiva');
      await openCase(page, scenario.case);
      await page.getByRole('link', { name: 'Asignaciones', exact: true }).click();
      await expect(assignments(page)).toHaveAttribute('aria-busy', 'false');
      await searchAssignments(page, scenario.subject.email);
      await expect(assignments(page)).toContainText('Inactiva');
      await capture(page, testInfo, `${name}-inactive-assignment`);
      await findAccount(page, scenario.subject, 'inactive');
      const active = await changeAccess(page, scenario.subject, 'litigator', true);
      expect(BigInt(active.revision)).toBe(BigInt(inactive.revision) + 1n);

      // This session was not used while inactive; reactivation must not revive it.
      const rejected = responseTo(subject, `/cases/${scenario.case.id}/administration`);
      await subject.getByRole('button', { name: 'Actualizar resumen', exact: true }).click();
      expect((await rejected).status()).toBe(401);
      await expect(subject.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
      await loginAs(subject, scenario.subject, 1);
      await openCase(subject, scenario.case);
      await expect(subject.getByRole('link', { name: 'Asignaciones', exact: true })).toHaveCount(0);

      await findAccount(page, scenario.subject);
      await expect(card(page, scenario.subject.id)).toContainText('Litigante');
      await expect(card(page, scenario.subject.id)).toContainText('Activa');
      await capture(page, testInfo, `${name}-reactivated-directory`);
      await openCase(page, scenario.case);
      if (scenario.closed) await expect(page.locator('.case-summary')).toContainText('Cerrado');
      await page.getByRole('link', { name: 'Asignaciones', exact: true }).click();
      await expect(assignments(page)).toHaveAttribute('aria-busy', 'false');
      await searchAssignments(page, scenario.available.email, 'available');
      await assignments(page)
        .getByRole('button', { name: `Asignar ${scenario.available.email}`, exact: true })
        .click();
      const assigned = responseTo(
        page,
        `/cases/${scenario.case.id}/members/${scenario.available.id}`,
        'PUT',
      );
      await assignments(page)
        .getByRole('button', { name: 'Confirmar asignaci\u00f3n', exact: true })
        .click();
      expect((await assigned).status()).toBe(204);
      await expect(assignments(page)).toHaveAttribute('aria-busy', 'false');
      await searchAssignments(page, scenario.available.email);
      await assignments(page)
        .getByRole('button', { name: `Retirar ${scenario.available.email}`, exact: true })
        .click();
      const removed = responseTo(
        page,
        `/cases/${scenario.case.id}/members/${scenario.available.id}`,
        'DELETE',
      );
      await assignments(page)
        .getByRole('button', { name: 'Confirmar retiro', exact: true })
        .click();
      expect((await removed).status()).toBe(204);
      await expect(assignments(page)).toHaveAttribute('aria-busy', 'false');
      await accountAction(scenario.owner, 1, async (call) => {
        const result = await call(
          'GET',
          `/cases/${scenario.case.id}/members?limit=20&selection=assigned&email_prefix=${encodeURIComponent(scenario.subject.email)}`,
        );
        expect(result.items.map((row) => [row.id, row.assigned_at, row.role, row.active])).toEqual([
          [scenario.subject.id, scenario.assignment, 'litigator', true],
        ]);
      });
      expect(pageErrors).toEqual([]);
    } finally {
      await context.close();
    }
  });
}

test('real self-demotion confirms before logout and Client remains outside account administration', async ({
  page,
  browser,
}, testInfo) => {
  await page.goto('/');
  await loginAs(page, accounts.selfOwner, 0);
  await findAccount(page, accounts.selfOwner);
  await changeAccess(page, accounts.selfOwner, 'paralegal', true);
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await expect(directory(page)).toHaveCount(0);
  await loginAs(page, accounts.selfOwner, 1);
  await expect(
    page.getByRole('navigation').getByRole('button', { name: 'Equipo', exact: true }),
  ).toHaveCount(0);
  await openCase(page, accounts.desktop.case);
  await expect(page.getByRole('link', { name: 'Asignaciones', exact: true })).toHaveCount(0);
  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const client = await context.newPage(),
      directoryRequests = [];
    client.on('request', (request) => {
      const path = new URL(request.url()).pathname;
      if (!path.startsWith('/api/v1/')) return;
      if (path === '/api/v1/users' || /\/members(?:\/|$)/.test(path)) directoryRequests.push(path);
    });
    await client.goto('/');
    await loginAs(client, accounts.client, 0);
    await openCase(client, accounts.desktop.case, false);
    await expect(client.getByRole('link', { name: 'Asignaciones', exact: true })).toHaveCount(0);
    await expect(
      client.getByRole('navigation').getByRole('button', { name: 'Equipo', exact: true }),
    ).toHaveCount(0);
    expect(directoryRequests).toEqual([]);
    await accountAction(accounts.client, 1, async (call) => {
      for (const path of [
        '/users',
        `/users/${accounts.selfOwner.id}`,
        `/cases/${accounts.desktop.case.id}/members`,
      ])
        expect((await call('GET', path, undefined, 403)).error.code).toBe('permission_denied');
    });
  } finally {
    await context.close();
  }
  await navigate(page, 'Inicio');
});
