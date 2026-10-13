import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { downloadedBytes, captureOwnerLayouts } from './owner-certificate-ui.mjs';
import {
  ownerLoginSetup,
  checkOwnerLoginRequests,
  holdLogin,
  loginPath,
  publicReceipt,
  selection,
  signature,
  owner,
} from './owner-login-fixtures.mjs';
import {
  panel,
  prepare,
  proof,
  again,
  download,
  signatureInput,
  back,
  openOwnerLogin,
  selectReceipt,
  prepareLogin,
  selectSignature,
  enterLoginMfa,
  settled,
  assertNoStoredAttempt,
} from './owner-login-ui.mjs';

test.afterEach(async ({ page }) => checkOwnerLoginRequests(page));

test('disabled absent or invalid availability cannot accept receipt material or probe a proof and password remains usable', async ({
  page,
}) => {
  for (const availability of [false, 404, 'malformed', 'disconnect']) {
    const state = await ownerLoginSetup(page);
    state.availability = availability;
    expect(state.calls).toEqual([]);
    await openOwnerLogin(page);
    await expect(panel(page).getByRole('alert')).toContainText('no est\u00e1 disponible');
    await expect(panel(page).locator('input[type="file"]')).toHaveCount(0);
    expect(state.calls.map((call) => [call.method, call.path])).toEqual([
      ['GET', `${loginPath}/availability`],
    ]);
    await back(page).click();
    await login(page, false, false);
    expect(state.starts).toEqual([]);
    expect(state.proofs).toEqual([]);
    expect(state.mfas).toHaveLength(1);
    expect(state.mfas[0].certificate).toBe(false);
    await checkOwnerLoginRequests(page);
  }
});

test('public receipt and external182-byte signing reach the existing MFA only after an explicit one-use proof', async ({
  page,
}, testInfo) => {
  const state = await ownerLoginSetup(page);
  await openOwnerLogin(page);
  const input = panel(page).getByLabel('Recibo p\u00fablico del v\u00ednculo', { exact: true });
  for (const secret of [
    '-----BEGIN PRIVATE KEY-----\nprivate-selection-sentinel\n-----END PRIVATE KEY-----',
    JSON.stringify({ private_key: 'private-selection-sentinel' }),
  ]) {
    await input.setInputFiles({
      name: 'rejected-container.json',
      mimeType: 'application/json',
      buffer: Buffer.from(secret),
    });
    await expect(panel(page).getByRole('alert')).toBeVisible();
    await expect(prepare(page)).toBeDisabled();
    await expect(input).toHaveValue('');
    expect(state.starts).toEqual([]);
    expect(state.calls.some((call) => call.body?.includes('private-selection-sentinel'))).toBe(
      false,
    );
  }
  await prepareLogin(page);
  await expect(panel(page)).toContainText(selection.subject);
  await expect(panel(page)).toContainText(selection.bindingId);
  expect(state.starts).toHaveLength(1);
  expect(state.starts[0].input).toEqual({
    owner_id: selection.ownerId,
    binding_id: selection.bindingId,
  });
  const bytes = await downloadedBytes(page, download(page));
  expect(bytes).toHaveLength(182);
  expect(bytes).toEqual(Buffer.from(state.starts[0].value.statement_base64, 'base64'));
  expect(bytes.subarray(0, 8).toString()).toBe('OWNAUTH1');
  await selectSignature(page);
  await captureOwnerLayouts(page, testInfo, 'owner-login-prepared');
  expect(state.proofs).toEqual([]);
  expect(state.mfas).toEqual([]);
  await proof(page).click();
  await expect(page.getByRole('heading', { name: 'Un paso m\u00e1s.', exact: true })).toBeVisible();
  expect(state.proofs).toHaveLength(1);
  expect(state.proofs[0].input).toEqual({
    challenge_token: state.starts[0].value.challenge_token,
    signature_base64: signature,
  });
  expect(state.mfas).toEqual([]);
  expect(state.calls.some((call) => call.path === '/api/v1/dashboard')).toBe(false);
  await captureOwnerLayouts(page, testInfo, 'owner-login-mfa');
  await assertNoStoredAttempt(page, [state.starts[0].value.challenge_token, signature]);
  await enterLoginMfa(page, true);
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  expect(state.mfas).toHaveLength(1);
  expect(state.mfas[0]).toMatchObject({ path: '/api/v1/auth/mfa/recovery', certificate: true });
  expect(state.proofs).toHaveLength(1);
});

test('late file start and proof results cannot return after method change or a recovery link', async ({
  page,
}) => {
  for (const stage of ['file', 'start', 'proof']) {
    const state = await ownerLoginSetup(page);
    await openOwnerLogin(page);
    let gate = null;
    if (stage === 'file') {
      await page.evaluate(() => {
        const read = File.prototype.arrayBuffer;
        File.prototype.arrayBuffer = function () {
          if (this.name !== 'held-receipt.json') return read.call(this);
          window.ownerReceiptReadEntered = true;
          return new Promise((resolve) => {
            window.releaseOwnerReceipt = () => resolve(read.call(this));
          });
        };
      });
      await selectReceipt(page, publicReceipt(), 'held-receipt.json');
      await expect.poll(() => page.evaluate(() => window.ownerReceiptReadEntered)).toBe(true);
    } else if (stage === 'start') {
      await selectReceipt(page);
      gate = holdLogin(state, `${loginPath}/start`);
      await prepare(page).click();
      await expect.poll(() => gate.entered).toBe(true);
    } else {
      await prepareLogin(page);
      await selectSignature(page);
      gate = holdLogin(state, `${loginPath}/proof`);
      await proof(page).click();
      await expect.poll(() => gate.entered).toBe(true);
    }
    try {
      if (stage === 'proof') {
        const reset = Buffer.alloc(32, 90).toString('base64url');
        await page.evaluate((token) => {
          location.hash = `#password-reset=${token}`;
        }, reset);
        await expect(
          page.getByRole('heading', { name: 'Elige una nueva contrase\u00f1a.' }),
        ).toBeVisible();
        expect(new URL(page.url()).hash).not.toContain(reset);
      } else await back(page).click();
      const response = gate
        ? page.waitForResponse((value) => value.url().endsWith(gate.path))
        : null;
      if (gate) gate.release();
      else await page.evaluate(() => window.releaseOwnerReceipt());
      if (response) await (await response).finished();
      await settled(page);
      await expect(panel(page)).toHaveCount(0);
      await expect(
        page.getByRole('heading', { name: 'Un paso m\u00e1s.', exact: true }),
      ).toHaveCount(0);
      expect(state.mfas).toEqual([]);
      expect(state.calls.some((call) => call.path === '/api/v1/dashboard')).toBe(false);
      if (stage === 'proof')
        await page
          .getByRole('button', { name: 'Volver al inicio de sesi\u00f3n', exact: true })
          .click();
      await login(page, false, false);
      expect(state.mfas).toHaveLength(1);
      expect(state.mfas[0].certificate).toBe(false);
      await checkOwnerLoginRequests(page);
    } finally {
      gate?.release();
    }
  }
});

test('uncertain proof and a locally expired nonce require explicit fresh preparation without a second proof', async ({
  page,
}) => {
  await page.clock.install({ time: new Date('2026-10-03T18:00:00Z') });
  await page.clock.pauseAt(new Date('2026-10-03T18:00:01Z'));
  const state = await ownerLoginSetup(page);
  await openOwnerLogin(page);
  await prepareLogin(page);
  await selectSignature(page);
  state.nextProof = 'disconnect';
  await proof(page).click();
  await expect(panel(page).getByRole('alert')).toBeVisible();
  await expect(again(page)).toBeEnabled();
  await expect(signatureInput(page)).toHaveCount(0);
  expect(state.proofs).toHaveLength(1);
  expect(state.starts).toHaveLength(1);
  expect(state.mfas).toEqual([]);
  await again(page).click();
  await expect(download(page)).toBeEnabled();
  expect(state.starts).toHaveLength(2);
  expect(state.starts[1].value.challenge_token).not.toBe(state.starts[0].value.challenge_token);
  expect(state.starts[1].value.statement_base64).not.toBe(state.starts[0].value.statement_base64);
  await selectSignature(page);
  await page.clock.fastForward(300000);
  if ((await proof(page).isVisible()) && (await proof(page).isEnabled())) await proof(page).click();
  await expect(panel(page).getByRole('alert')).toContainText(/venci|expir/);
  await expect(again(page)).toBeEnabled();
  expect(state.proofs).toHaveLength(1);
  expect(state.starts).toHaveLength(2);
  expect(state.mfas).toEqual([]);
  await assertNoStoredAttempt(
    page,
    state.starts.map((item) => item.value.challenge_token),
  );
});

test('rejected or mismatched MFA clears the certificate attempt and only a newly signed challenge can admit the Owner', async ({
  page,
}) => {
  for (const outcome of [
    { status: 401 },
    { user: { ...owner, id: '77777777-7777-4777-8777-777777777777' } },
    { user: { ...owner, role: 'litigator' } },
  ]) {
    const state = await ownerLoginSetup(page);
    await openOwnerLogin(page);
    await prepareLogin(page);
    await selectSignature(page);
    await proof(page).click();
    state.nextMfa = outcome;
    await enterLoginMfa(page);
    await expect(panel(page)).toBeVisible();
    await expect(panel(page).getByRole('alert')).toBeVisible();
    await expect(again(page)).toBeEnabled();
    await expect(page.getByRole('button', { name: 'Verificar y entrar', exact: true })).toHaveCount(
      0,
    );
    await expect(signatureInput(page)).toHaveCount(0);
    expect(state.proofs).toHaveLength(1);
    expect(state.mfas).toHaveLength(1);
    expect(state.calls.some((call) => call.path === '/api/v1/dashboard')).toBe(false);
    await again(page).click();
    await expect(download(page)).toBeEnabled();
    expect(state.starts).toHaveLength(2);
    await selectSignature(page);
    await proof(page).click();
    await enterLoginMfa(page);
    await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
    expect(state.mfas).toHaveLength(2);
    expect(state.proofs).toHaveLength(2);
    expect(state.proofs[1].input.challenge_token).not.toBe(state.proofs[0].input.challenge_token);
    await checkOwnerLoginRequests(page);
  }
});
