import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  enrollmentSetup,
  checkEnrollmentRequests,
  armEnrollment,
  holdEnrollmentRequest,
  includeSimilarAccounts,
  usersPath,
  submittedEmail,
} from './session-member-enrollment-fixtures.mjs';
import {
  enterEnrollment,
  fillEnrollment,
  expectBlankEnrollment,
  expectNoEnrollmentMaterial,
  email,
  role,
  create,
  resume,
  discard,
  consult,
  panel,
  directory,
} from './session-member-enrollment-ui.mjs';

test.afterEach(async ({ page }) => checkEnrollmentRequests(page));

for (const applied of [true, false]) {
  test(`an uncertain enrollment remains nonrepeatable after full directory lookup with the exact account ${applied ? 'present' : 'absent'}`, async ({
    page,
  }) => {
    const state = await enrollmentSetup(page);
    includeSimilarAccounts(state);
    await login(page, false, false);
    await enterEnrollment(page);
    const element = await fillEnrollment(page, submittedEmail);
    const response = page.waitForResponse(
      (value) => value.request().method() === 'POST' && new URL(value.url()).pathname === usersPath,
    );
    const sent = armEnrollment(state, { applied, hold: true });
    await create(page).click();
    await expect.poll(() => sent.entered).toBe(true);
    await expire(page, state, element);
    await login(page, false, false);
    await enterEnrollment(page);
    await resume(page).click();
    await expect(email(page)).toHaveValue(submittedEmail);
    await expect(role(page)).toHaveValue('litigator');
    await expectNoEnrollmentMaterial(page);
    await expect(create(page)).toBeDisabled();
    expect(state.enrollmentWrites).toHaveLength(1);
    sent.release();
    expect((await response).status()).toBe(applied ? 201 : 503);
    await expect(consult(page)).toBeEnabled();
    await expectNoEnrollmentMaterial(page);
    const before = state.calls.length;
    const listing = holdEnrollmentRequest(state, 'GET', usersPath);
    await consult(page).click();
    await expect.poll(() => listing.entered).toBe(true);
    await expect(create(page)).toBeDisabled();
    expect(state.enrollmentWrites).toHaveLength(1);
    listing.release();
    await expect(consult(page)).toBeEnabled();
    await expect(panel(page)).toContainText(
      applied ? 'Existe una cuenta con este correo.' : 'No se encontro una cuenta con este correo.',
    );
    await expect(panel(page)).toContainText(
      'La consulta del directorio no confirma este envio ni recupera el segundo factor.',
    );
    const reads = state.calls
      .slice(before)
      .filter((call) => call.method === 'GET' && call.path === usersPath);
    expect(reads).toHaveLength(2);
    for (const call of reads) {
      const query = new URLSearchParams(call.search);
      expect(query.get('email_prefix')).toBe(submittedEmail.toLowerCase());
      expect(query.get('status')).toBe('all');
      expect(call.headers.authorization).toBe(`Bearer ${state.current.token}`);
    }
    expect(new URLSearchParams(reads[0].search).has('cursor')).toBe(false);
    expect(new URLSearchParams(reads[1].search).has('cursor')).toBe(true);
    await expect(create(page)).toBeDisabled();
    await expectNoEnrollmentMaterial(page);
    expect(state.enrollmentWrites).toHaveLength(1);
    expect(
      state.enrollmentRows.filter((row) => row.email === submittedEmail.toLowerCase()),
    ).toHaveLength(applied ? 1 : 0);
    await discard(page).click();
    await expect(directory(page)).toBeVisible();
    await expectBlankEnrollment(page);
    const blank = await panel(page).elementHandle();
    await expire(page, state, blank);
    await login(page, false, false);
    await enterEnrollment(page);
    await expectBlankEnrollment(page);
    expect(state.enrollmentWrites).toHaveLength(1);
  });
}
