import { writeFile } from 'node:fs/promises';
import { test, expect } from '@playwright/test';
import {
  accounts,
  caseRoute,
  withSession,
  scheduleHearing,
  declareMeasure,
  capture,
  hearingDetail,
  decisionDetail,
  administrationDetail,
} from './precautionary-hearings-real-helpers.mjs';
import { agendaAndAlert, hearingAlerts } from './precautionary-hearings-real-tracking.mjs';
import { rectifyWithLostResponse } from './precautionary-hearings-real-administration.mjs';

if (!accounts?.desktop?.subject || !accounts?.mobile?.initial)
  throw new Error('The live precautionary fixture must retain exact subjects and initial hearings');

for (const [name, width, role, anchor, action] of [
  ['desktop', 1440, 'owner', 'precautionary', 'correct'],
  ['mobile', 390, 'litigator', 'initial', 'replace_entered_in_error'],
]) {
  test(`real precautionary workflow preserves original hearing decision and rectification at ${width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(180000);
    const scenario = accounts[name],
      actor = accounts[role],
      errors = [],
      writes = [];
    const principal = { id: actor.id, email: actor.email, role };
    page.on('pageerror', (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await withSession(page, actor, async (call) => {
      const preferences = (await call('GET', '/alert-preferences')).preferences;
      expect(preferences.email_transport).toBe('disabled');
      expect(preferences.values.hearing_upcoming.lead_hours).toEqual([48, 24]);
      expect(preferences.values.hearing_upcoming.channels.internal).toBe(true);
      page.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (path.startsWith('/api/v1' + caseRoute(scenario) + '/') && request.method() !== 'GET')
          writes.push({ path, method: request.method(), body: request.postDataJSON() });
      });
      const scheduled = await scheduleHearing(page, scenario),
        hearing = scheduled.operation;
      expect(hearing.capture.review).toEqual(scheduled.prepared);
      expect(scheduled.prepared.actor).toEqual(principal);
      expect(scheduled.prepared.resolved_values.scheduled_at).toBe(scheduled.scheduledAt);
      expect(scheduled.prepared.resolved_values.purpose).toBe('imposition');
      expect(scheduled.prepared.resolved_values.review_targets).toEqual([]);
      expect(scheduled.prepared.sources.support).toMatchObject({
        document_id: scenario.support.id,
        version: scenario.support.version,
        digest: scenario.support.digest,
      });
      expect(scheduled.prepared.sources.participants).toHaveLength(1);
      expect(scheduled.prepared.sources.participants[0].subject).toEqual(scenario.subject);
      expect(hearing.history.captures).toEqual([hearing.capture]);
      await hearingDetail(page).getByText('Autor y recibo original', { exact: true }).click();
      await expect(hearingDetail(page)).toContainText(hearing.capture.capture_digest);
      const tracking = await agendaAndAlert(page, call, scenario, actor, scheduled, testInfo, name);
      const decision = await declareMeasure(page, scenario, hearing, anchor);
      expect(decision.operation.group.review).toEqual(decision.prepared.review);
      expect(decision.prepared.review.actor).toEqual(principal);
      expect(decision.operation.group.measures).toHaveLength(1);
      const judicial = decision.operation.group.measures[0];
      expect(judicial.result.action).toBe('impose');
      expect(judicial.result.sources.subject).toEqual(scenario.subject);
      expect(judicial.result.values.validity.start).toEqual({
        precision: 'unknown',
        reason: 'No consta inicio de vigencia',
      });
      expect(judicial.result.values.validity.end).toBeNull();
      await expect(decisionDetail(page)).toContainText('Persona declarada');
      await capture(page, testInfo, `precautionary-decision-${name}`);
      const path = caseRoute(scenario);
      const before = await call('GET', `${path}/measures/${judicial.result.id}`);
      const administration = await rectifyWithLostResponse(page, scenario, before, action);
      expect(administration.prepared.actor).toEqual(principal);
      const current = await call('GET', `${path}/measures/${before.reference.id}`);
      expect(current.reference.revision).toBe(before.reference.revision + 1);
      expect(current.judicial_origin).toEqual(before.judicial_origin);
      expect(current.last_judicial).toEqual(before.last_judicial);
      expect(current.last_action).toBe('impose');
      const historical = await call(
        'GET',
        `${path}/measures/${before.reference.id}/revisions/${before.reference.revision}?capture_digest=${before.reference.capture_digest}`,
      );
      expect(historical).toEqual(before);
      let replacement = null;
      if (action === 'correct') {
        expect(current.validity).toBe('valid');
        expect(current.record.capture.result.values.conditions).toBe(
          'Presentarse en la sede rectificada del soporte',
        );
        expect(administration.operation.capture.replacement_link).toBeNull();
      } else {
        replacement = await call(
          'GET',
          `${path}/measures/${administration.prepared.command.action.replacement_id}`,
        );
        expect(current.validity).toBe('entered_in_error');
        expect(replacement.reference.revision).toBe(1);
        expect(replacement.judicial_origin).toEqual(before.judicial_origin);
        expect(replacement.last_judicial).toEqual(before.last_judicial);
        expect(replacement.record.capture.result.sources.subject).toEqual(
          scenario.replacementSubject,
        );
        expect(administration.operation.capture.replacement_link).toEqual({
          entered_in_error: current.reference,
          replacement: replacement.reference,
        });
        await expect(administrationDetail(page)).toContainText('Persona sustituta declarada');
      }
      await administrationDetail(page)
        .getByText('Contexto y recibo original', { exact: true })
        .click();
      await expect(administrationDetail(page)).toContainText(
        administration.operation.origin.capture_digest,
      );
      await capture(page, testInfo, `precautionary-rectification-recovered-${name}`);
      const hearingPage = await call('GET', `${path}/precautionary-hearings?limit=20`);
      expect(hearingPage.items).toEqual([hearing]);
      expect(hearingPage.has_more).toBe(false);
      const initialPage = await call('GET', `${path}/hearings?status=all&limit=20`);
      expect(initialPage.hearings.map((row) => row.id)).toEqual([scenario.initial.id]);
      expect(await call('GET', `${path}/hearings/${scenario.initial.id}/revisions/1`)).toEqual(
        scenario.initial,
      );
      expect(
        await call('GET', `${path}/measure-decisions/${decision.operation.origin.decision_id}`),
      ).toEqual(decision.operation);
      expect(
        (await call('GET', `${path}/measure-administrative-operations?limit=20`)).items,
      ).toEqual([administration.operation]);
      expect(await hearingAlerts(call, hearing.capture.review.command.hearing_id)).toEqual([
        tracking.receipt.alert,
      ]);
      expect(writes.map((row) => row.path)).toEqual([
        `${'/api/v1' + path}/precautionary-hearings/prepare`,
        `${'/api/v1' + path}/precautionary-hearings/submit`,
        `${'/api/v1' + path}/measure-decisions/prepare`,
        `${'/api/v1' + path}/measure-decisions/submit`,
        `${'/api/v1' + path}/measure-administrative-operations/prepare`,
        `${'/api/v1' + path}/measure-administrative-operations/submit`,
      ]);
      expect(writes.every((row) => row.method === 'POST')).toBe(true);
      const receiptPath = testInfo.outputPath('precautionary-receipt.json');
      await writeFile(
        receiptPath,
        JSON.stringify(
          {
            principal,
            scheduled,
            tracking,
            decision,
            before,
            administration,
            current,
            historical,
            replacement,
            writes,
          },
          null,
          2,
        ),
        'utf8',
      );
      await testInfo.attach('precautionary-original-records-and-recovery', {
        path: receiptPath,
        contentType: 'application/json',
      });
      expect(errors).toEqual([]);
    });
  });
}
