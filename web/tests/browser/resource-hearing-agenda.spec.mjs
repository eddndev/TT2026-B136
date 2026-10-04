import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { agendaPage, cursorFor } from './combined-agenda-helpers.mjs';
import {
  ownCard,
  ownDetail,
  setupResourceHearingAgenda,
  openResourceHearingAgenda,
} from './resource-hearing-agenda-helpers.mjs';

for (const width of [1440, 390])
  test(`resource hearing opens exact captured sources inside agenda at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupResourceHearingAgenda(page, { closed: true });
    await openResourceHearingAgenda(page, state);
    expect(
      await page
        .locator('[data-agenda-kind]')
        .evaluateAll((rows) => rows.map((row) => row.dataset.agendaKind)),
    ).toEqual(['hearing', 'deadline', 'resource_hearing']);
    await ownCard(page, state).click();
    const detail = ownDetail(page),
      hearing = state.creation.hearing;
    await expect(detail).toBeVisible();
    expect(state.order).toEqual(['administration', 'exact']);
    expect(state.detailCalls).toHaveLength(1);
    expect(state.detailCalls[0].method()).toBe('GET');
    await expect(detail).toContainText(/hist[o\u00f3]rica/i);
    await expect(detail).toContainText(hearing.values.venue);
    await expect(detail).toContainText(hearing.values.scheduling_basis.statement);
    await expect(detail).toContainText('18:00:00');
    await expect(detail).toContainText('-06:00');
    await expect(detail).toContainText(hearing.sources.support.name);
    await expect(detail).toContainText(hearing.sources.resource.values.title);
    await expect(detail).toContainText(hearing.sources.act.act.values.statement);
    for (const person of hearing.sources.participants)
      await expect(detail).toContainText(person.display_name);
    await detail.getByText('Autor y recibo original', { exact: true }).click();
    await expect(detail).toContainText(hearing.recorded_by.email);
    await expect(detail).toContainText(state.creation.origin.association_id);
    await expect(detail).toContainText(state.creation.origin.capture_digest);
    await expect(
      detail.getByRole('button', { name: 'Cancelar audiencia', exact: true }),
    ).toHaveCount(0);
    await expect(
      page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
    ).toHaveCount(0);
    await expect(page.getByRole('region', { name: 'Agenda combinada', exact: true })).toBeVisible();
    const images = new URL('../../../output/resource-hearings/ui/', import.meta.url);
    await mkdir(images, { recursive: true });
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: fileURLToPath(new URL(`resource-hearing-agenda-${width}.png`, images)),
      fullPage: true,
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await detail.getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true }).click();
    await expect(detail).toHaveCount(0);
    await expect(page.getByLabel('Fecha de referencia', { exact: true })).toHaveValue('2026-01-02');
    await expect(page.locator('[data-agenda-kind]')).toHaveCount(3);
  });

test('resource hearing filter clears cancelled while mixed cancelled excludes its rows', async ({
  page,
}) => {
  const state = await setupResourceHearingAgenda(page);
  await openResourceHearingAgenda(page, state);
  const kind = page.getByRole('combobox', { name: 'Tipo de actividad', exact: true });
  const status = page.getByRole('combobox', { name: 'Estado de audiencia', exact: true });
  await status.selectOption('cancelled');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  await expect(ownCard(page, state)).toHaveCount(0);
  await expect(page.locator('[data-agenda-kind="deadline"]')).toHaveCount(1);
  await kind.selectOption('resource_hearing');
  await expect(status).toHaveValue('scheduled');
  await expect(status.locator('option[value="cancelled"]')).toHaveCount(0);
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  await expect(ownCard(page, state)).toBeVisible();
  await expect(page.locator('[data-agenda-kind]')).toHaveCount(1);
  expect(state.calls.at(-1).searchParams.get('kind')).toBe('resource_hearing');
  expect(state.calls.at(-1).searchParams.get('hearing_status')).toBe('scheduled');
});

test('opening and closing own detail preserves an incomplete page and its continuation', async ({
  page,
}) => {
  const state = await setupResourceHearingAgenda(page);
  state.handle = async (route, url) => {
    if (!url.searchParams.get('from')?.startsWith('2026-01-02')) return false;
    const next = cursorFor(url, state.own.at.unix_seconds, 0, 2, state.creation.hearing.id);
    await route.fulfill({
      json: url.searchParams.has('cursor')
        ? agendaPage(url, [])
        : agendaPage(url, state.rows, false, next),
    });
    return true;
  };
  await openResourceHearingAgenda(page, state);
  const count = state.calls.length;
  await ownCard(page, state).click();
  await expect(ownDetail(page)).toBeVisible();
  await ownDetail(page)
    .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
    .click();
  expect(state.calls).toHaveLength(count);
  await expect(page.locator('[data-agenda-kind]')).toHaveCount(3);
  await page.getByRole('button', { name: 'Cargar m\u00e1s actividades', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Cargar m\u00e1s actividades', exact: true }),
  ).toHaveCount(0);
  expect(state.calls.at(-1).searchParams.get('cursor')).toContain(
    `:2:${state.creation.hearing.id}`,
  );
  await expect(page.locator('[data-agenda-kind]')).toHaveCount(3);
});

test('revoked case access removes every family without requesting its own detail', async ({
  page,
}) => {
  const state = await setupResourceHearingAgenda(page);
  await openResourceHearingAgenda(page, state);
  state.denied = true;
  await ownCard(page, state).click();
  await expect(
    page.getByRole('region', { name: 'Agenda combinada', exact: true }).getByRole('alert'),
  ).toBeVisible();
  expect(state.order).toEqual(['administration']);
  expect(state.detailCalls).toHaveLength(0);
  await expect(page.locator('[data-agenda-kind]')).toHaveCount(0);
  await expect(ownDetail(page)).toHaveCount(0);
});
