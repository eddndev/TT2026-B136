import { test, expect } from '@playwright/test';
import { resourceDeadlineDraft } from '../../src/lib/resource-deadline-values.mjs';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import { responseTo } from './procedural-resources-helpers.mjs';
import {
  accounts,
  panel,
  detail,
  route,
  openActivities,
  resourceReference,
  accountAction,
} from './resource-activities-real-helpers.mjs';
import {
  editor,
  fillContextualDeadline,
  queryContextualAgenda,
} from './resource-activities-contextual-helpers.mjs';

if (!accounts?.contextual) throw new Error('The contextual deadline fixture must be provisioned');

test('real resource act creates one deadline and association visible in the agenda', async ({
  page,
}, testInfo) => {
  const scenario = accounts.contextual,
    actor = accounts.owner;
  const errors = [],
    writes = [];
  page.on('pageerror', (failure) => errors.push(failure.message));
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await loginAs(page, actor, 3);
  await openActivities(page, scenario);
  page.on('request', (request) => {
    const path = new URL(request.url()).pathname;
    if (path.startsWith('/api/v1/') && request.method() !== 'GET') writes.push(path);
  });
  await panel(page).getByRole('button', { name: 'Crear plazo', exact: true }).click();
  await fillContextualDeadline(page, scenario, actor, 'Plazo contextual con acto exacto');
  const base = '/api/v1' + route(scenario) + '/deadlines';
  const preparing = responseTo(page, base + '/prepare', 'POST');
  await editor(page).getByRole('button', { name: 'Preparar plazo y vinculo', exact: true }).click();
  const preparedResponse = await preparing;
  expect(preparedResponse.status()).toBe(200);
  expect(preparedResponse.headers()['cache-control']).toBe('no-store');
  const draft = await preparedResponse.json();
  await testInfo.attach('contextual-preparation', {
    body: JSON.stringify(draft, null, 2),
    contentType: 'application/json',
  });
  resourceDeadlineDraft(draft);
  expect(draft.command.resource).toEqual(resourceReference(scenario.resource));
  expect(draft.command.act).toEqual({
    id: scenario.resourceAct.act.id,
    revision: 1,
    resource_revision: 2,
    capture_digest: scenario.resourceAct.receipt.capture_digest,
  });
  expect(draft.association.resource).toEqual(scenario.resource);
  expect(draft.association.act).toEqual(scenario.resourceAct);
  expect(draft.deadline.calculation.result.blocks).toEqual([]);
  expect(draft.deadline.calculation.result.due_at).toEqual(scenario.dueAt);
  const confirm = editor(page).getByRole('button', {
    name: 'Confirmar plazo y vinculo',
    exact: true,
  });
  await expect(confirm).toBeDisabled();
  await editor(page)
    .getByRole('checkbox', {
      name: 'Reconozco el resultado y las capturas seleccionadas',
      exact: true,
    })
    .check();
  const submitting = responseTo(page, base + '/submit', 'POST');
  await confirm.click();
  const committedResponse = await submitting;
  expect(committedResponse.status()).toBe(201);
  expect(committedResponse.headers()['cache-control']).toBe('no-store');
  expect(committedResponse.request().postDataJSON()).toEqual({
    command: draft.command,
    expected_submission_digest: draft.submission_digest,
  });
  const result = await committedResponse.json();
  const { deadline, association } = result;
  expect(result.submission_digest).toBe(draft.submission_digest);
  expect(deadline.id).toBe(draft.command.deadline.deadline_id);
  expect(deadline.receipt.operation_id).toBe(draft.command.deadline.operation_id);
  expect(association.receipt.operation_id).toBe(deadline.receipt.operation_id);
  expect(deadline.receipt.submission_digest).toBe(draft.deadline.submission_digest);
  expect(association.receipt.submission_digest).toBe(draft.association.submission_digest);
  expect(association.selection.target).toEqual({
    kind: 'deadline',
    ...resourceReference(deadline),
  });
  expect(association.sources.target).toEqual({ kind: 'deadline', record: deadline });
  await expect(editor(page)).toHaveCount(0);
  await expect(detail(page)).toContainText('Plazo contextual con acto exacto');
  expect(writes).toEqual([base + '/prepare', base + '/submit']);
  await accountAction(actor, 4, async (call) => {
    const replay = await call(
      'POST',
      route(scenario) + '/deadlines/submit',
      {
        command: draft.command,
        expected_submission_digest: draft.submission_digest,
      },
      201,
    );
    expect(replay).toEqual(result);
    const list = await call('GET', route(scenario));
    expect(list.associations).toHaveLength(1);
    expect(list.associations[0].association).toEqual(association);
    expect((await call('GET', `/cases/${scenario.case.id}/deadlines`)).deadlines).toHaveLength(1);
  });
  await page.screenshot({
    path: testInfo.outputPath('contextual-deadline-desktop.png'),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await navigate(page, 'Agenda');
  await queryContextualAgenda(page, scenario, deadline);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({
    path: testInfo.outputPath('contextual-deadline-mobile.png'),
    fullPage: true,
  });
  expect(errors).toEqual([]);
});
