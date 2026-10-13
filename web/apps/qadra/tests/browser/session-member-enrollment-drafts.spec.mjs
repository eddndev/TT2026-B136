import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  enrollmentSetup,
  checkEnrollmentRequests,
  holdEnrollmentRequest,
  armEnrollment,
  usersPath,
  principalPath,
  submittedEmail,
} from './session-member-enrollment-fixtures.mjs';
import {
  enterEnrollment,
  fillEnrollment,
  expectBlankEnrollment,
  expectNoEnrollmentMaterial,
  email,
  password,
  role,
  create,
  resume,
  rawEmail,
  panel,
} from './session-member-enrollment-ui.mjs';

test.afterEach(async ({ page }) => checkEnrollmentRequests(page));

test('same Owner restores partial email and role only after fresh authority while the initial password stays empty', async ({
  page,
}) => {
  const state = await enrollmentSetup(page);
  await login(page, false, false);
  await enterEnrollment(page);
  const element = await fillEnrollment(page);
  await expire(page, state, element);
  await login(page, true, false);
  await enterEnrollment(page);
  await expect(resume(page)).toBeEnabled();
  const before = state.calls.length;
  const fresh = holdEnrollmentRequest(state, 'GET', principalPath);
  await resume(page).click();
  await expect.poll(() => fresh.entered).toBe(true);
  expect(await email(page).evaluateAll((nodes) => nodes.map((node) => node.value))).not.toContain(
    rawEmail,
  );
  expect(await create(page).evaluateAll((nodes) => nodes.some((node) => !node.disabled))).toBe(
    false,
  );
  fresh.release();
  await expect(email(page)).toHaveValue(rawEmail);
  await expect(role(page)).toHaveValue('litigator');
  await expectNoEnrollmentMaterial(page);
  await expect(password(page)).toBeEditable();
  expect(state.enrollmentWrites).toEqual([]);
  const reads = state.calls.slice(before).filter((call) => call.path === principalPath);
  expect(reads).toHaveLength(1);
  expect(reads[0]).toMatchObject({
    method: 'GET',
    body: null,
    search: '',
    headers: { authorization: `Bearer ${state.current.token}` },
  });
});

test('another Owner and explicit logout cannot recover a former nonsecret enrollment draft', async ({
  page,
}) => {
  const state = await enrollmentSetup(page);
  await login(page, false, false);
  await enterEnrollment(page);
  await expire(page, state, await fillEnrollment(page));
  await signInOther(page);
  await enterEnrollment(page);
  await expectBlankEnrollment(page);
  await fillEnrollment(page, 'Other.Unfinished@');
  await page
    .getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true })
    .filter({ visible: true })
    .click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await login(page, false, false);
  await enterEnrollment(page);
  await expectBlankEnrollment(page);
  expect(state.enrollmentWrites).toEqual([]);
});

test('fresh loss of Owner authority discards an unopened enrollment draft instead of reviving it when authority returns', async ({
  page,
}) => {
  const state = await enrollmentSetup(page);
  await login(page, false, false);
  await enterEnrollment(page);
  await expire(page, state, await fillEnrollment(page));
  await login(page, false, false);
  await enterEnrollment(page);
  await expect(resume(page)).toBeEnabled();
  state.enrollmentIdentity = { ...state.current.user, role: 'litigator' };
  await resume(page).click();
  await expect(panel(page).getByRole('alert')).toBeVisible();
  expect(await email(page).evaluateAll((nodes) => nodes.map((node) => node.value))).not.toContain(
    rawEmail,
  );
  expect(await create(page).evaluateAll((nodes) => nodes.some((node) => !node.disabled))).toBe(
    false,
  );
  state.enrollmentIdentity = null;
  await navigate(page, 'Inicio');
  await enterEnrollment(page);
  await expectBlankEnrollment(page);
  expect(state.enrollmentWrites).toEqual([]);
});

test('a confirmed enrollment retires its nonsecret capture before directory refresh and never restores one-time MFA material', async ({
  page,
}) => {
  const state = await enrollmentSetup(page);
  await login(page, false, false);
  await enterEnrollment(page);
  const element = await fillEnrollment(page, submittedEmail);
  armEnrollment(state);
  await create(page).click();
  await expect(
    page.getByRole('heading', { name: 'Configura el segundo factor', exact: true }),
  ).toBeVisible();
  expect(state.enrollmentWrites).toHaveLength(1);
  await page
    .getByRole('checkbox', { name: 'Ya guard\u00e9 la clave y los c\u00f3digos', exact: true })
    .check();
  const refresh = holdEnrollmentRequest(state, 'GET', usersPath);
  await page.getByRole('button', { name: 'Finalizar', exact: true }).click();
  await expect.poll(() => refresh.entered).toBe(true);
  await expire(page, state, element);
  refresh.release();
  await login(page, false, false);
  await enterEnrollment(page);
  await expectBlankEnrollment(page);
  expect(state.enrollmentWrites).toHaveLength(1);
});
