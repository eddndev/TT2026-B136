import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  deadlineFixture,
  caseId,
} from './deadline-editor-helpers.mjs';
import { id, known } from '../fixtures/deadline-unit.mjs';
import { v1Record as detail } from '../fixtures/deadline-v2-unit.mjs';
import {
  factRecord,
  factPrepared,
  factCommand,
  factResolutionSource,
} from '../fixtures/procedural-facts.mjs';
import { resultRecord } from '../fixtures/hearing-results.mjs';
const list = (page) => page.getByRole('region', { name: 'Plazos registrados', exact: true });
const panel = (page) => page.getByRole('region', { name: 'Detalle de plazo', exact: true });
const open = (page, row) =>
  list(page)
    .getByRole('button', { name: `Consultar plazo ${row.id}`, exact: true })
    .click();

test('deadline pages retain applied status until explicit filtering and advance by UUID', async ({
  page,
}) => {
  const rows = Array.from({ length: 21 }, (_, i) => {
    const value = deadlineFixture();
    value.command.deadline_id = id(100 + i);
    value.definition.title = `Plazo activo ${i + 1}`;
    return detail(value);
  });
  const retired = deadlineFixture('retire');
  retired.command.deadline_id = id(121);
  retired.definition.title = 'Plazo retirado';
  rows.push(detail(retired));
  const state = await setupDeadlines(page, { deadlines: rows });
  await openDeadlines(page);
  await expect(list(page).getByRole('button', { name: /^Consultar plazo / })).toHaveCount(20);
  await expect(list(page)).not.toContainText(rows[0].definition.input.qualification.statement);
  expect(state.calls.filter((c) => c.path.includes('/deadline-profiles'))).toHaveLength(0);
  await page
    .getByRole('combobox', { name: 'Estado del plazo', exact: true })
    .selectOption('retired');
  await page.getByRole('button', { name: 'Plazos siguientes', exact: true }).click();
  await expect(list(page)).toContainText('Plazo activo 21');
  expect(state.calls.at(-1).search).toContain('status=active');
  expect(state.calls.at(-1).search).toContain(`after_id=${rows[19].id}`);
  await page.getByRole('button', { name: 'Aplicar estado', exact: true }).click();
  await expect(list(page).getByRole('button', { name: /^Consultar plazo / })).toHaveCount(1);
  await expect(list(page)).toContainText('Plazo retirado');
  expect(state.calls.at(-1).search).toContain('status=retired');
  expect(state.calls.at(-1).search).not.toContain('after_id');
  await page.getByRole('combobox', { name: 'Estado del plazo', exact: true }).selectOption('all');
  await page.getByRole('button', { name: 'Aplicar estado', exact: true }).click();
  await expect(list(page).getByRole('button', { name: /^Consultar plazo / })).toHaveCount(20);
  await page.getByRole('button', { name: 'Plazos siguientes', exact: true }).click();
  await expect(list(page).getByRole('button', { name: /^Consultar plazo / })).toHaveCount(2);
  expect(state.calls.at(-1).search).toContain('status=all');
  await page.getByRole('button', { name: 'Plazos anteriores', exact: true }).click();
  await expect(list(page)).toContainText('Plazo activo 1');
  expect(state.calls.at(-1).search).not.toContain('after_id');
});
test('deadline history is light, paged and loads immutable calculation only on exact selection', async ({
  page,
}) => {
  const rows = Array.from({ length: 11 }, (_, i) => {
    const p = deadlineFixture(i ? 'correct' : 'register');
    p.command.operation_id = id(200 + i);
    p.command.change.expected_revision = i;
    p.result_revision = i + 1;
    p.definition.title = `Contenido privado revision ${i + 1}`;
    p.command.change.definition = structuredClone(p.definition);
    return detail(p);
  });
  const state = await setupDeadlines(page, { deadlines: rows });
  await openDeadlines(page);
  await open(page, rows[0]);
  await expect(panel(page)).toContainText(rows[10].definition.title);
  await panel(page).getByRole('button', { name: 'Ver historial de plazo', exact: true }).click();
  const history = page.getByRole('region', { name: 'Historial de plazo', exact: true });
  await expect(history.getByRole('button', { name: /^Consultar plazo revisi/ })).toHaveCount(10);
  await expect(history).not.toContainText('Contenido privado');
  expect(state.calls.filter((c) => c.path.includes('/revisions/'))).toHaveLength(0);
  await history.getByRole('button', { name: 'Cargar cambios anteriores', exact: true }).click();
  await expect(history.getByRole('button', { name: /^Consultar plazo revisi/ })).toHaveCount(11);
  expect(state.calls.at(-1).search).toContain('before_revision=2');
  await history
    .getByRole('button', { name: 'Consultar plazo revisi\u00f3n 1', exact: true })
    .click();
  await expect(panel(page)).toContainText(rows[0].definition.title);
  await expect(panel(page)).toContainText('Consultada exactamente');
  await expect(
    panel(page).getByRole('button', { name: 'Corregir plazo', exact: true }),
  ).toHaveCount(0);
  expect(state.calls.at(-1).path).toContain(`/deadlines/${rows[0].id}/revisions/1`);
  await panel(page).getByRole('button', { name: 'Consultar plazo actual', exact: true }).click();
  await expect(panel(page)).toContainText(rows[10].definition.title);
});
test('captured profile opens its exact corpus without expanding it in the collection', async ({
  page,
}) => {
  const row = detail(deadlineFixture()),
    state = await setupDeadlines(page, { deadlines: [row] });
  await openDeadlines(page);
  await open(page, row);
  expect(state.calls.filter((c) => c.path.includes('/deadline-profiles'))).toHaveLength(0);
  await panel(page)
    .getByRole('button', { name: 'Consultar perfil capturado', exact: true })
    .click();
  const profile = page.getByRole('region', { name: 'Perfil exacto del plazo', exact: true });
  await expect(profile).toContainText('Perfil de prueba');
  await expect(profile).toContainText('Supuesto declarado');
  await expect(profile.getByRole('link')).toHaveAttribute('href', 'https://example.test/fixture');
  expect(state.calls.at(-1).path).toBe(
    `/api/v1/cases/${caseId}/deadline-profiles/${id(2)}/revisions/1`,
  );
});
for (const family of ['resolution', 'notification', 'hearing_result'])
  test(`captured ${family} opens the selected immutable source`, async ({ page }) => {
    const resolution = factRecord(),
      notification = factRecord(factPrepared(factCommand('notification'))),
      hearing = resultRecord();
    notification.sources.resolution = factResolutionSource(resolution);
    const source =
      family === 'resolution' ? resolution : family === 'notification' ? notification : hearing;
    const p = deadlineFixture(),
      ref =
        family === 'hearing_result'
          ? {
              family,
              hearing_id: source.hearing_id,
              result_id: source.id,
              revision: 1,
              agreement_id: null,
            }
          : {
              family,
              id: source.id,
              revision: 1,
              ...(family === 'notification' ? { resolution: source.values.resolution } : {}),
            };
    p.definition.input.selection.source = known(ref);
    p.command.change.definition = structuredClone(p.definition);
    const href =
      family === 'hearing_result'
        ? `/api/v1/cases/${caseId}/hearings/${source.hearing_id}/results/${source.id}/revisions/1`
        : family === 'notification'
          ? `/api/v1/cases/${caseId}/resolutions/${resolution.id}/notifications/${source.id}/revisions/1`
          : `/api/v1/cases/${caseId}/resolutions/${source.id}/revisions/1`;
    const captured = {
      case_id: caseId,
      reference: ref,
      values_digest: source.values_digest,
      sources_digest: source.receipt.sources_digest ?? null,
      submission_digest: source.receipt.submission_digest,
      status: source.status,
      href,
    };
    p.calculation.material.source = captured;
    p.calculation.material.source_head = structuredClone(captured);
    const at =
      family === 'hearing_result'
        ? { precision: 'date', year: 2026, month: 9, day: 1, offset_seconds: -21600 }
        : { precision: 'unknown' };
    const block =
      family === 'hearing_result'
        ? { kind: 'insufficient_precision', observed: 'date' }
        : { kind: 'unknown_anchor' };
    const rule = { kind: 'elapsed_hours', quantity: 24 };
    p.calculation.result = {
      requirement: {
        kind: 'source_field',
        field: {
          resolution: 'resolution_issued_at',
          notification: 'notification_practiced_at',
          hearing_result: 'hearing_session_event_time',
        }[family],
      },
      trigger_outcome: { kind: 'extracted', at },
      rule,
      arithmetic: { rule, anchor: at, outcome: { kind: 'blocked', block }, trace: [] },
      due_at: null,
      blocks: [{ kind: 'arithmetic', block }],
    };
    const row = detail(p),
      state = await setupDeadlines(page, {
        deadlines: [row],
        facts: [resolution, notification],
        results: [hearing],
      });
    await openDeadlines(page);
    await open(page, row);
    await panel(page)
      .getByText('Consultar insumos y fuentes de esta revisi\u00f3n', { exact: true })
      .click();
    await panel(page)
      .getByRole('button', { name: 'Consultar fuente seleccionada', exact: true })
      .click();
    await expect(panel(page)).toContainText(source.values.summary);
    const calls = family === 'hearing_result' ? state.facts.results.calls : state.facts.calls;
    expect(calls.at(-1).path).toBe(href);
    expect(calls.every((c) => c.method === 'GET')).toBe(true);
  });
