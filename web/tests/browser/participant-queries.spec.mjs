import { test, expect } from '@playwright/test';
import {
  participantSetup,
  participant,
  participantId,
  directory,
  detail,
  openParticipant,
} from './participant-helpers.mjs';

test('literal name and exact role filters run on the server with exclusive UUID cursors', async ({
  page,
}) => {
  const rows = Array.from({ length: 51 }, (_, index) => ({
    ...participant,
    id: `${(index + 1).toString(16).padStart(8, '0')}-1111-4111-8111-111111111111`,
    display_name: `Persona ${index}`,
  }));
  rows[50].display_name = 'Ana%_ Mu\u00f1oz';
  const state = await participantSetup(page, 'owner', rows);
  await expect(directory(page).locator('.participant-row')).toHaveCount(50);
  await directory(page).getByRole('button', { name: 'Siguiente', exact: true }).click();
  await expect(directory(page).locator('.participant-row')).toHaveCount(1);
  expect(new URLSearchParams(state.calls.at(-1).search).get('after_id')).toBe(rows[49].id);
  await directory(page).getByRole('button', { name: 'Anterior', exact: true }).click();
  await expect(directory(page).locator('.participant-row')).toHaveCount(50);
  await page.getByLabel('Buscar por nombre', { exact: true }).fill('ana%_');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await expect(directory(page).locator('.participant-row')).toHaveCount(0);
  await page.getByLabel('Buscar por nombre', { exact: true }).fill('Ana%_');
  await page.getByLabel('Rol manual exacto', { exact: true }).fill('Defensa');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await expect(directory(page).locator('.participant-row')).toHaveCount(1);
  const query = new URLSearchParams(state.calls.at(-1).search);
  expect(query.get('name')).toBe('Ana%_');
  expect(query.get('procedural_role')).toBe('Defensa');
  expect(query.has('after_id')).toBe(false);
});

test('history uses exclusive revision cursors and retains captured authors', async ({ page }) => {
  const state = await participantSetup(page);
  state.records.set(
    participantId,
    Array.from({ length: 51 }, (_, index) => ({
      ...participant,
      revision: index + 1,
      changed_by: {
        ...participant.changed_by,
        email: index ? 'later@example.com' : 'original@example.com',
      },
    })),
  );
  await openParticipant(page);
  await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  await expect(page.locator('.participant-revision')).toHaveCount(50);
  await page.getByRole('button', { name: 'Cargar cambios anteriores', exact: true }).click();
  await expect(page.locator('.participant-revision')).toHaveCount(51);
  expect(new URLSearchParams(state.calls.at(-1).search).get('before_revision')).toBe('2');
  await page.getByText('Cambio 1', { exact: true }).click();
  await expect(
    page.locator('.participant-revision[open]').getByText('original@example.com', { exact: true }),
  ).toBeVisible();
});

test('mobile directory and editor keep long literal values and controls visible without overlap', async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const row = {
    ...participant,
    display_name: '<script>window.bad=true</script> ' + '\u00c1'.repeat(120),
    procedural_role: 'Rol manual '.repeat(7),
    organization: 'Organizaci\u00f3n '.repeat(10),
  };
  await participantSetup(page, 'owner', [row]);
  await openParticipant(page, row.display_name);
  await expect(
    detail(page).getByRole('heading', { name: row.display_name, exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => window.bad)).toBeUndefined();
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath('participants-mobile.png'), fullPage: true });
  const boxes = await page
    .locator('.participant-filter-fields input, .participant-filter-fields select')
    .evaluateAll((elements) =>
      elements.map((element) => {
        const r = element.getBoundingClientRect();
        return { top: r.top, bottom: r.bottom, width: r.width };
      }),
    );
  expect(boxes.every((box) => box.width > 200)).toBe(true);
  expect(boxes[0].bottom).toBeLessThan(boxes[1].top);
  expect(boxes[1].bottom).toBeLessThan(boxes[2].top);
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Editar participante', exact: true });
  await modal
    .getByRole('button', { name: 'Guardar participante', exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('participants-editor-mobile.png') });
  const actions = await modal.locator('.dialog-actions button').evaluateAll((elements) =>
    elements.map((element) => {
      const r = element.getBoundingClientRect();
      return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };
    }),
  );
  expect(actions[0].right <= actions[1].left || actions[0].bottom <= actions[1].top).toBe(true);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.keyboard.press('Escape');
  await expect(modal).not.toBeVisible();
});
