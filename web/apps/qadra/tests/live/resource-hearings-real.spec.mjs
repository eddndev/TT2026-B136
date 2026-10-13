import { test, expect } from '@playwright/test';
import { accounts } from './resource-activities-real-helpers.mjs';
import { responseTo } from './procedural-resources-helpers.mjs';
import {
  ownEditor,
  ownDetail,
  ownRoute,
  ownExact,
  withOwnSession,
  fillOwnHearing,
  assertOwnDraft,
  unlinkOwn,
  captureOwn,
} from './resource-hearings-real-helpers.mjs';
import {
  ownAlerts,
  waitOwnAlert,
  checkOwnAgenda,
  checkOwnInbox,
} from './resource-hearings-tracking-helpers.mjs';

if (!accounts?.desktop?.resourceInitial || !accounts?.mobile?.resourceAct)
  throw new Error('The live resource activities fixture must retain resource and act history');

for (const [name, width, role, code] of [
  ['desktop', 1440, 'owner', 7],
  ['mobile', 390, 'litigator', 2],
]) {
  test(`real own hearing reconciles creation and shares its exact origin with agenda and alert at ${width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(90000);
    const scenario = accounts[name],
      actor = accounts[role],
      errors = [],
      writes = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    // Earlier resource scenarios reserve Owner codes 0..6 and Litigator 0..1.
    await withOwnSession(page, actor, code, async (call) => {
      const preferences = (await call('GET', '/alert-preferences')).preferences;
      expect(preferences.email_transport).toBe('disabled');
      expect(preferences.values.hearing_upcoming).toEqual({
        lead_hours: [48, 24],
        channels: { internal: true, email: true },
      });
      const participant = await call(
        'POST',
        `/cases/${scenario.case.id}/participants`,
        {
          display_name: `Participante de audiencia propia ${name}`,
          procedural_role: 'Testigo',
        },
        201,
      );
      page.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (
          path.startsWith('/api/v1/') &&
          path !== '/api/v1/auth/activity' &&
          request.method() !== 'GET'
        )
          writes.push({ path, method: request.method(), body: request.postDataJSON() });
      });
      const schedule = await fillOwnHearing(page, scenario, participant);
      const base = '/api/v1' + ownRoute(scenario);
      const preparing = responseTo(page, base + '/prepare', 'POST');
      await ownEditor(page)
        .getByRole('button', { name: 'Preparar audiencia y vinculo', exact: true })
        .click();
      const preparedResponse = await preparing;
      expect(preparedResponse.status()).toBe(200);
      expect(preparedResponse.headers()['cache-control']).toBe('no-store');
      const draft = await preparedResponse.json();
      assertOwnDraft(draft, scenario, actor, participant, schedule);
      const confirm = ownEditor(page).getByRole('button', {
        name: 'Confirmar audiencia y vinculo',
        exact: true,
      });
      await expect(confirm).toBeDisabled();
      await ownEditor(page)
        .getByRole('checkbox', {
          name: 'Reconozco la programacion y las capturas seleccionadas',
          exact: true,
        })
        .check();
      const endpoint = `**${base}/submit`;
      let creation, submitted, finished, failed;
      const intercepted = new Promise((resolve, reject) => {
        finished = resolve;
        failed = reject;
      });
      await page.route(endpoint, async (intercept) => {
        try {
          submitted = intercept.request().postDataJSON();
          const response = await intercept.fetch({ maxRedirects: 0 });
          expect(response.status()).toBe(201);
          expect(response.headers()['cache-control']).toBe('no-store');
          creation = await response.json();
          await intercept.abort('failed');
          finished();
        } catch (error) {
          failed(error);
          await intercept.abort('failed').catch(() => {});
        }
      });
      try {
        // The uncertainty heading can appear before the pending POST completes.
        await Promise.all([confirm.click(), intercepted]);
        await expect(
          ownEditor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
        ).toBeVisible();
        expect(submitted).toEqual({
          command: draft.command,
          expected_submission_digest: draft.submission_digest,
        });
        expect(creation.submission_digest).toBe(draft.submission_digest);
        expect(creation.hearing.resource).toEqual(draft.command.resource);
        expect(creation.hearing.act).toEqual(draft.command.act);
        expect(creation.hearing.values).toEqual(draft.command.values);
        expect(creation.hearing.recorded_by).toEqual(draft.recorded_by);
        expect(creation.association.sources.target).toEqual({
          kind: 'resource_hearing',
          record: creation.hearing,
        });
        expect(creation.origin).toEqual({
          case_id: scenario.case.id,
          resource_id: scenario.resource.id,
          hearing_id: draft.command.hearing_id,
          association_id: draft.command.association_id,
          operation_id: draft.command.operation_id,
          submission_digest: draft.submission_digest,
          capture_digest: creation.hearing.capture_digest,
        });
        const unlinked = await unlinkOwn(call, scenario, creation);
        const reconciling = responseTo(page, ownExact(scenario, creation));
        await ownEditor(page)
          .getByRole('button', { name: 'Consultar resultado', exact: true })
          .click();
        const exact = await reconciling;
        expect(exact.status()).toBe(200);
        expect(exact.headers()['cache-control']).toBe('no-store');
        expect(await exact.json()).toEqual(creation);
        await expect(ownEditor(page)).toHaveCount(0);
        await expect(ownDetail(page)).toContainText(creation.hearing.values.venue);
        expect(writes.map(({ path }) => path)).toEqual([base + '/prepare', base + '/submit']);
        await testInfo.attach('own-hearing-creation-and-unlink', {
          body: JSON.stringify({ draft, creation, unlinked }, null, 2),
          contentType: 'application/json',
        });
      } finally {
        await page.unroute(endpoint);
      }
      await ownDetail(page).getByText('Autor y recibo original', { exact: true }).click();
      await expect(ownDetail(page)).toContainText(creation.hearing.capture_digest);
      await captureOwn(page, testInfo, `resource-hearing-reconciled-${name}`);
      const agenda = await checkOwnAgenda(page, scenario, creation, schedule.seconds);
      const alert = await waitOwnAlert(call, actor, creation, schedule.seconds);
      const receipt = await checkOwnInbox(page, scenario, creation, alert, testInfo, name);
      expect((await call('GET', `/alerts/${alert.id}`)).alert).toEqual(receipt.alert);
      expect(await ownAlerts(call, creation)).toEqual([receipt.alert]);
      expect(writes.map(({ path }) => path)).toEqual([
        base + '/prepare',
        base + '/submit',
        `/api/v1/alerts/${alert.id}/read`,
      ]);
      expect(writes.every(({ method }) => method === 'POST')).toBe(true);
      const ownPage = await call('GET', ownRoute(scenario) + '?limit=20');
      expect(ownPage.items.filter((row) => row.hearing.id === creation.hearing.id)).toEqual([
        creation,
      ]);
      expect(ownPage.has_more).toBe(false);
      await testInfo.attach('own-hearing-agenda-and-alert', {
        body: JSON.stringify({ agenda, alert, receipt }, null, 2),
        contentType: 'application/json',
      });
      expect(errors).toEqual([]);
    });
  });
}
