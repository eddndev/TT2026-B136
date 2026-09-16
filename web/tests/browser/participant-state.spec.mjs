import { test, expect } from '@playwright/test';
import {
  participantSetup,
  participant,
  participantId,
  otherParticipantId,
  directory,
  detail,
  openParticipant,
} from './participant-helpers.mjs';
import { caseId, otherCaseId, navigate } from './helpers.mjs';
const endpoint = `**/cases/${caseId}/participants/${participantId}`;
async function settle(page) {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
}

for (const action of ['filter', 'create']) {
  test(`a late participant detail cannot undo a ${action} and releases the open control`, async ({
    page,
  }) => {
    await participantSetup(page);
    let release;
    await page.route(endpoint, async (route) => {
      await new Promise((resolve) => {
        release = resolve;
      });
      await route.fulfill({ json: participant });
    });
    await directory(page)
      .getByRole('button', { name: `Abrir ${participant.display_name}`, exact: true })
      .click();
    await expect.poll(() => typeof release).toBe('function');
    if (action === 'filter') {
      await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
      await expect(
        directory(page).getByRole('button', {
          name: `Abrir ${participant.display_name}`,
          exact: true,
        }),
      ).toBeEnabled();
    } else {
      await page.getByRole('button', { name: 'Registrar ficha pendiente', exact: true }).click();
      const modal = page.getByRole('dialog', { name: 'Agregar participante', exact: true });
      await modal.getByLabel('Nombre del participante', { exact: true }).fill('Persona nueva');
      await modal.getByLabel('Rol en el expediente', { exact: true }).fill('Testigo');
      await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
      await expect(
        detail(page).getByRole('heading', { name: 'Persona nueva', exact: true }),
      ).toBeVisible();
    }
    const finished = page.waitForEvent('requestfinished', (request) =>
      request.url().endsWith(`/participants/${participantId}`),
    );
    release();
    await finished;
    await settle(page);
    await expect(
      detail(page).getByRole('heading', { name: participant.display_name, exact: true }),
    ).toHaveCount(0);
    await page.unroute(endpoint);
    if (action === 'filter') await openParticipant(page);
    else
      await expect(
        detail(page).getByRole('heading', { name: 'Persona nueva', exact: true }),
      ).toBeVisible();
  });
}

test('a late list cannot replace the row or detail from a confirmed revision', async ({ page }) => {
  await participantSetup(page);
  await openParticipant(page);
  let release,
    first = true;
  await page.route('**/participants?*', async (route) => {
    if (!first) return route.fallback();
    first = false;
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({
      json: { participants: [participant], has_more: false, next_after_id: null },
    });
  });
  await directory(page).getByRole('button', { name: 'Actualizar', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Editar participante', exact: true });
  await modal.getByLabel('Nombre del participante', { exact: true }).fill('Nombre confirmado');
  await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(
    detail(page).getByRole('heading', { name: 'Nombre confirmado', exact: true }),
  ).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().includes('/participants?'),
  );
  release();
  await finished;
  await settle(page);
  await expect(
    directory(page).getByRole('button', { name: 'Abrir Nombre confirmado', exact: true }),
  ).toBeVisible();
});

for (const deniedPart of ['detail', 'history', 'edit']) {
  test(`${deniedPart} access denial clears participant data and any pending views`, async ({
    page,
  }) => {
    await participantSetup(page);
    await openParticipant(page);
    if (deniedPart === 'history') {
      await page.route('**/participants/*/history?*', (route) =>
        route.fulfill({ status: 403, json: { error: { code: 'forbidden' } } }),
      );
      await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
    } else {
      await page.route(endpoint, (route) =>
        route.fulfill({ status: 404, json: { error: { code: 'participant_not_found' } } }),
      );
      if (deniedPart === 'detail')
        await directory(page)
          .getByRole('button', { name: `Abrir ${participant.display_name}`, exact: true })
          .click();
      else {
        await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
        await page
          .getByRole('dialog')
          .getByRole('button', { name: 'Guardar participante', exact: true })
          .click();
      }
    }
    await expect(page.getByRole('alert')).toBeVisible();
    await expect(detail(page)).toHaveCount(0);
    await expect(directory(page).locator('.participant-row')).toHaveCount(0);
    await expect(page.getByRole('dialog')).toHaveCount(0);
  });
}

test('history from an abandoned case cannot enter another case or show an old error', async ({
  page,
}) => {
  await participantSetup(page);
  await openParticipant(page);
  let release;
  await page.route('**/participants/*/history?*', async (route) => {
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({ status: 403 });
  });
  await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.route(`**/cases/${otherCaseId}/participants**`, (route) =>
    route.fulfill({
      json: {
        participants: [
          {
            ...participant,
            id: otherParticipantId,
            case_id: otherCaseId,
            display_name: 'Otra persona',
          },
        ],
        has_more: false,
        next_after_id: null,
      },
    }),
  );
  await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
  await page.getByRole('button', { name: /Otro expediente/ }).click();
  await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  await expect(
    directory(page).getByRole('button', { name: 'Abrir Otra persona', exact: true }),
  ).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().includes('/history?'),
  );
  release();
  await finished;
  await settle(page);
  await expect(page.getByRole('alert')).toHaveCount(0);
  await expect(page.getByText(participant.display_name, { exact: true })).toHaveCount(0);
});

test('a pending mutation is discarded when navigation destroys the participant workspace', async ({
  page,
}) => {
  await participantSetup(page);
  await openParticipant(page);
  let release;
  await page.route(endpoint, async (route) => {
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({ json: { ...participant, revision: 2, display_name: 'Respuesta vieja' } });
  });
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Guardar participante', exact: true })
    .click();
  await expect.poll(() => typeof release).toBe('function');
  await page.evaluate(() => {
    location.hash = 'overview';
  });
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) => request.method() === 'PUT');
  release();
  await finished;
  await settle(page);
  await expect(page.getByText('Respuesta vieja', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Participante guardado.', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('dialog')).toHaveCount(0);
});
