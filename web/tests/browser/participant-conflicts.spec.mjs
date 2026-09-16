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

test('archive conflict retains only status intent and preserves concurrent text on explicit retry', async ({
  page,
}, testInfo) => {
  const state = await participantSetup(page);
  await openParticipant(page);
  await page.getByRole('button', { name: 'Archivar participante', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Archivar participante', exact: true });
  state.records.get(participantId).push({
    ...participant,
    revision: 2,
    display_name: 'Nombre actualizado',
    procedural_role: 'Rol actualizado',
  });
  await modal.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
  await expect(modal.getByRole('alert')).toContainText('cambi');
  await expect(
    modal.getByRole('button', { name: 'Confirmar archivo', exact: true }),
  ).toBeDisabled();
  await modal.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(modal.getByText('Nombre actualizado', { exact: true })).toBeVisible();
  await expect(modal.getByRole('alert')).toHaveCount(0);
  await modal
    .getByRole('button', { name: 'Confirmar archivo', exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('participant-status-conflict.png') });
  await modal.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
  await expect(modal).not.toBeVisible();
  expect(state.calls.filter((call) => call.method === 'PUT').map((call) => call.body)).toEqual([
    { expected_revision: 1, directory_status: 'archived' },
    { expected_revision: 2, directory_status: 'archived' },
  ]);
  expect(state.records.get(participantId).at(-1)).toMatchObject({
    revision: 3,
    display_name: 'Nombre actualizado',
    procedural_role: 'Rol actualizado',
    directory_status: 'archived',
  });
});

test('already completed status and revision exhaustion do not invite blind resubmission', async ({
  page,
}) => {
  const state = await participantSetup(page);
  await openParticipant(page);
  await page.getByRole('button', { name: 'Archivar participante', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Archivar participante', exact: true });
  state.records
    .get(participantId)
    .push({ ...participant, revision: 2, directory_status: 'archived' });
  await modal.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
  await modal.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(modal.getByRole('status')).toContainText('ya est');
  await expect(
    modal.getByRole('button', { name: 'Confirmar archivo', exact: true }),
  ).toBeDisabled();
  await modal.getByRole('button', { name: 'Cerrar', exact: true }).click();
  await expect(page.locator('.participant-row')).toHaveCount(0);
  await page.route('**/participants/*/directory-status', (route) =>
    route.fulfill({ status: 409, json: { error: { code: 'participant_revision_exhausted' } } }),
  );
  await page.getByRole('button', { name: 'Reactivar participante', exact: true }).click();
  const reactivation = page.getByRole('dialog', { name: 'Reactivar participante', exact: true });
  await reactivation
    .getByRole('button', { name: 'Confirmar reactivaci\u00f3n', exact: true })
    .click();
  await expect(reactivation.getByRole('alert')).toContainText('l\u00edmite');
  await expect(
    reactivation.getByRole('button', { name: 'Consultar datos actuales', exact: true }),
  ).toHaveCount(0);
  await expect(
    reactivation.getByRole('button', { name: 'Confirmar reactivaci\u00f3n', exact: true }),
  ).toBeDisabled();
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

test('conflict refresh rechecks filter membership while retaining the current detail and draft', async ({
  page,
}) => {
  const state = await participantSetup(page);
  await page.getByLabel('Buscar por nombre', { exact: true }).fill('Ana');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await openParticipant(page);
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  await editor(page).getByLabel('Nombre del participante', { exact: true }).fill('Borrador Ana');
  state.records.get(participantId).push({
    ...participant,
    revision: 2,
    display_name: 'Nombre diferente',
    directory_status: 'archived',
  });
  await editor(page).getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await editor(page).getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(editor(page).getByText('Nombre diferente', { exact: true })).toBeVisible();
  await expect(editor(page).getByLabel('Nombre del participante', { exact: true })).toHaveValue(
    'Borrador Ana',
  );
  await editor(page).getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expect(page.locator('.participant-row')).toHaveCount(0);
  await expect(
    detail(page).getByRole('heading', { name: 'Nombre diferente', exact: true }),
  ).toBeVisible();
  await expect(detail(page).getByText('Revisi\u00f3n 2', { exact: true })).toBeVisible();
});
