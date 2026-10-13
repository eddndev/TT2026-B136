import { test, expect } from '@playwright/test';
import {
  participantSetup,
  participant,
  participantId,
  directory,
  detail,
  openParticipant,
} from './participant-helpers.mjs';

async function openHistory(page) {
  await openParticipant(page);
  await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  await expect(page.locator('.participant-revision')).toHaveCount(1);
}

async function holdRefreshes(page, denial) {
  const releases = {};
  const counts = { list: 0, history: 0 };
  for (const [kind, pattern] of [
    ['list', '**/participants?*'],
    ['history', '**/participants/*/history?*'],
  ]) {
    await page.route(pattern, async (route) => {
      counts[kind]++;
      if (counts[kind] === 1)
        await new Promise((resolve) => {
          releases[kind] = resolve;
        });
      if (denial?.kind === kind)
        return route.fulfill({
          status: denial.status,
          json: { error: { code: 'participant_not_found' } },
        });
      await route.fallback();
    });
  }
  return {
    counts,
    async ready() {
      await expect.poll(() => Object.keys(releases).sort()).toEqual(['history', 'list']);
    },
    async release(kind) {
      const finished = page.waitForEvent('requestfinished', (request) =>
        kind === 'history'
          ? request.url().includes('/history?')
          : request.url().includes('/participants?'),
      );
      releases[kind]();
      await finished;
      await page.evaluate(
        () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
      );
    },
    cleanup() {
      Object.values(releases).forEach((release) => release());
    },
  };
}

for (const mode of ['edit', 'archive']) {
  for (const first of ['list', 'history']) {
    test(`${mode} conflict waits for both refreshed readers when ${first} finishes first`, async ({
      page,
    }) => {
      const state = await participantSetup(page);
      await openHistory(page);
      const editing = mode === 'edit';
      const title = editing ? 'Editar participante' : 'Archivar participante';
      await page.getByRole('button', { name: title, exact: true }).click();
      const modal = page.getByRole('dialog', { name: title, exact: true });
      if (editing)
        await modal.getByLabel('Nombre del participante', { exact: true }).fill('Mi borrador');
      state.records.get(participantId).push({
        ...participant,
        revision: 2,
        display_name: 'Cambio concurrente',
      });
      await modal
        .getByRole('button', {
          name: editing ? 'Guardar participante' : 'Confirmar archivo',
          exact: true,
        })
        .click();
      await expect(modal.getByRole('alert')).toContainText('cambi');
      const held = await holdRefreshes(page);
      try {
        await modal.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
        await held.ready();
        const submit = modal.locator('.dialog-actions button.primary');
        await expect(submit).toBeDisabled();
        if (editing)
          await expect(modal.getByLabel('Nombre del participante', { exact: true })).toHaveValue(
            'Mi borrador',
          );
        expect(state.calls.filter((call) => call.method === 'PUT')).toHaveLength(1);
        await held.release(first);
        await expect(submit).toBeDisabled();
        await held.release(first === 'list' ? 'history' : 'list');
        await expect(submit).toBeEnabled();
        await expect(page.locator('.participant-revision')).toHaveCount(2);
        expect(held.counts).toEqual({ list: 1, history: 1 });
        await submit.click();
        await expect(modal).not.toBeVisible();
        expect(
          state.calls.filter((call) => call.method === 'PUT').map((call) => call.body),
        ).toMatchObject([
          { expected_revision: 1 },
          {
            expected_revision: 2,
            ...(editing ? { display_name: 'Mi borrador' } : { directory_status: 'archived' }),
          },
        ]);
      } finally {
        held.cleanup();
      }
    });
  }
}

test('a confirmed edit keeps its dialog busy until list and open history are refreshed', async ({
  page,
}) => {
  await participantSetup(page);
  await openHistory(page);
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Editar participante', exact: true });
  await modal.getByLabel('Nombre del participante', { exact: true }).fill('Nombre confirmado');
  const held = await holdRefreshes(page);
  try {
    await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
    await held.ready();
    await expect(modal).toBeVisible();
    await expect(modal.locator('.dialog-actions button.primary')).toBeDisabled();
    await held.release('list');
    await expect(modal).toBeVisible();
    await expect(modal.locator('.dialog-actions button.primary')).toBeDisabled();
    await held.release('history');
    await expect(modal).not.toBeVisible();
    await expect(detail(page).getByRole('heading', { name: 'Nombre confirmado' })).toBeVisible();
    await expect(
      directory(page).getByRole('button', { name: 'Abrir Nombre confirmado', exact: true }),
    ).toBeVisible();
    await expect(page.locator('.participant-revision')).toHaveCount(2);
    expect(held.counts).toEqual({ list: 1, history: 1 });
  } finally {
    held.cleanup();
  }
});

for (const status of [403, 404]) {
  test(`history refresh denial ${status} clears protected views before the late list returns`, async ({
    page,
  }) => {
    const state = await participantSetup(page);
    await openHistory(page);
    await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
    const modal = page.getByRole('dialog', { name: 'Editar participante', exact: true });
    await modal.getByLabel('Nombre del participante', { exact: true }).fill('Mi borrador');
    state.records.get(participantId).push({ ...participant, revision: 2 });
    await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
    await expect(modal.getByRole('alert')).toContainText('cambi');
    const held = await holdRefreshes(page, { kind: 'history', status });
    try {
      await modal.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
      await held.ready();
      await held.release('history');
      await expect(detail(page)).toHaveCount(0);
      await expect(page.getByRole('dialog')).toHaveCount(0);
      await expect(directory(page).locator('.participant-row')).toHaveCount(0);
      await held.release('list');
      await expect(detail(page)).toHaveCount(0);
      await expect(page.getByRole('dialog')).toHaveCount(0);
      await expect(directory(page).locator('.participant-row')).toHaveCount(0);
      expect(state.calls.filter((call) => call.method === 'PUT')).toHaveLength(1);
    } finally {
      held.cleanup();
    }
  });
}
