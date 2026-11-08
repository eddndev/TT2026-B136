import { test, expect } from '@playwright/test';
import { detail } from './hearing-result-helpers.mjs';
import { capture } from './case-administration-helpers.mjs';
import {
  scenario,
  scope,
  endpoint,
  editor,
  responseTo,
  authenticate,
  openEditor,
  fillEditor,
  prepare,
  acknowledge,
  submit,
  verifyExact,
} from './hearing-derived-deadline-helpers.mjs';
import { observeAgenda, observeAlerts } from './hearing-derived-deadline-observation.mjs';

for (const [width, recoveryBase] of [[1440, 0]])
  for (const [timed, recoveryOffset] of [
    [true, 0],
    [false, 1],
  ])
    test(`real compound result creates a ${timed ? 'calculable' : 'blocked'} deadline at ${width}px`, async ({
      page,
    }, info) => {
      const errors = [],
        writes = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.setViewportSize({ width, height: 1000 });
      const call = await authenticate(page, recoveryBase + recoveryOffset);
      page.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (path.startsWith(scope) && request.method() !== 'GET')
          writes.push([request.method(), path]);
      });
      await openEditor(page);
      const title = `Plazo conjunto ${timed ? 'calculable' : 'sin hora'} ${width}`;
      await fillEditor(page, title, timed);
      const ready = await prepare(page);
      expect(ready.result.values.event_time).toEqual(
        timed
          ? { precision: 'instant', at: scenario.instant.at }
          : { precision: 'date', date: scenario.instant.date, offset: scenario.instant.offset },
      );
      expect(ready.deadline.result.due_at).toEqual(timed ? scenario.dueAt : null);
      if (timed) expect(ready.deadline.result.blocks).toEqual([]);
      else {
        expect(ready.deadline.result.blocks).toContainEqual({
          kind: 'arithmetic',
          block: { kind: 'insufficient_precision', observed: 'date' },
        });
        await expect(editor(page)).toContainText('Sin vencimiento calculado');
        await expect(editor(page)).toContainText('no permite este c');
      }
      await capture(page, info, 'joint-review');
      const record = await submit(page);
      await expect(detail(page)).toContainText(`Resultado sintetico: ${title}`);
      await verifyExact(call, ready, record);
      const agenda = await observeAgenda(page, call, record, timed);
      const alerts = await observeAlerts(page, call, record, timed);
      expect(writes).toEqual([
        ['POST', `${endpoint}/prepare`],
        ['POST', `${endpoint}/submit`],
      ]);
      await capture(page, info, timed ? 'deadline-alert' : 'blocked-result');
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
      await info.attach('compound-acceptance', {
        body: Buffer.from(JSON.stringify({ ready, record, agenda, alerts }, null, 2)),
        contentType: 'application/json',
      });
      expect(errors).toEqual([]);
    });
