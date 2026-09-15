import { test, expect } from '@playwright/test';
import {
  participantSetup,
  participant,
  directory,
  detail,
  openParticipant,
} from './participant-helpers.mjs';

test('case-local directory creates, edits and archives without changing account access', async ({
  page,
}) => {
  const state = await participantSetup(page, 'owner', []);
  await expect(
    page.getByText('A\u00fan no hay participantes en esta consulta', { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  let dialog = page.getByRole('dialog', { name: 'Agregar participante', exact: true });
  await expect(
    dialog.getByText('Registrar a esta persona no le da acceso al sistema.', { exact: true }),
  ).toBeVisible();
  await dialog
    .getByLabel('Nombre del participante', { exact: true })
    .fill('Jos\u00e9,  Mu\u00f1oz');
  await dialog.getByLabel('Rol en el expediente', { exact: true }).fill('Cliente');
  await dialog.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(detail(page).getByText('Revisi\u00f3n 1', { exact: true })).toBeVisible();
  expect(state.calls.find((call) => call.method === 'POST').body).toEqual({
    display_name: 'Jos\u00e9,  Mu\u00f1oz',
    procedural_role: 'Cliente',
    organization: null,
    legal_status: null,
  });
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  dialog = page.getByRole('dialog', { name: 'Editar participante', exact: true });
  await dialog.getByLabel('Organizaci\u00f3n (opcional)', { exact: true }).fill('Oficina');
  await dialog.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(detail(page).getByText('Oficina', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Archivar participante', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
  await expect(directory(page).getByText('Jos\u00e9,  Mu\u00f1oz', { exact: true })).toHaveCount(0);
  await page.getByLabel('Estado del directorio', { exact: true }).selectOption('archived');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await openParticipant(page, 'Jos\u00e9,  Mu\u00f1oz');
  await expect(detail(page).getByText('Archivado', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  await expect(page.locator('.participant-revision')).toHaveCount(3);
  await page.getByText('Cambio 1', { exact: true }).click();
  await expect(
    page.locator('.participant-revision[open]').getByText('autor@example.com', { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Reactivar participante', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar reactivaci\u00f3n', exact: true }).click();
  expect(
    state.calls.filter((call) => call.path.endsWith('/directory-status')).map((call) => call.body),
  ).toEqual([
    { expected_revision: 2, directory_status: 'archived' },
    { expected_revision: 3, directory_status: 'active' },
  ]);
  expect(state.requests.some((call) => /\/users|\/members/.test(call.path))).toBe(false);
});

test('directory permissions keep paralegal read-only and client outside participant requests', async ({
  page,
}) => {
  await participantSetup(page, 'paralegal');
  await openParticipant(page);
  await expect(page.getByRole('button', { name: 'Agregar participante', exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByRole('button', { name: 'Editar participante', exact: true })).toHaveCount(
    0,
  );
  await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  await expect(page.locator('.participant-revision')).toHaveCount(1);
  await page.unrouteAll({ behavior: 'wait' });
  const state = await participantSetup(page, 'client');
  await expect(page.getByRole('link', { name: 'Participantes', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'participants';
  });
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  expect(state.calls).toEqual([]);
});
