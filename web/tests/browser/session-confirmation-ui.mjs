import { expect } from '@playwright/test';
import { navigate } from './helpers.mjs';

export async function beginConfirmationCase(page) {
  await navigate(page, 'Expedientes');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('all');
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
}
export async function enterConfirmationCase(page) {
  await beginConfirmationCase(page);
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}
export function expectOnlyOriginalCommand(state, path, method) {
  expect(state.confirmationWrites).toHaveLength(1);
  expect(state.confirmationWrites[0]).toMatchObject({
    path,
    method,
    headers: { authorization: `Bearer ${state.grants[0].access_token}` },
  });
}
export function expectFreshRead(gate, state) {
  expect(gate.call.headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(gate.call.headers.authorization).not.toBe(`Bearer ${state.grants[0].access_token}`);
}
export async function releaseOldConfirmation(page, gate, state) {
  const response = page.waitForResponse(
    (value) =>
      new URL(value.url()).pathname === gate.path &&
      value.request().method() === gate.method &&
      value.request().headers().authorization === gate.call.headers.authorization,
  );
  gate.release();
  await response;
  await state.advance(20);
}
