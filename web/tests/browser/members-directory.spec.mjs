import { test, expect } from '@playwright/test';
import { memberActor, memberId, memberRecord } from '../fixtures/members.mjs';
import { caseId, navigate } from './helpers.mjs';
import {
  setupMembers,
  enterDirectory,
  directory,
  editor,
  memberCard,
  openAccess,
  reviewAccess,
  openAssignments,
  assignments,
} from './members-helpers.mjs';

for (const width of [1440, 390]) {
  test(`Owner manages accounts and email assignments at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupMembers(page, { closed: width === 390 });
    await enterDirectory(page);
    const row = state.rows.find((item) => item.id === memberId(2)),
      originalRevision = row.revision;
    await openAccess(page, row);
    await reviewAccess(page, 'litigator', false);
    await expect(editor(page)).toContainText('asignaciones');
    await editor(page)
      .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
      .click();
    await expect(editor(page)).toHaveCount(0);
    await expect(memberCard(page, row.id)).toHaveCount(0);
    expect(
      state.calls
        .filter((call) => call.method === 'PUT' && call.path.endsWith('/access'))
        .map((call) => call.body),
    ).toEqual([{ expected_revision: originalRevision, role: 'litigator', active: false }]);
    await directory(page)
      .getByRole('combobox', { name: 'Estado de la cuenta', exact: true })
      .selectOption('inactive');
    await directory(page).getByRole('button', { name: 'Buscar cuentas', exact: true }).click();
    await expect(memberCard(page, row.id)).toContainText('Inactiva');
    await openAccess(page, row);
    await reviewAccess(page, 'litigator', true);
    await expect(editor(page)).toContainText('nuevo inicio de sesi\u00f3n');
    await editor(page)
      .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
      .click();
    await expect(editor(page)).toHaveCount(0);
    await directory(page)
      .getByRole('combobox', { name: 'Estado de la cuenta', exact: true })
      .selectOption('all');
    await directory(page).getByRole('button', { name: 'Buscar cuentas', exact: true }).click();
    await expect(memberCard(page, row.id)).toContainText('Litigante');
    await expect(memberCard(page, row.id)).toContainText('Activa');
    expect(state.memberships.get(caseId).has(row.id)).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: testInfo.outputPath('member-directory.png'), fullPage: true });

    await openAssignments(page);
    await expect(assignments(page)).toContainText('person4@example.test');
    await expect(assignments(page)).toContainText('Inactiva');
    await expect(assignments(page)).toContainText('acceso global');
    await assignments(page)
      .getByRole('combobox', { name: 'Selecci\u00f3n de cuentas', exact: true })
      .selectOption('available');
    await assignments(page).getByLabel('Buscar por correo', { exact: true }).fill(' PERSON6 ');
    await assignments(page)
      .getByRole('button', { name: 'Buscar asignaciones', exact: true })
      .click();
    await assignments(page)
      .getByRole('button', { name: 'Asignar person6@example.test', exact: true })
      .click();
    await assignments(page)
      .getByRole('button', { name: 'Confirmar asignaci\u00f3n', exact: true })
      .click();
    await expect(
      assignments(page).getByRole('button', { name: 'Asignar person6@example.test', exact: true }),
    ).toHaveCount(0);
    await assignments(page)
      .getByRole('combobox', { name: 'Selecci\u00f3n de cuentas', exact: true })
      .selectOption('assigned');
    await assignments(page)
      .getByRole('button', { name: 'Buscar asignaciones', exact: true })
      .click();
    await assignments(page)
      .getByRole('button', { name: 'Retirar person6@example.test', exact: true })
      .click();
    await assignments(page).getByRole('button', { name: 'Confirmar retiro', exact: true }).click();
    await expect(
      assignments(page).getByRole('button', { name: 'Retirar person6@example.test', exact: true }),
    ).toHaveCount(0);
    expect(
      state.calls
        .filter((call) => /\/members\//.test(call.path))
        .map((call) => [call.method, call.path]),
    ).toEqual(
      ['PUT', 'DELETE'].map((method) => [method, `/api/v1/cases/${caseId}/members/${memberId(6)}`]),
    );
    expect(state.memberships.get(caseId).has(memberId(4))).toBe(true);
    await assignments(page).getByLabel('Buscar por correo', { exact: true }).fill('');
    await assignments(page)
      .getByRole('button', { name: 'Buscar asignaciones', exact: true })
      .click();
    await expect(assignments(page)).toContainText('person4@example.test');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: testInfo.outputPath('member-assignments.png'), fullPage: true });
  });
}

test('directory accumulates pages and changing filters resets its opaque cursor', async ({
  page,
}) => {
  const rows = [memberActor, ...Array.from({ length: 51 }, (_, index) => memberRecord(index + 2))];
  const state = await setupMembers(page, { rows });
  await enterDirectory(page);
  const more = directory(page).getByRole('button', {
    name: 'Cargar m\u00e1s cuentas',
    exact: true,
  });
  for (let attempt = 0; attempt < 5 && (await more.count()); attempt++) {
    await more.click();
    await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
  }
  await expect(directory(page).locator('[data-user-id]')).toHaveCount(52);
  expect(state.calls.some((call) => call.query.cursor)).toBe(true);
  await directory(page).getByLabel('Buscar por correo', { exact: true }).fill(' PERSON52 ');
  await directory(page).getByRole('button', { name: 'Buscar cuentas', exact: true }).click();
  await expect(directory(page).locator('[data-user-id]')).toHaveCount(1);
  await expect(memberCard(page, memberId(52))).toBeVisible();
  expect(state.calls.at(-1).query.email_prefix).toBe('person52');
  expect(state.calls.at(-1).query.cursor).toBeUndefined();
});

test('existing account creation retains one-time enrollment and remains distinct from invitations', async ({
  page,
}) => {
  const state = await setupMembers(page);
  await enterDirectory(page);
  await page.getByLabel('Correo del nuevo usuario', { exact: true }).fill('new@example.test');
  await page.getByLabel('Contrase\u00f1a inicial', { exact: true }).fill('synthetic-new-password');
  await page.getByRole('button', { name: 'Crear usuario', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Configura el segundo factor', exact: true }),
  ).toBeVisible();
  await page
    .getByRole('checkbox', { name: 'Ya guard\u00e9 la clave y los c\u00f3digos', exact: true })
    .check();
  await page.getByRole('button', { name: 'Finalizar', exact: true }).click();
  await expect(page.getByLabel('Contrase\u00f1a inicial', { exact: true })).toHaveValue('');
  await expect(page.getByText('TESTSECRET', { exact: true })).toHaveCount(0);
  expect(
    state.calls.filter((call) => call.method === 'POST' && call.path === '/api/v1/users'),
  ).toHaveLength(1);
  expect(state.calls.some((call) => /invitation|invite/.test(call.path))).toBe(false);
  await navigate(page, 'Inicio');
  expect(
    await page.evaluate(
      () => Object.keys(localStorage).length + Object.keys(sessionStorage).length,
    ),
  ).toBe(0);
});
