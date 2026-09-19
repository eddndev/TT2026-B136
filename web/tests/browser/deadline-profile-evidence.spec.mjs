import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  deadlineFixture,
  caseId,
} from './deadline-editor-helpers.mjs';
import { profile, id } from '../fixtures/deadline-unit.mjs';
import { v1Record as detail } from '../fixtures/deadline-v2-unit.mjs';
import { calendarFixtureValues } from '../fixtures/judicial-calendars.mjs';

test('exact profile examples expose their complete embedded calendar and declared reference titles', async ({
  page,
}) => {
  const exact = profile(caseId),
    calendar = calendarFixtureValues();
  calendar.sources = calendar.sources.map((source, index) => ({
    ...source,
    title: `Fuente del ejemplo ${index + 1}`,
  }));
  calendar.weekly_pattern = calendar.weekly_pattern.map((rule) => ({
    ...rule,
    classification: 'countable',
    source_ids: [calendar.sources[0].id],
    explanation: 'Dia computable del ejemplo',
  }));
  exact.definition.template = {
    kind: 'fixed',
    rule: {
      kind: 'days',
      quantity: 1,
      inclusion: 'after_anchor',
      basis: 'calendar_countable',
      final_day: 'preserve',
    },
  };
  exact.definition.completion = { kind: 'civil_candidate_only' };
  exact.definition.examples = [
    {
      ...exact.definition.examples[0],
      anchor: { precision: 'date', year: 2000, month: 2, day: 28, offset_seconds: null },
      calendar,
      expected: { kind: 'arithmetic', outcome: { kind: 'civil_candidate', date: '2000-03-01' } },
    },
  ];
  exact.definition.examples.push({
    ...structuredClone(exact.definition.examples[0]),
    id: id(24),
    calendar: null,
    expected: {
      kind: 'arithmetic',
      outcome: { kind: 'blocked', block: { kind: 'missing_calendar' } },
    },
  });
  const row = detail(deadlineFixture()),
    state = await setupDeadlines(page, { deadlines: [row], profiles: [exact] });
  await openDeadlines(page);
  await page.getByRole('button', { name: `Consultar plazo ${row.id}`, exact: true }).click();
  const panel = page.getByRole('region', { name: 'Perfil exacto del plazo', exact: true });
  await panel.getByRole('button', { name: 'Consultar perfil capturado', exact: true }).click();
  await panel.getByText('Ejemplos declarados del perfil (2)', { exact: true }).click();
  const examples = panel
    .locator('details')
    .filter({ has: page.getByText('Ejemplos declarados del perfil (2)', { exact: true }) });
  const first = examples.locator(':scope > article').nth(0),
    second = examples.locator(':scope > article').nth(1);
  await expect(first).toContainText(
    `Referencias del ejemplo: ${exact.definition.references[0].title}`,
  );
  await first.getByText('Calendario declarado del ejemplo', { exact: true }).click();
  await expect(first).toContainText(calendar.scope.title);
  await expect(first).toContainText(`${calendar.coverage.from} / ${calendar.coverage.through}`);
  await expect(
    first.getByRole('link', {
      name: `Abrir referencia: ${calendar.sources[0].title}`,
      exact: true,
    }),
  ).toHaveAttribute('href', calendar.sources[0].official_url);
  await first
    .getByText('Patr\u00f3n semanal y excepciones de esta revisi\u00f3n', { exact: true })
    .click();
  await expect(first).toContainText(calendar.exceptions[0].explanation);
  await expect(first).toContainText('Dia computable del ejemplo');
  await expect(second).toContainText('Sin calendario declarado en este ejemplo.');
  expect(state.calls.filter((call) => call.path.includes('/judicial-calendars'))).toHaveLength(0);
  expect(state.calls.at(-1).path).toBe(
    `/api/v1/cases/${caseId}/deadline-profiles/${exact.id}/revisions/1`,
  );
});
