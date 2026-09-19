import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import {
  captureReevaluation,
  historical,
  withReevaluationApi,
} from './deadline-reevaluation-helpers.mjs';
import {
  advanceFollowSource,
  waitForDeadlineRevision,
} from '../../../scripts/web-deadline-reevaluation.mjs';

const enabled = process.env.TT_DEADLINE_REEVALUATION_ACCEPTANCE === '1';
if (enabled && !fixture.combinedAgenda)
  throw new Error('Enable combined agenda fixtures before selecting this live spec');
if (enabled && !fixture.combinedAgenda.fiveCases)
  throw new Error('Enable five-case combined agenda fixtures before selecting this live spec');
test.skip(!enabled, 'Requires disposable Follow acceptance fixtures');

const agenda = (page) => page.getByRole('region', { name: 'Agenda combinada', exact: true });
const activity = (page, kind, id) =>
  page.getByRole('button', {
    name: `Consultar ${kind === 'hearing' ? 'audiencia' : 'plazo'} ${id}`,
    exact: true,
  });

async function query(page, view, date) {
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption(view);
  await page.getByLabel('Fecha de referencia', { exact: true }).fill(date);
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('+00:00');
  const response = page.waitForResponse(
    (result) =>
      new URL(result.url()).pathname === '/api/v1/agenda' && result.request().method() === 'GET',
  );
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  const result = await response;
  expect(result.status()).toBe(200);
  await expect(agenda(page)).toHaveAttribute('aria-busy', 'false');
  await expect(agenda(page).getByRole('alert')).toHaveCount(0);
  return result.json();
}

async function openExact(page, scenario, kind) {
  const record = scenario[kind];
  const family = kind === 'hearing' ? 'hearings' : 'deadlines';
  const path = `/api/v1/cases/${scenario.case.id}/${family}/${record.id}/revisions/1`;
  const response = page.waitForResponse(
    (result) => new URL(result.url()).pathname === path && result.request().method() === 'GET',
  );
  await activity(page, kind, record.id).click();
  const result = await response;
  expect(result.status()).toBe(200);
  const exact = await result.json();
  expect(exact).toEqual(kind === 'deadline' ? historical(record) : record);
  const label = kind === 'hearing' ? 'Detalle de audiencia' : 'Detalle de plazo';
  await expect(page.getByRole('region', { name: label, exact: true })).toContainText(
    /consultada exactamente/i,
  );
  if (kind === 'deadline') {
    expect(exact.operational.due_at).toBeNull();
    expect(exact.calculation.result.due_at).toEqual(scenario.dueAt);
  }
  await page.getByRole('button', { name: 'Ir a Agenda', exact: true }).click();
  await expect(agenda(page)).toHaveAttribute('aria-busy', 'false');
  await expect(page.getByRole('combobox', { name: 'Vista de agenda', exact: true })).toHaveValue(
    'month',
  );
}

for (const [name, width] of [
  ['desktop', 1440],
  ['mobile', 390],
])
  test(`real combined agenda preserves exact history and excludes pending deadlines at ${width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(180000);
    const scenario = fixture.combinedAgenda[name];
    const errors = [],
      paths = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', (request) => paths.push(new URL(request.url()).pathname));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await loginAs(page, scenario.operator, 0);
    paths.length = 0;
    await navigate(page, 'Agenda');
    await expect(agenda(page)).toHaveAttribute('aria-busy', 'false');
    for (const [view, label] of [
      ['day', 'Vista diaria'],
      ['week', 'Vista semanal'],
      ['month', 'Vista mensual'],
    ]) {
      const result = await query(page, view, scenario.date);
      expect(result.complete).toBe(true);
      expect(result.next_cursor).toBeNull();
      expect(result.items.map((row) => row.kind)).toEqual(['hearing', 'deadline']);
      expect(result.items[0].hearing).toMatchObject({
        id: scenario.hearing.id,
        case_id: scenario.case.id,
        revision: 1,
      });
      expect(result.items[1].deadline).toMatchObject({
        id: scenario.deadline.id,
        case_id: scenario.case.id,
        revision: 1,
        receipt_kind: 'v2',
        review_state: 'accepted',
        operational: {
          freshness: 'current',
          due_at: scenario.dueAt,
          checked_at: result.checked_at,
        },
      });
      expect(result.items.map((row) => row.at)).toEqual([scenario.dueAt, scenario.dueAt]);
      const region = page.getByRole('region', { name: label, exact: true });
      await expect(region).toBeVisible();
      await expect(activity(page, 'hearing', scenario.hearing.id)).toBeVisible();
      await expect(activity(page, 'deadline', scenario.deadline.id)).toBeVisible();
      expect(
        await region
          .locator('[data-agenda-kind]')
          .evaluateAll((rows) => rows.map((row) => row.dataset.agendaKind)),
      ).toEqual(['hearing', 'deadline']);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
    }
    expect(
      paths.filter((path) => path === '/api/v1/cases' || path === '/api/v1/case-administrations'),
    ).toEqual([]);
    await captureReevaluation(page, testInfo, `combined-month-${name}`);
    await openExact(page, scenario, 'hearing');
    await openExact(page, scenario, 'deadline');

    await withReevaluationApi(scenario.owner, async (call) => {
      await advanceFollowSource(call, scenario.case.id, scenario.source);
      const pending = await waitForDeadlineRevision(
        call,
        scenario.case.id,
        scenario.deadline.id,
        2,
      );
      expect(pending.tracking.review.state).toBe('pending');
      expect(pending.operational.due_at).toBeNull();
      expect(pending.calculation).toEqual(scenario.deadline.calculation);
      for (const view of ['day', 'week', 'month']) {
        const result = await query(page, view, scenario.date);
        expect(result.complete).toBe(true);
        expect(result.items.map((row) => [row.kind, row.hearing?.id])).toEqual([
          ['hearing', scenario.hearing.id],
        ]);
        await expect(activity(page, 'hearing', scenario.hearing.id)).toBeVisible();
        await expect(activity(page, 'deadline', scenario.deadline.id)).toHaveCount(0);
      }
      const path = `/cases/${scenario.case.id}/deadlines/${scenario.deadline.id}`;
      expect(await call('GET', `${path}/revisions/1`)).toEqual(historical(scenario.deadline));
      expect((await call('GET', '/audit/verify')).valid).toBe(true);
    });
    await captureReevaluation(page, testInfo, `combined-pending-${name}`);
    expect(errors).toEqual([]);
  });

for (const [name, width, recoveryIndex] of [
  ['desktop', 1440, 1],
  ['mobile', 390, 2],
])
  test(`real combined agenda queries five assigned cases and excludes an unassigned sixth at ${width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(180000);
    const scenario = fixture.combinedAgenda.fiveCases;
    const expectedCases = scenario.visible.map((entry) => entry.case.id).sort();
    const expectedRows = scenario.visible
      .map((entry) => `${entry.kind}:${entry[entry.kind].id}`)
      .sort((left, right) => {
        const rank = (value) => (value.startsWith('hearing:') ? 0 : 1);
        return rank(left) - rank(right) || (left < right ? -1 : left > right ? 1 : 0);
      });
    const at = scenario.visible.find((entry) => entry.kind === 'deadline').dueAt;
    const errors = [],
      paths = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', (request) => {
      const path = new URL(request.url()).pathname;
      if (path.startsWith('/api/v1/')) paths.push(path);
    });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await loginAs(page, scenario.operator, recoveryIndex);
    paths.length = 0;
    await navigate(page, 'Agenda');
    await expect(agenda(page)).toHaveAttribute('aria-busy', 'false');
    for (const view of ['day', 'week', 'month']) {
      const before = paths.length;
      const result = await query(page, view, scenario.date);
      expect(paths.slice(before)).toEqual(['/api/v1/agenda']);
      expect(result.complete).toBe(true);
      expect(result.next_cursor).toBeNull();
      expect(result.items).toHaveLength(5);
      expect(result.items.map((row) => row[row.kind].case_id).sort()).toEqual(expectedCases);
      expect(new Set(expectedCases).size).toBe(5);
      expect(result.items.map((row) => `${row.kind}:${row[row.kind].id}`)).toEqual(expectedRows);
      expect(result.items.map((row) => row.at)).toEqual(Array(5).fill(at));
      expect(result.items.filter((row) => row.kind === 'hearing')).toHaveLength(3);
      expect(result.items.filter((row) => row.kind === 'deadline')).toHaveLength(2);
      for (const row of result.items) {
        expect(row[row.kind].revision).toBe(1);
        if (row.kind === 'deadline') {
          expect(row.deadline.review_state).toBe('accepted');
          expect(row.deadline.operational).toMatchObject({
            freshness: 'current',
            due_at: at,
            checked_at: result.checked_at,
          });
        }
      }
      for (const entry of scenario.visible)
        await expect(activity(page, entry.kind, entry[entry.kind].id)).toBeVisible();
      expect(JSON.stringify(result)).not.toContain(scenario.hidden.case.id);
      expect(JSON.stringify(result)).not.toContain(scenario.hidden.hearing.id);
      await expect(activity(page, 'hearing', scenario.hidden.hearing.id)).toHaveCount(0);
    }
    expect(paths.every((path) => path === '/api/v1/agenda')).toBe(true);
    await captureReevaluation(page, testInfo, `combined-five-cases-${name}`);
    for (const entry of scenario.visible) await openExact(page, entry, entry.kind);
    expect(errors).toEqual([]);
  });
