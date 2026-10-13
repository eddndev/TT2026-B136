import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { queryAgenda } from './combined-agenda-helpers.mjs';
import {
  ownCard,
  ownDetail,
  setupResourceHearingAgenda,
  openResourceHearingAgenda,
  rendered,
} from './resource-hearing-agenda-helpers.mjs';

for (const action of ['refresh', 'filter', 'navigate', 'logout'])
  test(`a late resource hearing detail cannot reappear after ${action}`, async ({ page }) => {
    const state = await setupResourceHearingAgenda(page);
    let release;
    const gate = new Promise((resolve) => {
      release = resolve;
    });
    state.onExact = async (route) => {
      await gate;
      await route.fulfill({ json: state.creation });
    };
    try {
      await openResourceHearingAgenda(page, state);
      await ownCard(page, state).click();
      await expect.poll(() => state.detailCalls.length).toBe(1);
      if (action === 'refresh') {
        const count = state.calls.length;
        await page.getByRole('button', { name: 'Actualizar Agenda', exact: true }).click();
        await expect.poll(() => state.calls.length).toBe(count + 1);
      } else if (action === 'filter') {
        await queryAgenda(page, 'day', '2026-01-05');
        await expect(
          page.getByText('No hay actividades en esta consulta.', { exact: true }),
        ).toBeVisible();
      } else if (action === 'navigate') {
        await navigate(page, 'Inicio');
        await expect(
          page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
        ).toBeVisible();
      } else {
        await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
        await login(page, false, false);
      }
      const finished = page.waitForEvent(
        'requestfinished',
        (request) => new URL(request.url()).pathname === state.exactPath,
      );
      release();
      await finished;
      await rendered(page);
      await expect(ownDetail(page)).toHaveCount(0);
      await expect(
        page.getByText(state.creation.hearing.values.scheduling_basis.statement, { exact: true }),
      ).toHaveCount(0);
      expect(state.detailCalls).toHaveLength(1);
    } finally {
      release();
    }
  });
