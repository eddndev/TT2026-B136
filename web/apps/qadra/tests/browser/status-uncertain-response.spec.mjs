import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import {
  participantSetup,
  participant,
  participantId,
  openParticipant,
} from './participant-helpers.mjs';

for (const applied of [false, true]) {
  test(`case status 503 requires an explicit current read before a new decision, applied=${applied}`, async ({
    page,
  }) => {
    await setup(page);
    let current = administration(caseRecord),
      writes = 0,
      reads = 0;
    await page.route(`**/cases/${caseRecord.id}/administration`, (route) => {
      expect(route.request().method()).toBe('GET');
      reads++;
      return route.fulfill({ json: current });
    });
    await page.route(`**/cases/${caseRecord.id}/administrative-status`, (route) => {
      expect(route.request().method()).toBe('PUT');
      expect(route.request().postDataJSON()).toEqual({
        expected_revision: 1,
        administrative_status: 'closed',
      });
      writes++;
      if (applied) current = administration(caseRecord, 2, null, 'closed');
      return route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
    });
    await login(page, false, false);
    await navigate(page, 'Expedientes');
    await page.getByRole('button', { name: /Defensa inicial/ }).click();
    await page.getByRole('button', { name: 'Cerrar administrativamente', exact: true }).click();
    const dialog = page.getByRole('dialog');
    const confirm = dialog.getByRole('button', {
      name: 'Confirmar cierre administrativo',
      exact: true,
    });
    const before = reads;
    await confirm.click();
    await expect(dialog.getByRole('alert')).toBeVisible();
    await expect(confirm).toBeDisabled();
    expect(writes).toBe(1);
    expect(reads).toBe(before);
    await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
    await expect.poll(() => reads).toBe(before + 1);
    await expect(dialog.getByRole('status')).toContainText(
      applied ? 'ya tiene el estado' : 'Datos actuales consultados',
    );
    if (applied) await expect(confirm).toBeDisabled();
    else await expect(confirm).toBeEnabled();
    expect(writes).toBe(1);
  });
}

for (const failure of ['network', '503'])
  for (const applied of [false, true]) {
    test(`participant status ${failure} requires a fresh record without replay, applied=${applied}`, async ({
      page,
    }) => {
      const state = await participantSetup(page);
      await openParticipant(page);
      let writes = 0;
      await page.route(`**/participants/${participantId}/directory-status`, (route) => {
        expect(route.request().method()).toBe('PUT');
        expect(route.request().postDataJSON()).toEqual({
          expected_revision: 1,
          directory_status: 'archived',
        });
        writes++;
        if (applied)
          state.records.get(participantId).push({
            ...participant,
            revision: 2,
            directory_status: 'archived',
            display_name: 'Nombre actual',
          });
        return failure === 'network'
          ? route.abort('failed')
          : route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
      });
      await page.getByRole('button', { name: 'Archivar participante', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Archivar participante', exact: true });
      const confirm = dialog.getByRole('button', { name: 'Confirmar archivo', exact: true });
      const reads = () =>
        state.calls.filter(
          (call) => call.method === 'GET' && call.path.endsWith(`/participants/${participantId}`),
        ).length;
      const before = reads();
      await confirm.click();
      await expect(dialog.getByRole('alert')).toBeVisible();
      await expect(confirm).toBeDisabled();
      expect(writes).toBe(1);
      expect(reads()).toBe(before);
      await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
      await expect.poll(reads).toBe(before + 1);
      await expect(dialog.getByRole('status')).toContainText(
        applied ? 'ya est' : 'Datos actuales consultados',
      );
      if (applied) {
        await expect(dialog.getByText('Nombre actual', { exact: true })).toBeVisible();
        await expect(confirm).toBeDisabled();
      } else await expect(confirm).toBeEnabled();
      expect(writes).toBe(1);
    });
  }

test('closing and reopening an uncertain case status keeps the current-read requirement', async ({
  page,
}) => {
  await setup(page);
  let writes = 0,
    reads = 0;
  await page.route(`**/cases/${caseRecord.id}/administration`, (route) => {
    expect(route.request().method()).toBe('GET');
    reads++;
    return route.fulfill({ json: administration(caseRecord) });
  });
  await page.route(`**/cases/${caseRecord.id}/administrative-status`, (route) => {
    expect(route.request().method()).toBe('PUT');
    expect(route.request().postDataJSON()).toEqual({
      expected_revision: 1,
      administrative_status: 'closed',
    });
    writes++;
    return route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  const open = page.getByRole('button', { name: 'Cerrar administrativamente', exact: true });
  await open.click();
  const dialog = page.getByRole('dialog');
  const confirm = dialog.getByRole('button', {
    name: 'Confirmar cierre administrativo',
    exact: true,
  });
  const before = reads;
  await confirm.click();
  await expect(dialog.getByRole('alert')).toBeVisible();
  expect(reads).toBe(before);
  expect(writes).toBe(1);
  for (let cycle = 0; cycle < 2; cycle++) {
    await dialog.getByRole('button', { name: 'Cancelar', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await open.click();
    await expect(confirm).toBeDisabled();
    expect(reads).toBe(before + cycle);
    expect(writes).toBe(1);
    await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
    await expect.poll(() => reads).toBe(before + cycle + 1);
    await expect(confirm).toBeEnabled();
    expect(writes).toBe(1);
  }
});

test('closing and reopening an uncertain participant status keeps the current-read requirement', async ({
  page,
}) => {
  const state = await participantSetup(page);
  await openParticipant(page);
  let writes = 0;
  await page.route(`**/participants/${participantId}/directory-status`, (route) => {
    expect(route.request().method()).toBe('PUT');
    expect(route.request().postDataJSON()).toEqual({
      expected_revision: 1,
      directory_status: 'archived',
    });
    writes++;
    return route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
  });
  const reads = () =>
    state.calls.filter(
      (call) => call.method === 'GET' && call.path.endsWith(`/participants/${participantId}`),
    ).length;
  const open = page.getByRole('button', { name: 'Archivar participante', exact: true });
  await open.click();
  const dialog = page.getByRole('dialog', { name: 'Archivar participante', exact: true });
  const confirm = dialog.getByRole('button', { name: 'Confirmar archivo', exact: true });
  const before = reads();
  await confirm.click();
  await expect(dialog.getByRole('alert')).toBeVisible();
  expect(reads()).toBe(before);
  expect(writes).toBe(1);
  for (let cycle = 0; cycle < 2; cycle++) {
    await dialog.getByRole('button', { name: 'Cancelar', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await open.click();
    await expect(confirm).toBeDisabled();
    expect(reads()).toBe(before + cycle);
    expect(writes).toBe(1);
    await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
    await expect.poll(reads).toBe(before + cycle + 1);
    await expect(confirm).toBeEnabled();
    expect(writes).toBe(1);
  }
});
