import { test, expect } from '@playwright/test';
import { memberActor, memberId } from '../fixtures/members.mjs';
import {
  setupMembers,
  enterDirectory,
  directory,
  editor,
  openAccess,
  reviewAccess,
} from './members-helpers.mjs';

const writes = (state) =>
  state.calls.filter((call) => call.method === 'PUT' && call.path.endsWith('/access'));

test('access conflict preserves intent until the Owner explicitly accepts the current revision', async ({
  page,
}) => {
  const state = await setupMembers(page);
  await enterDirectory(page);
  const target = state.rows.find((row) => row.id === memberId(2));
  const previous = target.revision;
  let conflict = true;
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/access') || call.method !== 'PUT' || !conflict) return false;
    conflict = false;
    target.revision = String(BigInt(target.revision) + 1n);
    await route.fulfill({ status: 409, json: { error: { code: 'user_revision_conflict' } } });
    return true;
  };
  await openAccess(page, target);
  await reviewAccess(page, 'litigator', false);
  await editor(page)
    .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
    .click();
  await expect(editor(page).getByRole('alert')).toBeVisible();
  expect(writes(state)).toHaveLength(1);
  expect(writes(state)[0].body.expected_revision).toBe(previous);
  await editor(page).getByRole('button', { name: 'Consultar cuenta actual', exact: true }).click();
  await expect(editor(page)).toContainText(target.revision);
  await editor(page)
    .getByRole('button', { name: 'Usar revisi\u00f3n actual y conservar cambios', exact: true })
    .click();
  await expect(
    editor(page).getByRole('combobox', { name: 'Rol de la cuenta', exact: true }),
  ).toHaveValue('litigator');
  await expect(
    editor(page).getByRole('combobox', { name: 'Estado de la cuenta', exact: true }),
  ).toHaveValue('inactive');
  const accepted = target.revision;
  await editor(page).getByRole('button', { name: 'Revisar acceso', exact: true }).click();
  await editor(page)
    .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
    .click();
  await expect(editor(page)).toHaveCount(0);
  expect(writes(state)).toHaveLength(2);
  expect(writes(state)[1].body).toEqual({
    expected_revision: accepted,
    role: 'litigator',
    active: false,
  });
});

test('the last active Owner cannot be removed and the rejected change does not log out', async ({
  page,
}) => {
  const state = await setupMembers(page, { rows: [structuredClone(memberActor)] });
  await enterDirectory(page);
  await openAccess(page, memberActor);
  await reviewAccess(page, 'owner', false);
  await editor(page)
    .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
    .click();
  await expect(editor(page).getByRole('alert')).toContainText(/\u00faltimo|administrador/i);
  await expect(directory(page)).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toHaveCount(0);
  expect(state.rows[0]).toEqual(memberActor);
  expect(writes(state)).toHaveLength(1);
});

test('a confirmed self role change returns to login and clears account data without remote logout', async ({
  page,
}) => {
  const state = await setupMembers(page);
  await enterDirectory(page);
  await openAccess(page, memberActor);
  await reviewAccess(page, 'paralegal', true);
  await editor(page)
    .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
    .click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await expect(directory(page)).toHaveCount(0);
  await expect(editor(page)).toHaveCount(0);
  expect(writes(state)).toHaveLength(1);
  expect(
    [...state.calls, ...state.baseCalls].some((call) => call.path.endsWith('/auth/logout')),
  ).toBe(false);
  expect(
    await page.evaluate(
      () => Object.keys(localStorage).length + Object.keys(sessionStorage).length,
    ),
  ).toBe(0);
});

test('an uncertain access response requires a read before another explicit submission', async ({
  page,
}) => {
  const state = await setupMembers(page);
  await enterDirectory(page);
  const target = state.rows.find((row) => row.id === memberId(2)),
    before = structuredClone(target);
  let lost = true;
  state.handle = async (route, call) => {
    if (call.method !== 'PUT' || !lost) return false;
    lost = false;
    await route.abort('failed');
    return true;
  };
  await openAccess(page, target);
  await reviewAccess(page, 'litigator', true);
  await editor(page)
    .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
    .click();
  await expect(editor(page).getByRole('alert')).toBeVisible();
  expect(writes(state)).toHaveLength(1);
  await expect(
    editor(page).getByRole('button', { name: 'Confirmar cambio de acceso', exact: true }),
  ).toHaveCount(0);
  await editor(page).getByRole('button', { name: 'Consultar cuenta actual', exact: true }).click();
  expect(target).toEqual(before);
  await editor(page)
    .getByRole('button', { name: 'Usar revisi\u00f3n actual y conservar cambios', exact: true })
    .click();
  await editor(page).getByRole('button', { name: 'Revisar acceso', exact: true }).click();
  await editor(page)
    .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
    .click();
  await expect(editor(page)).toHaveCount(0);
  expect(writes(state)).toHaveLength(2);
  expect(writes(state)[1].body).toEqual(writes(state)[0].body);
});
