import { test, expect } from '@playwright/test';
import { login, caseRecord } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import { participant, openParticipant, detail, directory } from './participant-helpers.mjs';
import {
  confirmationSetup,
  checkConfirmationRequests,
  holdConfirmationRequest,
  appliedCaseStatus,
  casePath,
  statusPath,
  participantPath,
} from './session-confirmation-fixtures.mjs';
import {
  beginConfirmationCase,
  enterConfirmationCase,
  expectOnlyOriginalCommand,
  expectFreshRead,
  releaseOldConfirmation,
} from './session-confirmation-ui.mjs';

test.afterEach(async ({ page }) => checkConfirmationRequests(page));

test('an applied case closure is read after MFA and its late receipt cannot replace a newer context or repeat the command', async ({
  page,
}) => {
  const state = await confirmationSetup(page);
  await login(page, false, false);
  await enterConfirmationCase(page);
  await page.getByRole('button', { name: 'Cerrar administrativamente', exact: true }).click();
  const oldDialog = page.getByRole('dialog', { name: 'Cerrar administrativamente', exact: true });
  const sent = holdConfirmationRequest(state, 'PUT', statusPath);
  state.confirmationWrite = {
    path: statusPath,
    method: 'PUT',
    body: { expected_revision: 1, administrative_status: 'closed' },
    response: appliedCaseStatus(),
    apply: () => {
      state.administration = appliedCaseStatus();
    },
  };
  await oldDialog
    .getByRole('button', { name: 'Confirmar cierre administrativo', exact: true })
    .click();
  await expect.poll(() => sent.entered).toBe(true);
  await expire(page, state, await oldDialog.elementHandle());
  state.administration = administration(
    { ...caseRecord, title: 'Defensa inicial actual' },
    3,
    null,
    'closed',
  );
  await login(page, false, false);
  const fresh = holdConfirmationRequest(state, 'GET', casePath);
  await beginConfirmationCase(page);
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toHaveCount(0);
  expectOnlyOriginalCommand(state, statusPath, 'PUT');
  expectFreshRead(fresh, state);
  fresh.release();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('button', { name: 'Reactivar expediente', exact: true }).click();
  const current = page.getByRole('dialog', { name: 'Reactivar expediente', exact: true });
  await expect(current).toContainText('Defensa inicial actual');
  await expect(current).toContainText('Revisi\u00f3n 3');
  await releaseOldConfirmation(page, sent, state);
  await expect(current).toBeVisible();
  await expect(current).toContainText('Revisi\u00f3n 3');
  await expect(
    current.getByRole('button', { name: 'Confirmar reactivaci\u00f3n', exact: true }),
  ).toBeEnabled();
  await expect(current.getByRole('alert')).toHaveCount(0);
  expectOnlyOriginalCommand(state, statusPath, 'PUT');
});

test('an unapplied participant status is discarded at expiry and its late error cannot poison a new explicit intention', async ({
  page,
}) => {
  const state = await confirmationSetup(page);
  await login(page, false, false);
  await enterConfirmationCase(page);
  await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  await openParticipant(page);
  await detail(page).getByRole('button', { name: 'Archivar participante', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Archivar participante', exact: true });
  const path = `${participantPath}/directory-status`;
  const sent = holdConfirmationRequest(state, 'PUT', path);
  state.confirmationWrite = {
    path,
    method: 'PUT',
    body: { expected_revision: 1, directory_status: 'archived' },
    status: 503,
    response: { error: { code: 'server_busy' } },
  };
  await dialog.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
  await expect.poll(() => sent.entered).toBe(true);
  await expire(page, state, await dialog.elementHandle());
  await login(page, false, false);
  const start = state.calls.length;
  await enterConfirmationCase(page);
  expect(
    state.calls.slice(start).some((call) => call.path === casePath && call.method === 'GET'),
  ).toBe(true);
  await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
  const fresh = holdConfirmationRequest(state, 'GET', participantPath);
  await directory(page)
    .getByRole('button', { name: `Abrir ${participant.display_name}`, exact: true })
    .click();
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(detail(page)).toHaveCount(0);
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expectFreshRead(fresh, state);
  expectOnlyOriginalCommand(state, path, 'PUT');
  fresh.release();
  await expect(detail(page)).toContainText('Revisi\u00f3n 1');
  await expect(dialog).not.toBeVisible();
  await detail(page).getByRole('button', { name: 'Archivar participante', exact: true }).click();
  await expect(
    dialog.getByRole('button', { name: 'Confirmar archivo', exact: true }),
  ).toBeEnabled();
  await releaseOldConfirmation(page, sent, state);
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByRole('button', { name: 'Confirmar archivo', exact: true }),
  ).toBeEnabled();
  await expect(dialog.getByRole('alert')).toHaveCount(0);
  expectOnlyOriginalCommand(state, path, 'PUT');
});
