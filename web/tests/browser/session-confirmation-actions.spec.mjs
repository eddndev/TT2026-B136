import { test, expect } from '@playwright/test';
import { login, openDocument } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { assignments } from './members-helpers.mjs';
import {
  confirmationSetup,
  checkConfirmationRequests,
  holdConfirmationRequest,
  membersPath,
  assignmentPath,
  versionPath,
  casePath,
} from './session-confirmation-fixtures.mjs';
import {
  enterConfirmationCase,
  expectOnlyOriginalCommand,
  expectFreshRead,
  releaseOldConfirmation,
} from './session-confirmation-ui.mjs';

test.afterEach(async ({ page }) => checkConfirmationRequests(page));

test('an applied membership removal is read fresh after MFA and its late result cannot clear a newly chosen assignment', async ({
  page,
}) => {
  const state = await confirmationSetup(page);
  await login(page, false, false);
  await enterConfirmationCase(page);
  await page.getByRole('link', { name: 'Asignaciones', exact: true }).click();
  const panel = assignments(page);
  await expect(panel).toHaveAttribute('aria-busy', 'false');
  await panel.getByLabel('Buscar por correo', { exact: true }).fill(' PERSON2 ');
  await panel.getByRole('button', { name: 'Buscar asignaciones', exact: true }).click();
  await panel.getByRole('button', { name: 'Retirar person2@example.test', exact: true }).click();
  const sent = holdConfirmationRequest(state, 'DELETE', assignmentPath);
  state.confirmationWrite = {
    path: assignmentPath,
    method: 'DELETE',
    body: null,
    status: 204,
    response: null,
    apply: () => {
      state.assignment = false;
    },
  };
  await panel.getByRole('button', { name: 'Confirmar retiro', exact: true }).click();
  await expect.poll(() => sent.entered).toBe(true);
  await expire(page, state, await panel.elementHandle());
  const afterExpiry = state.calls.length;
  await login(page, false, false);
  await enterConfirmationCase(page);
  expect(
    state.calls.slice(afterExpiry).some((call) => call.path === casePath && call.method === 'GET'),
  ).toBe(true);
  const fresh = holdConfirmationRequest(state, 'GET', membersPath);
  await page.getByRole('link', { name: 'Asignaciones', exact: true }).click();
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(panel.locator('.member-confirmation')).toHaveCount(0);
  await expect(panel.getByRole('button', { name: /^(Retirar|Asignar) person2/ })).toHaveCount(0);
  expectFreshRead(fresh, state);
  expectOnlyOriginalCommand(state, assignmentPath, 'DELETE');
  fresh.release();
  await expect(panel).toHaveAttribute('aria-busy', 'false');
  await expect(panel).toContainText('No hay cuentas');
  await expect(panel.getByLabel('Buscar por correo', { exact: true })).toHaveValue('');
  await panel
    .getByRole('combobox', { name: 'Selecci\u00f3n de cuentas', exact: true })
    .selectOption('available');
  await panel.getByRole('button', { name: 'Buscar asignaciones', exact: true }).click();
  await panel.getByRole('button', { name: 'Asignar person2@example.test', exact: true }).click();
  const confirm = panel.getByRole('button', { name: 'Confirmar asignaci\u00f3n', exact: true });
  await expect(confirm).toBeEnabled();
  await releaseOldConfirmation(page, sent, state);
  await expect(confirm).toBeEnabled();
  await expect(panel.getByRole('status').filter({ hasText: /confirmad/ })).toHaveCount(0);
  await expect(panel.getByRole('alert')).toHaveCount(0);
  expectOnlyOriginalCommand(state, assignmentPath, 'DELETE');
});

test('an unapplied seal is read as the exact version after MFA without replay and its late failure cannot change a new confirmation', async ({
  page,
}) => {
  const state = await confirmationSetup(page);
  await login(page);
  await openDocument(page);
  await expect(
    page.getByRole('button', { name: 'Actualizar historial', exact: true }),
  ).toBeEnabled();
  const startSeal = page.getByRole('button', { name: 'Sellar documento', exact: true });
  const confirm = page.getByRole('button', { name: 'Confirmar sellado', exact: true });
  await startSeal.click();
  const sent = holdConfirmationRequest(state, 'POST', `${versionPath}/seal`);
  state.confirmationWrite = {
    path: `${versionPath}/seal`,
    method: 'POST',
    body: null,
    status: 503,
    response: { error: { code: 'server_busy' } },
  };
  await confirm.click();
  await expect.poll(() => sent.entered).toBe(true);
  await expire(page, state, await page.locator('.detail-panel').elementHandle());
  const afterExpiry = state.calls.length;
  await login(page);
  expect(
    state.calls.slice(afterExpiry).some((call) => call.path === casePath && call.method === 'GET'),
  ).toBe(true);
  await openDocument(page);
  await expect(confirm).toHaveCount(0);
  await expect(
    page.getByRole('button', { name: 'Actualizar historial', exact: true }),
  ).toBeEnabled();
  const fresh = holdConfirmationRequest(state, 'GET', versionPath);
  await page
    .getByRole('region', { name: 'Historial de versiones', exact: true })
    .getByRole('button', { name: /Versi\u00f3n 1 \/ contrato\.pdf/ })
    .click();
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(startSeal).toHaveCount(0);
  await expect(confirm).toHaveCount(0);
  expectFreshRead(fresh, state);
  expectOnlyOriginalCommand(state, `${versionPath}/seal`, 'POST');
  fresh.release();
  await expect(startSeal).toBeEnabled();
  await expect(confirm).toHaveCount(0);
  await startSeal.click();
  await expect(confirm).toBeEnabled();
  await releaseOldConfirmation(page, sent, state);
  await expect(confirm).toBeEnabled();
  await expect(page.locator('.detail-panel').getByRole('alert')).toHaveCount(0);
  expectOnlyOriginalCommand(state, `${versionPath}/seal`, 'POST');
});
