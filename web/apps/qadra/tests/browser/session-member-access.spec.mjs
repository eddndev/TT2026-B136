import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  memberDraftSetup,
  checkMemberDraftRequests,
  enterMembers,
  prepareAccessDraft,
  directory,
  editor,
  openAccess,
  reviewAccess,
  accessValues,
  closeAccess,
  holdMemberRead,
  armMemberWrite,
  memberPath,
  usersPath,
  mePath,
  reviewButton,
  confirmButton,
  consultButton,
  adoptButton,
  assertNoAccessCommand,
} from './session-member-access-helpers.mjs';

test.afterEach(async ({ page }) => checkMemberDraftRequests(page));

test('member access reentry waits for the current Owner and exact target before restoring unapproved choices', async ({
  page,
}) => {
  const state = await memberDraftSetup(page);
  await login(page, false, false);
  await expire(page, state, await prepareAccessDraft(page, state));
  await login(page, true, false);
  await enterMembers(page);
  const before = state.calls.length;
  const owner = holdMemberRead(state, mePath);
  const target = holdMemberRead(state, memberPath(state.target));
  await directory(page)
    .getByRole('button', { name: `Administrar acceso de ${state.target.email}`, exact: true })
    .click();
  await expect.poll(() => owner.entered).toBe(true);
  expect(target.entered).toBe(false);
  await expect(confirmButton(page)).toHaveCount(0);
  owner.release();
  await expect.poll(() => target.entered).toBe(true);
  expect(
    await editor(page)
      .getByRole('combobox')
      .evaluateAll((nodes) => nodes.map((node) => node.value)),
  ).not.toContain('litigator');
  await expect(confirmButton(page)).toHaveCount(0);
  target.release();
  await accessValues(page, 'litigator', false);
  await expect(reviewButton(page)).toBeEnabled();
  await assertNoAccessCommand(page, state);
  const reads = state.calls
    .slice(before)
    .filter((call) => [mePath, memberPath(state.target)].includes(call.path));
  expect(reads.map((call) => call.path)).toEqual([mePath, memberPath(state.target)]);
  for (const call of reads)
    expect(call).toMatchObject({
      method: 'GET',
      body: null,
      search: '',
      headers: { authorization: `Bearer ${state.current.token}` },
    });
  const element = await editor(page).elementHandle();
  await closeAccess(page);
  await expire(page, state, element);
  await login(page, false, false);
  await enterMembers(page);
  await openAccess(page, state.target);
  await accessValues(page, 'paralegal', true);
  await expect(reviewButton(page)).toBeEnabled();
  await assertNoAccessCommand(page, state);
});

test('a changed member revision stays exact and requires an explicit comparison before a new access command', async ({
  page,
}) => {
  const state = await memberDraftSetup(page);
  const saved = state.target.revision;
  await login(page, false, false);
  await expire(page, state, await prepareAccessDraft(page, state));
  state.target.role = 'client';
  state.target.revision = String(BigInt(saved) + 1n);
  await login(page, false, false);
  await enterMembers(page);
  await openAccess(page, state.target);
  await accessValues(page, 'litigator', false);
  await expect(editor(page)).toContainText(`Revisi\u00f3n consultada ${saved}`);
  await expect(reviewButton(page)).toHaveCount(0);
  await assertNoAccessCommand(page, state);
  await expect(consultButton(page)).toBeEnabled();
  await consultButton(page).click();
  await expect(editor(page)).toContainText(`Revisi\u00f3n actual ${state.target.revision}`);
  await expect(adoptButton(page)).toBeEnabled();
  expect(state.memberWrites).toEqual([]);
  await adoptButton(page).click();
  await accessValues(page, 'litigator', false);
  const accepted = state.target.revision;
  await reviewButton(page).click();
  armMemberWrite(state, state.target.id);
  await confirmButton(page).click();
  await expect(editor(page)).toHaveCount(0);
  expect(state.memberWrites).toHaveLength(1);
  expect(state.memberWrites[0].input).toEqual({
    expected_revision: accepted,
    role: 'litigator',
    active: false,
  });
  expect(accepted).not.toBe(String(Number(saved) + 1));
});

test('an access command retained through expiry never replays and its late response cannot approve the recovered choices', async ({
  page,
}) => {
  const state = await memberDraftSetup(page);
  await login(page, false, false);
  const element = await prepareAccessDraft(page, state);
  const pending = armMemberWrite(state, state.target.id, { hold: true });
  const response = page.waitForResponse(
    (reply) =>
      reply.request().method() === 'PUT' &&
      new URL(reply.url()).pathname === `${memberPath(state.target)}/access`,
  );
  await confirmButton(page).click();
  await expect.poll(() => pending.entered).toBe(true);
  await expire(page, state, element);
  await login(page, false, false);
  await enterMembers(page);
  await openAccess(page, state.target);
  await accessValues(page, 'litigator', false);
  await expect(editor(page)).toContainText('No se pudo confirmar el resultado.');
  await expect(reviewButton(page)).toHaveCount(0);
  await assertNoAccessCommand(page, state, 1);
  pending.release();
  expect((await response).status()).toBe(200);
  await expect(editor(page)).toContainText('No se pudo confirmar el resultado.');
  await assertNoAccessCommand(page, state, 1);
  await consultButton(page).click();
  await expect(adoptButton(page)).toBeEnabled();
  await expect(reviewButton(page)).toHaveCount(0);
  await adoptButton(page).click();
  await expect(reviewButton(page)).toBeEnabled();
  await assertNoAccessCommand(page, state, 1);
});

test('confirmed access clears its draft before directory refresh and before a confirmed own-role change expires the session', async ({
  page,
}) => {
  const state = await memberDraftSetup(page);
  await login(page, false, false);
  const element = await prepareAccessDraft(page, state, 'litigator', true);
  const listing = holdMemberRead(state, usersPath);
  armMemberWrite(state, state.target.id);
  await confirmButton(page).click();
  await expect.poll(() => listing.entered).toBe(true);
  expect(state.memberWrites).toHaveLength(1);
  await expire(page, state, element);
  listing.release();
  state.target.role = 'client';
  state.target.revision = String(BigInt(state.target.revision) + 1n);
  await login(page, false, false);
  await enterMembers(page);
  await openAccess(page, state.target);
  await accessValues(page, 'client', true);
  await expect(reviewButton(page)).toBeEnabled();
  await expect(consultButton(page)).toHaveCount(0);
  await closeAccess(page);
  await openAccess(page, state.actor);
  await reviewAccess(page, 'paralegal', true);
  armMemberWrite(state, state.actor.id);
  await confirmButton(page).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(state.memberWrites).toHaveLength(2);
  expect(state.calls.filter((call) => call.path === '/api/v1/auth/logout')).toEqual([]);
  // A separate Owner restores access; reauthentication must not revive confirmed intent.
  state.actor.role = 'owner';
  state.actor.revision = String(BigInt(state.actor.revision) + 1n);
  await login(page, false, false);
  await enterMembers(page);
  await openAccess(page, state.actor);
  await accessValues(page, 'owner', true);
  await expect(reviewButton(page)).toBeEnabled();
  await expect(consultButton(page)).toHaveCount(0);
  await assertNoAccessCommand(page, state, 2);
});

test('a different Owner cannot inherit account access choices and returning to the first account cannot revive them', async ({
  page,
}) => {
  const state = await memberDraftSetup(page);
  await login(page, false, false);
  await expire(page, state, await prepareAccessDraft(page, state));
  await signInOther(page);
  await enterMembers(page);
  await openAccess(page, state.target);
  await accessValues(page, 'paralegal', true);
  await expect(reviewButton(page)).toBeEnabled();
  await closeAccess(page);
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await login(page, false, false);
  await enterMembers(page);
  await openAccess(page, state.target);
  await accessValues(page, 'paralegal', true);
  await assertNoAccessCommand(page, state);
});

test('fresh loss of member permission discards the pending access choices before any later reopening', async ({
  page,
}) => {
  const state = await memberDraftSetup(page);
  await login(page, false, false);
  await expire(page, state, await prepareAccessDraft(page, state));
  await login(page, false, false);
  state.memberDenied = true;
  await navigate(page, 'Equipo');
  await expect(directory(page).getByRole('alert')).toBeVisible();
  await expect(editor(page)).toHaveCount(0);
  expect(state.memberWrites).toEqual([]);
  state.memberDenied = false;
  await navigate(page, 'Inicio');
  await enterMembers(page);
  await openAccess(page, state.target);
  await accessValues(page, 'paralegal', true);
  await expect(reviewButton(page)).toBeEnabled();
  await assertNoAccessCommand(page, state);
});
