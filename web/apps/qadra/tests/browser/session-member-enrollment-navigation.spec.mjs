import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  enrollmentSetup,
  checkEnrollmentRequests,
  armEnrollment,
  holdEnrollmentRequest,
  principalPath,
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
} from './session-member-enrollment-ui.mjs';

test.afterEach(async ({ page }) => checkEnrollmentRequests(page));

for (const restored of [false, true]) {
  test(`navigation retains an uncertain enrollment ${restored ? 'after' : 'before'} session recovery until explicit discard`, async ({
    page,
  }) => {
    const state = await enrollmentSetup(page);
    await login(page, false, false);
    await enterEnrollment(page);
    const element = await fillEnrollment(page, submittedEmail);
    armEnrollment(state, { applied: false });
    await create(page).click();
    await expect(consult(page)).toBeEnabled();
    await expect(create(page)).toBeDisabled();
    expect(state.enrollmentWrites).toHaveLength(1);

    if (restored) {
      await expire(page, state, element);
      await login(page, false, false);
      await enterEnrollment(page);
      await resume(page).click();
      await expect(email(page)).toHaveValue(submittedEmail);
      await expect(create(page)).toBeDisabled();
    }

    await navigate(page, 'Inicio');
    await enterEnrollment(page);
    await expect(resume(page)).toBeEnabled();
    await expect(discard(page)).toBeEnabled();
    await expectNoEnrollmentMaterial(page);
    const before = state.calls.length;
    const fresh = holdEnrollmentRequest(state, 'GET', principalPath);
    await resume(page).click();
    await expect.poll(() => fresh.entered).toBe(true);
    expect(await email(page).evaluateAll((nodes) => nodes.map((node) => node.value))).not.toContain(
      submittedEmail,
    );
    await expect(create(page)).toBeDisabled();
    fresh.release();
    await expect(email(page)).toHaveValue(submittedEmail);
    await expect(role(page)).toHaveValue('litigator');
    await expect(consult(page)).toBeEnabled();
    await expect(create(page)).toBeDisabled();
    await expectNoEnrollmentMaterial(page);
    const reads = state.calls.slice(before).filter((call) => call.path === principalPath);
    expect(reads).toHaveLength(1);
    expect(reads[0]).toMatchObject({
      method: 'GET',
      body: null,
      search: '',
      headers: { authorization: `Bearer ${state.current.token}` },
    });
    expect(state.enrollmentWrites).toHaveLength(1);

    await discard(page).click();
    await expectBlankEnrollment(page);
    await navigate(page, 'Inicio');
    await enterEnrollment(page);
    await expectBlankEnrollment(page);
    expect(state.enrollmentWrites).toHaveLength(1);
  });
}
