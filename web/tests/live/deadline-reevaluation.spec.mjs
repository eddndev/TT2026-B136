import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import {
  openDeadlines,
  openDeadline,
  editor,
  detail,
  prepare,
  submit,
} from './deadline-helpers.mjs';
import {
  following,
  calculation,
  dueAt,
  historical,
  withReevaluationApi,
  refreshDeadline,
  technicalReceipt,
  captureReevaluation,
} from './deadline-reevaluation-helpers.mjs';
import {
  advanceFollowCalendar,
  advanceFollowSource,
  waitForDeadlineRevision,
} from '../../../scripts/web-deadline-reevaluation.mjs';

if (!fixture.deadlineReevaluation)
  throw new Error('Enable deadline reevaluation fixtures before selecting this live spec');

for (const [name, width] of [
  ['desktop', 1440],
  ['mobile', 390],
])
  test(`real Follow reevaluation and explicit human review at ${width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(180000);
    const scenario = fixture.deadlineReevaluation[name];
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await loginAs(page, scenario.operator, 0);
    await openDeadlines(page, scenario.case);
    await openDeadline(page, scenario.deadline);
    await expect(following(page)).toContainText('2026-01-31 23:30:00');
    await withReevaluationApi(scenario.owner, async (call) => {
      const path = `/cases/${scenario.case.id}/deadlines/${scenario.deadline.id}`;
      const first = scenario.deadline;
      expect((await call('GET', path)).operational.due_at).toEqual(dueAt('2026-01-31T23:30:00Z'));
      const calendar = await advanceFollowCalendar(call, scenario.calendar);
      const second = await waitForDeadlineRevision(call, scenario.case.id, first.id, 2);
      technicalReceipt(second, first, calendar, 'calendar', 'accepted');
      expect(second.definition.input.calendar).toEqual({ id: calendar.id, revision: 2 });
      expect(second.operational.freshness).toBe('current');
      expect(second.operational.due_at).toEqual(dueAt('2026-02-01T23:30:00Z'));
      await refreshDeadline(page, second);
      await expect(following(page)).toContainText('Revisi\u00f3n aceptada');
      await expect(following(page)).toContainText('2026-02-01 23:30:00');
      await expect(detail(page)).toContainText('Servicio de reevaluaci\u00f3n');
      await captureReevaluation(page, testInfo, `follow-calendar-${name}`);

      const source = await advanceFollowSource(call, scenario.case.id, scenario.source);
      const third = await waitForDeadlineRevision(call, scenario.case.id, first.id, 3);
      technicalReceipt(third, second, source, 'resolution', 'pending');
      expect(third.tracking.review.reasons).toEqual([
        { dependency: 'source', reason: 'source_changed' },
      ]);
      expect(third.calculation).toEqual(second.calculation);
      expect(third.definition).toEqual(second.definition);
      expect(third.definition.input.selection.source.value.revision).toBe(1);
      expect(
        third.tracking.observations.entries.find((entry) => entry.role === 'source').revision,
      ).toBe(2);
      expect(third.operational.due_at).toBeNull();
      await refreshDeadline(page, third);
      await expect(following(page)).toContainText('Revisi\u00f3n pendiente');
      await expect(following(page)).toContainText('Cambi\u00f3 la fuente');
      await expect(following(page).locator('.deadline-operational')).not.toContainText(
        '2026-02-01',
      );
      await expect(calculation(page)).toContainText('2026-02-01 23:30:00');
      await captureReevaluation(page, testInfo, `follow-pending-${name}`);

      await detail(page).getByRole('button', { name: 'Revisar seguimiento', exact: true }).click();
      const form = editor(page);
      for (const dependency of ['el perfil', 'la fuente', 'el calendario'])
        await expect(
          form.getByRole('combobox', { name: `Cuando cambie ${dependency}`, exact: true }),
        ).toHaveValue('follow');
      await form.getByRole('button', { name: 'Elegir fuente exacta', exact: true }).click();
      await form
        .getByRole('button', {
          name: `Consultar revisiones de resolucion ${source.id}`,
          exact: true,
        })
        .click();
      await form
        .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 2', exact: true })
        .click();
      await form
        .getByRole('button', {
          name: 'Vincular esta revisi\u00f3n de resoluci\u00f3n',
          exact: true,
        })
        .click();
      await form
        .getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })
        .fill('Revisada expresamente frente a la fuente actual sintetica');
      await form
        .getByRole('combobox', { name: 'El ambito del perfil aplica', exact: true })
        .selectOption('yes');
      await form
        .getByRole('combobox', { name: 'Existe una incidencia sin resolver', exact: true })
        .selectOption('no');
      await form
        .getByRole('combobox', { name: 'Se cumple la condicion 1', exact: true })
        .selectOption('yes');
      await form
        .getByLabel('Motivo', { exact: true })
        .fill('Aceptar explicitamente la nueva fuente declarada');
      const draft = await prepare(page);
      expect(draft.command.change.expected_revision).toBe(3);
      expect(draft.command.change.tracking).toEqual({
        profile: 'follow',
        source: 'follow',
        calendar: 'follow',
      });
      expect(draft.definition.input.selection.source.value.revision).toBe(2);
      expect(draft.tracking.review.state).toBe('accepted');
      expect(draft.author).toEqual({
        kind: 'user',
        id: scenario.operator.id,
        email: scenario.operator.email,
      });
      const fourth = await submit(page, draft);
      expect(fourth.revision).toBe(4);
      expect(fourth.recorded_by).toEqual(draft.author);
      expect(fourth.receipt.version.cause).toBeNull();
      expect(fourth.operational.freshness).toBe('not_checked');
      await expect(following(page)).toContainText('Vigencia no consultada');
      await refreshDeadline(page, fourth);
      await expect(following(page)).toContainText('2026-02-02 23:30:00');
      expect((await call('GET', path)).operational.due_at).toEqual(dueAt('2026-02-02T23:30:00Z'));
      await captureReevaluation(page, testInfo, `follow-accepted-${name}`);

      for (const row of [first, second, third, fourth])
        expect(await call('GET', `${path}/revisions/${row.revision}`)).toEqual(historical(row));
      await detail(page)
        .getByRole('button', { name: 'Ver historial de plazo', exact: true })
        .click();
      const history = page.getByRole('region', { name: 'Historial de plazo', exact: true });
      await expect(history).toContainText('Servicio de reevaluaci\u00f3n');
      await expect(history).toContainText(scenario.operator.email);
      await history
        .getByRole('button', { name: 'Consultar plazo revisi\u00f3n 2', exact: true })
        .click();
      await expect(detail(page)).toContainText('Consultada exactamente');
      await expect(following(page).locator('.deadline-operational')).not.toContainText(
        '2026-02-01',
      );
      await expect(calculation(page)).toContainText('2026-02-01 23:30:00');
      expect(
        (await call('GET', `${path}/history?limit=20`)).revisions.map((row) => row.revision),
      ).toEqual([4, 3, 2, 1]);
      expect((await call('GET', '/audit/verify')).valid).toBe(true);
      expect(errors).toEqual([]);
    });
  });
