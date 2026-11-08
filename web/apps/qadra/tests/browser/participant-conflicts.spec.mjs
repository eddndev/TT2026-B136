import { test, expect } from '@playwright/test';
import {
  participantSetup,
  participant,
  participantId,
  detail,
  openParticipant,
} from './participant-helpers.mjs';
const editor = (page) => page.getByRole('dialog', { name: 'Editar participante', exact: true });

test('full edit keeps its draft and explicitly compares a concurrent revision and state', async ({
  page,
}) => {
  const state = await participantSetup(page);
  await openParticipant(page);
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  await editor(page).getByLabel('Nombre del participante', { exact: true }).fill('Mi borrador');
  state.records.get(participantId).push({
    ...participant,
    revision: 2,
    display_name: 'Cambio concurrente',
    directory_status: 'archived',
  });
  await editor(page).getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(editor(page).getByRole('alert')).toContainText('cambiaron');
  await expect(editor(page).getByLabel('Nombre del participante', { exact: true })).toHaveValue(
    'Mi borrador',
  );
  expect(state.calls.filter((call) => call.method === 'PUT')).toHaveLength(1);
  await editor(page).getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(
    editor(page)
      .getByRole('region', { name: 'Valores actuales guardados' })
      .getByText('Cambio concurrente', { exact: true }),
  ).toBeVisible();
  await expect(editor(page).getByText(/incluido el estado/)).toBeVisible();
  await expect(editor(page).getByRole('alert')).toHaveCount(0);
  await editor(page).getByRole('button', { name: 'Guardar mis cambios', exact: true }).click();
  await expect(
    detail(page).getByRole('heading', { name: 'Mi borrador', exact: true }),
  ).toBeVisible();
  expect(
    state.calls.filter((call) => call.method === 'PUT').map((call) => call.body.expected_revision),
  ).toEqual([1, 2]);
  expect(state.records.get(participantId).at(-1).directory_status).toBe('active');
});

test('field validation uses scalar limits and an uncertain response retains the editor without retry', async ({
  page,
}) => {
  const state = await participantSetup(page);
  await page.getByRole('button', { name: 'Registrar ficha pendiente', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Agregar participante', exact: true });
  const name = modal.getByLabel('Nombre del participante', { exact: true });
  await name.fill('\u{1f600}'.repeat(201));
  await modal.getByLabel('Rol en el expediente', { exact: true }).fill('Rol');
  await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(name).toBeFocused();
  await expect(name).toHaveAttribute('aria-invalid', 'true');
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(0);
  await name.fill('\u{1f600}'.repeat(200));
  let sent = 0;
  await page.route('**/participants', (route) => {
    if (route.request().method() !== 'POST') return route.fallback();
    sent++;
    return route.abort('failed');
  });
  await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(modal.getByRole('alert')).toContainText('No se pudo confirmar');
  await expect(name).toHaveValue('\u{1f600}'.repeat(200));
  expect(sent).toBe(1);
});
