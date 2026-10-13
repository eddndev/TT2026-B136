import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { preparation, receipt, withdrawn } from '../fixtures/owner-certificates.mjs';
import {
  ownerCertificateSetup,
  checkOwnerCertificateRequests,
  bindingsPath,
} from './owner-certificate-fixtures.mjs';
import { openOwnerCertificates, current } from './owner-certificate-ui.mjs';

test.afterEach(async ({ page }) => checkOwnerCertificateRequests(page));

test('a terminal reply with different original signature cannot confirm withdrawal or discard its exact intent', async ({
  page,
}) => {
  const state = await ownerCertificateSetup(page);
  await login(page, false, false);
  const original = receipt(preparation(state.current.user.id));
  state.ownerRecords.set(original.binding_id, original);
  await openOwnerCertificates(page);
  await current(page).getByRole('button', { name: 'Retirar v\u00ednculo', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Retirar mi certificado', exact: true });
  const conflicting = withdrawn(original);
  conflicting.registration.signature_base64 = Buffer.alloc(384, 9).toString('base64');
  state.nextOwnerWrite = { action: 'withdraw', commit: false, response: conflicting };
  await dialog.getByRole('button', { name: 'Confirmar retiro', exact: true }).click();
  await expect(dialog.getByRole('alert')).toBeVisible();
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByRole('button', { name: 'Confirmar retiro', exact: true }),
  ).toBeDisabled();
  expect(state.ownerWrites).toHaveLength(1);
  expect(state.ownerRecords.get(original.binding_id)).toEqual(original);
  const start = state.calls.length;
  await dialog.getByRole('button', { name: 'Comprobar retiro', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Confirmar retiro', exact: true })).toBeEnabled();
  expect(
    state.calls
      .slice(start)
      .filter(
        (call) => call.method === 'GET' && call.path === `${bindingsPath}/${original.binding_id}`,
      ),
  ).toHaveLength(1);
  expect(state.ownerWrites).toHaveLength(1);
  state.nextOwnerWrite = { action: 'withdraw' };
  await dialog.getByRole('button', { name: 'Confirmar retiro', exact: true }).click();
  await expect(dialog).toBeHidden();
  expect(state.ownerWrites.map(({ id, values }) => ({ id, values }))).toEqual([
    { id: original.binding_id, values: { expected_revision: 1 } },
    { id: original.binding_id, values: { expected_revision: 1 } },
  ]);
  expect(state.ownerRecords.get(original.binding_id).registration).toEqual(original.registration);
});
