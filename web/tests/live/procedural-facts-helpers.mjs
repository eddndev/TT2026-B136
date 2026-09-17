import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';
export {
  factEditor,
  factDetail,
  factList,
  fillResolution,
  fillNotification,
} from '../browser/procedural-facts-helpers.mjs';
export { accountAction } from './hearing-result-helpers.mjs';
import { factEditor, factDetail, factList } from '../browser/procedural-facts-helpers.mjs';
export const accounts = fixture.proceduralFacts;
export async function openFacts(page, record = accounts.case) {
  await openCase(page, record);
  await page.getByRole('link', { name: 'Resoluciones', exact: true }).click();
  await expect(factList(page)).toHaveAttribute('aria-busy', 'false');
}
export async function openResolution(page, row) {
  await factList(page)
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${row.id}`, exact: true })
    .click();
  await expect(factDetail(page)).toBeVisible();
}
export async function prepare(page, family = 'resolution', expected = 200) {
  const base = family === 'resolution' ? 'resolutions' : 'notifications';
  const pending = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/${base}/prepare`) && response.request().method() === 'POST',
  );
  await factEditor(page, family)
    .getByRole('button', { name: 'Preparar registro', exact: true })
    .click();
  const response = await pending;
  expect(response.status()).toBe(expected);
  return response.json();
}
export async function submit(page, prepared, expected = 201) {
  const command = prepared.command,
    family = command.family,
    action = command.change.action;
  const base =
    family === 'resolution'
      ? '/resolutions'
      : `/resolutions/${command.resolution_id}/notifications`;
  const path = `${base}${action === 'record' ? '' : `/${command.id}${action === 'withdraw' ? '/withdrawal' : ''}`}`;
  const pending = page.waitForResponse(
    (response) =>
      response.url().endsWith(path) &&
      response.request().method() === (action === 'correct' ? 'PUT' : 'POST'),
  );
  await factEditor(page, family)
    .getByRole('button', { name: 'Confirmar registro', exact: true })
    .click();
  const response = await pending;
  expect(response.status()).toBe(expected);
  if (expected === 201) await expect(factEditor(page, family)).toHaveCount(0);
  return response.json();
}
