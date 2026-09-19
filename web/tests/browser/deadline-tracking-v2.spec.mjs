import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  editor,
  fillDeadline,
  confirmDeadline,
  deadlineError,
  caseId,
} from './deadline-editor-helpers.mjs';
import { overview, otherAdministration } from './case-administration-helpers.mjs';
import {
  v2Record,
  timedPrepared,
  technicalRecord,
  ids,
  instant,
  notChecked,
} from '../fixtures/deadline-v2-unit.mjs';

const panel = (page) => page.getByRole('region', { name: 'Detalle de plazo', exact: true });
const policy = (page) =>
  editor(page).getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true });
const prepareCalls = (state) => state.calls.filter((call) => call.path.endsWith('/prepare'));
const clone = (value) => structuredClone(value);
const inCase = (value) => JSON.parse(JSON.stringify(value).replaceAll(ids(1), caseId));
const open = (page, row) =>
  page.getByRole('button', { name: `Consultar plazo ${row.id}`, exact: true }).click();
async function begin(page, title = 'Revision explicita') {
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fillDeadline(page, title);
}
function stateExamples() {
  const current = inCase(v2Record(timedPrepared()));
  current.id = ids(20);
  current.definition.title = 'Plazo vigente consultado';
  current.operational = {
    freshness: 'current',
    checked_at: instant(),
    changed_dependencies: [],
    due_at: clone(current.calculation.result.due_at),
  };
  const changed = clone(current);
  changed.id = ids(21);
  changed.definition.title = 'Dependencia con cambios';
  changed.operational = {
    freshness: 'changed',
    checked_at: instant(),
    changed_dependencies: ['source'],
    due_at: null,
  };
  const predecessor = clone(current);
  predecessor.id = ids(22);
  predecessor.definition.title = 'Plazo con revision tecnica';
  predecessor.receipt.version = { kind: 'v1' };
  predecessor.tracking = null;
  predecessor.operational = notChecked();
  const pending = clone(current);
  pending.id = predecessor.id;
  pending.definition.title = predecessor.definition.title;
  pending.revision = 2;
  pending.reason = 'Observed profile change';
  const technical = inCase(technicalRecord());
  pending.receipt = clone(technical.receipt);
  pending.recorded_by = clone(technical.recorded_by);
  pending.tracking.review = clone(technical.tracking.review);
  pending.tracking.observations.entries[0] = clone(technical.tracking.observations.entries[0]);
  pending.operational = { ...clone(current.operational), due_at: null };
  const legacy = clone(predecessor);
  legacy.id = ids(23);
  legacy.definition.title = 'Plazo anterior sin seguimiento';
  return { current, changed, predecessor, pending, legacy };
}

// The harness must return V2; an id-only principal cannot reach prepare successfully.
for (const role of ['owner', 'litigator'])
  test(`explicit policy and complete principal flow for ${role}`, async ({ page }) => {
    const state = await setupDeadlines(page, { role });
    await openDeadlines(page);
    await begin(page);
    await expect(policy(page)).toHaveValue('');
    await editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }).click();
    await expect(editor(page).getByRole('alert')).toBeVisible();
    expect(prepareCalls(state)).toHaveLength(0);
    await policy(page).selectOption('follow');
    await confirmDeadline(page);
    await expect(editor(page)).toHaveCount(0);
    expect(state.submissions).toHaveLength(1);
    const command = state.submissions[0];
    expect(command.change.tracking).toEqual({
      profile: 'follow',
      source: 'undetermined',
      calendar: 'undetermined',
    });
    expect(Object.keys(command).sort()).toEqual(['change', 'deadline_id', 'operation_id']);
    expect(state.records.get(command.deadline_id).at(-1).recorded_by).toEqual({
      kind: 'user',
      id: ids(4),
      email: 'staff@example.test',
    });
  });

for (const width of [1440, 390])
  test(`current changed pending and legacy stay distinct at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    const rows = stateExamples();
    const state = await setupDeadlines(page, { deadlines: Object.values(rows) });
    await openDeadlines(page);
    const due = /2026-01-02 00:00:00/;
    await expect(
      page
        .getByRole('button', { name: `Consultar plazo ${rows.current.id}`, exact: true })
        .locator('.deadline-operational'),
    ).toContainText(due);
    for (const name of ['changed', 'pending', 'legacy'])
      await expect(
        page
          .getByRole('button', { name: `Consultar plazo ${rows[name].id}`, exact: true })
          .locator('.deadline-operational'),
      ).not.toContainText(due);
    for (const [name, label] of [
      ['current', 'Revisi\u00f3n aceptada'],
      ['changed', 'Revisi\u00f3n aceptada'],
      ['pending', 'Revisi\u00f3n pendiente'],
      ['legacy', 'Seguimiento sin declarar'],
    ]) {
      await open(page, rows[name]);
      const following = panel(page).getByRole('region', {
        name: 'Seguimiento del plazo',
        exact: true,
      });
      await expect(following).toContainText(label);
      if (name === 'current') await expect(following).toContainText(due);
      else await expect(following).not.toContainText(due);
      await expect(
        panel(page).getByRole('region', { name: 'Resultado del plazo', exact: true }),
      ).toContainText(due);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
    }
    await open(page, rows.pending);
    await panel(page).getByRole('button', { name: 'Ver historial de plazo', exact: true }).click();
    const history = page.getByRole('region', { name: 'Historial de plazo', exact: true });
    await expect(history).toContainText('Servicio de reevaluaci\u00f3n');
    await expect(history).toContainText('staff@example.test');
    await history
      .getByRole('button', { name: 'Consultar plazo revisi\u00f3n 1', exact: true })
      .click();
    await expect(panel(page)).toContainText('Consultada exactamente');
    await expect(
      panel(page).getByRole('region', { name: 'Seguimiento del plazo', exact: true }),
    ).not.toContainText(due);
    await expect(
      panel(page).getByRole('button', { name: 'Corregir plazo', exact: true }),
    ).toHaveCount(0);
    expect(state.submissions).toHaveLength(0);
  });

for (const destination of ['case', 'session'])
  test(`late prepared policies and receipt are discarded after changing ${destination}`, async ({
    page,
  }) => {
    const state = await setupDeadlines(page);
    await page.route('**/api/v1/case-administrations?*', (route) =>
      route.fulfill({
        json: {
          cases: [overview(state.facts.results.scheduling.admin), overview(otherAdministration())],
          has_more: false,
          next_after_id: null,
        },
      }),
    );
    let release;
    const held = new Promise((resolve) => {
      release = resolve;
    });
    let started = false,
      finished = false;
    state.handle = async (route, call) => {
      if (!call.path.endsWith('/prepare')) return;
      const result = state.prepare(call.body);
      started = true;
      await held;
      await route.fulfill({ json: result });
      finished = true;
      return true;
    };
    await openDeadlines(page);
    await begin(page, 'Borrador privado anterior');
    await policy(page).selectOption('fixed');
    await editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }).click();
    await expect.poll(() => started).toBe(true);
    if (destination === 'case') {
      await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
      await page.getByRole('button', { name: /Otro expediente/ }).click();
    } else await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
    release();
    await expect.poll(() => finished).toBe(true);
    await expect(editor(page)).toHaveCount(0);
    await expect(page.getByText('Borrador privado anterior', { exact: true })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Confirmar plazo', exact: true })).toHaveCount(0);
    expect(state.submissions).toHaveLength(0);
  });

test('conflicting correction retains the selected policy until an explicit base comparison', async ({
  page,
}) => {
  const initial = stateExamples().current;
  initial.operational = notChecked();
  const state = await setupDeadlines(page, { deadlines: [initial] });
  await openDeadlines(page);
  await open(page, initial);
  await panel(page).getByRole('button', { name: 'Corregir plazo', exact: true }).click();
  await policy(page).selectOption('fixed');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Revisar seguimiento');
  let rejected = false;
  state.handle = async (route, call) => {
    if (rejected || !call.path.endsWith('/prepare')) return;
    rejected = true;
    const concurrent = clone(call.body);
    concurrent.operation_id = ids(71);
    concurrent.change.tracking.profile = 'follow';
    state.commit(state.prepare(concurrent));
    await deadlineError(route, 'deadline_revision_conflict');
    return true;
  };
  await editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }).click();
  await expect(
    editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }),
  ).toBeDisabled();
  await expect(policy(page)).toHaveValue('fixed');
  expect(state.submissions).toHaveLength(0);
  await editor(page).getByRole('button', { name: 'Consultar base actual', exact: true }).click();
  await editor(page)
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await expect(policy(page)).toHaveValue('fixed');
  await confirmDeadline(page);
  await expect(editor(page)).toHaveCount(0);
  expect(state.submissions[0].change.expected_revision).toBe(2);
  expect(state.submissions[0].change.tracking.profile).toBe('fixed');
});

test('uncertain V2 submission reads its exact receipt once without an automatic retry', async ({
  page,
}) => {
  const state = await setupDeadlines(page);
  state.handle = async (route, call) => {
    if (call.method !== 'POST' || call.path.endsWith('/prepare')) return;
    state.submissions.push(call.body.command);
    state.commit(state.prepare(call.body.command));
    await route.abort('failed');
    return true;
  };
  await openDeadlines(page);
  await begin(page);
  await policy(page).selectOption('follow');
  await confirmDeadline(page);
  await expect(
    editor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  await editor(page).getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(
    state.calls.filter(
      (call) => call.path.includes('/deadlines/') && call.path.endsWith('/revisions/1'),
    ),
  ).toHaveLength(1);
});
