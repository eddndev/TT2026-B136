import { expect } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { loginAs } from './helpers.mjs';
import { exact, responseTo } from './procedural-resources-helpers.mjs';
import {
  openActivities,
  panel,
  route,
  resourceReference,
} from './resource-activities-real-helpers.mjs';

export const ownEditor = (page) =>
  page.getByRole('region', { name: 'Formulario de audiencia de recurso', exact: true });
export const ownDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia de recurso', exact: true });
export const ownRoute = (scenario) => route(scenario) + '/resource-hearings';
export const ownExact = (scenario, creation) =>
  '/api/v1' + ownRoute(scenario) + `/${creation.hearing.id}/revisions/1`;

export async function withOwnSession(page, actor, index, run) {
  const verified = responseTo(page, '/api/v1/auth/mfa/recovery', 'POST');
  await loginAs(page, actor, index);
  const session = await (await verified).json();
  let token = session.access_token;
  delete session.access_token;
  async function call(method, path, data, expected = 200) {
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      redirect: 'error',
      headers: {
        Authorization: `Bearer ${token}`,
        ...(data === undefined ? {} : { 'Content-Type': 'application/json' }),
      },
      body: data === undefined ? undefined : JSON.stringify(data),
    });
    expect(response.status, `${method} ${path}`).toBe(expected);
    return response.status === 204 ? null : response.json();
  }
  try {
    return await run(call);
  } finally {
    try {
      await call('POST', '/auth/logout', undefined, 204);
    } finally {
      token = null;
    }
  }
}

export async function fillOwnHearing(page, scenario, participant) {
  await openActivities(page, scenario);
  expect(await exact(page, scenario, 1)).toEqual(scenario.resourceInitial);
  await panel(page)
    .getByRole('button', { name: 'Crear audiencia de recurso', exact: true })
    .click();
  const form = ownEditor(page);
  await expect(form).toBeVisible();
  const seconds = Math.floor(Date.now() / 1000) + 36 * 3600;
  const local = new Date((seconds - 6 * 3600) * 1000).toISOString();
  const scheduled = local.slice(0, 19) + '-06:00';
  await form.getByLabel('Fecha', { exact: true }).fill(local.slice(0, 10));
  await form.getByLabel('Hora', { exact: true }).fill(local.slice(11, 19));
  await form.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await form
    .getByLabel('Sede o enlace', { exact: true })
    .fill(`Sala propia ${scenario.case.reference}`);
  await form
    .getByLabel('Nota', { exact: true })
    .fill('Programacion declarada con captura historica');
  await form.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
  await form.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
  await form.getByRole('button', { name: 'Usar este acto', exact: true }).click();
  await form
    .getByLabel('Base de senalamiento', { exact: true })
    .fill('Senalamiento sintetico con soporte previamente admitido');
  const support = scenario.resourceAct.act.supports[0];
  await form
    .getByRole('combobox', { name: 'Soporte admitido del senalamiento', exact: true })
    .selectOption({ label: `${support.name} / Version ${support.version}` });
  await form.getByRole('button', { name: 'Elegir participante', exact: true }).click();
  await form
    .getByRole('button', { name: `Consultar ficha: ${participant.display_name}`, exact: true })
    .click();
  await form.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
  return { scheduled, seconds, support };
}

export function assertOwnDraft(draft, scenario, actor, participant, schedule) {
  const command = draft.command;
  expect(command.case_id).toBe(scenario.case.id);
  expect(command.resource_id).toBe(scenario.resource.id);
  expect(command.resource).toEqual(resourceReference(scenario.resourceInitial));
  expect(command.expected_resource_revision).toBe(3);
  expect(command.act).toEqual({
    id: scenario.resourceAct.act.id,
    revision: 1,
    resource_revision: 2,
    capture_digest: scenario.resourceAct.receipt.capture_digest,
  });
  expect(command.values.kind).toBe(
    scenario.resourceInitial.values.kind === 'appeal' ? 'appeal_arguments' : 'written_revocation',
  );
  expect(command.values.scheduled_at).toBe(schedule.scheduled);
  expect(command.values.participants).toEqual([
    { participant_id: participant.id, revision: participant.revision },
  ]);
  const { document_id, version, digest } = schedule.support;
  expect(command.values.scheduling_basis.support).toEqual({ document_id, version, digest });
  expect(draft.recorded_by).toEqual({ id: actor.id, email: actor.email });
  expect(draft.resource).toEqual(scenario.resourceInitial);
  expect(draft.act).toEqual(scenario.resourceAct);
  expect(draft.sources.support).toEqual(schedule.support);
  expect(draft.sources.participants).toHaveLength(1);
  expect(draft.sources.participants[0]).toMatchObject({
    id: participant.id,
    revision: participant.revision,
    display_name: participant.display_name,
  });
  expect(draft.observed_resource_head).toEqual(resourceReference(scenario.resource));
}

export async function unlinkOwn(call, scenario, creation) {
  const command = {
    case_id: scenario.case.id,
    resource_id: scenario.resource.id,
    association_id: creation.association.id,
    operation_id: randomUUID(),
    expected_resource_revision: 3,
    change: {
      action: 'unlink',
      expected_revision: 1,
      reason: 'Separacion organizativa sin cancelar programacion',
    },
  };
  const draft = await call('POST', route(scenario) + '/prepare', command);
  const result = await call(
    'POST',
    route(scenario) + `/${creation.association.id}/unlink`,
    {
      command: draft.command,
      expected_submission_digest: draft.submission_digest,
    },
    201,
  );
  expect(result.status).toBe('unlinked');
  expect(result.revision).toBe(2);
  expect(result.sources.target).toEqual({ kind: 'resource_hearing', record: creation.hearing });
  return result;
}

export async function captureOwn(page, testInfo, name) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.evaluate(async () => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
}
