import { test, expect } from '@playwright/test';
import {
  typedSetup,
  fillTyped,
  prepareTyped,
  typed,
  subject,
  subjectId,
  pickSupport,
} from './typed-participant-helpers.mjs';
import { detail, participant, openParticipant } from './participant-helpers.mjs';
import { navigate } from './helpers.mjs';
test('completes a manual participant at its next revision without replacing historical manual fields', async ({
  page,
}) => {
  const state = await typedSetup(page);
  await openParticipant(page);
  await detail(page).getByRole('button', { name: 'Completar perfil', exact: true }).click();
  const dialog = page.getByRole('dialog', {
    name: 'Completar perfil de participante',
    exact: true,
  });
  await dialog.getByRole('button', { name: 'Elegir identidad existente', exact: true }).click();
  const picker = dialog.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await picker
    .getByRole('button', { name: 'Consultar identidad: Persona tipificada', exact: true })
    .click();
  await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  await dialog.getByLabel('Tipo de participante', { exact: true }).selectOption('other');
  await dialog.getByLabel('Nombre del rol', { exact: true }).fill('Observador');
  await dialog
    .getByLabel('Estado de Descripci\u00f3n del rol', { exact: true })
    .selectOption('unknown');
  await dialog
    .getByLabel('Motivo de Descripci\u00f3n del rol', { exact: true })
    .fill('No hay mas datos');
  await pickSupport(page, 'Soporte del rol');
  await prepareTyped(dialog);
  await dialog.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const rows = state.records.get(participant.id);
  expect(rows).toHaveLength(2);
  expect(rows[0]).toEqual(participant);
  expect(rows[1].revision).toBe(2);
  await detail(page).getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  const history = page.getByRole('region', { name: 'Historial del participante', exact: true });
  await history.getByText('Cambio 1', { exact: true }).click();
  await expect(history).toContainText('Ficha pendiente de tipificar');
  await history.getByText('Cambio 2', { exact: true }).click();
  await expect(history).toContainText('Identidad vinculada: revisi\u00f3n 1');
});
test('ignores a delayed current-identity response after leaving participants', async ({ page }) => {
  await typedSetup(page, 'owner', [typed]);
  await openParticipant(page, typed.display_name);
  let release, started;
  const pending = new Promise((resolve) => (started = resolve));
  await page.route('**/subjects/*', async (route) => {
    started();
    await new Promise((resolve) => (release = resolve));
    await route.fulfill({
      json: {
        ...subject,
        values: { ...subject.values, name: { state: 'known', value: 'Respuesta vieja privada' } },
      },
    });
  });
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await pending;
  await page.getByRole('link', { name: 'Resumen', exact: true }).click();
  release();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Respuesta vieja privada')).toHaveCount(0);
});
test('coordinates identity and evidence readers so a neighboring history cannot create a third request', async ({
  page,
}) => {
  await typedSetup(page, 'owner', [typed]);
  await openParticipant(page, typed.display_name);
  let release, started;
  const pending = new Promise((resolve) => (started = resolve));
  await page.route('**/subjects/*', async (route) => {
    started();
    await new Promise((resolve) => (release = resolve));
    await route.fulfill({ json: subject });
  });
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await pending;
  await expect(
    detail(page).getByRole('button', { name: 'Ver historial de cambios', exact: true }),
  ).toBeDisabled();
  await expect(
    detail(page).getByRole('button', { name: 'Editar participante', exact: true }),
  ).toBeDisabled();
  release();
  await expect(
    detail(page).getByRole('button', { name: 'Ver historial de cambios', exact: true }),
  ).toBeEnabled();
});
test('clears detail, identity and prepared materials when access is denied', async ({ page }) => {
  await typedSetup(page, 'owner', [typed]);
  await openParticipant(page, typed.display_name);
  await page.route('**/subjects/*', (route) =>
    route.fulfill({ status: 404, json: { error: { code: 'case_not_found' } } }),
  );
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await expect(detail(page)).toHaveCount(0);
  await expect(page.getByText('No consta en el soporte', { exact: false })).toHaveCount(0);
});
test('history exports the attested historical revision explicitly without fetching credentials per row', async ({
  page,
}) => {
  const origin = {
    participant_id: typed.id,
    participant_revision: 1,
    statement_digest: 'e'.repeat(64),
  };
  const state = await typedSetup(page, 'paralegal', [{ ...typed, credential_origin: origin }]);
  state.records.get(typed.id).push({ ...typed, revision: 2, credential_origin: origin });
  await openParticipant(page, typed.display_name);
  await detail(page).getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  const history = page.getByRole('region', { name: 'Historial del participante', exact: true });
  await history.getByText('Cambio 1', { exact: true }).click();
  await expect(
    history
      .getByRole('button', { name: 'Consultar evidencia de firma personal', exact: true })
      .first(),
  ).toBeVisible();
  expect(state.calls.filter((call) => call.path.endsWith('/credential'))).toHaveLength(0);
});

test('a duplicate-role lookup includes archived records and preserves the typed draft', async ({
  page,
}) => {
  const state = await typedSetup(page, 'owner', [{ ...typed, directory_status: 'archived' }]);
  const dialog = await fillTyped(page);
  await page.route('**/participants/proposals/review', (route) =>
    route.fulfill({ status: 409, json: { error: { code: 'participant_role_conflict' } } }),
  );
  await dialog
    .getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true })
    .click();
  await dialog.getByRole('button', { name: 'Consultar directorio completo', exact: true }).click();
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    'Persona tipificada',
  );
  const directory = dialog.getByRole('region', {
    name: 'Consulta del directorio completo',
    exact: true,
  });
  await expect(directory).toContainText('Persona tipificada');
  expect(
    state.calls.some(
      (call) =>
        call.method === 'GET' &&
        call.search?.includes('status=all') &&
        call.search?.includes('kind=defendant'),
    ),
  ).toBe(true);
});
