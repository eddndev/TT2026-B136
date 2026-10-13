import { test, expect } from '@playwright/test';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import { activityDetail } from './resource-activities-helpers.mjs';
import {
  setupResourceHearing,
  openHearingForm,
  fillHearingForm,
  selectActSupportAndParticipant,
  prepareHearing,
  submitHearing,
  hearingEditor,
  hearingDetail,
  activityPanel,
  reference,
  participant,
} from './resource-hearing-scheduling-helpers.mjs';

for (const [width, role] of [
  [1440, 'owner'],
  [390, 'litigator'],
]) {
  test(`programs an own hearing from historical resource and act at ${width}px`, async ({
    page,
  }, info) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupResourceHearing(page, { role });
    await openHearingForm(page, state);
    await fillHearingForm(page);
    await selectActSupportAndParticipant(page, state);
    await prepareHearing(page);
    await expect(hearingEditor(page)).toContainText('Interposicion original declarada');
    await expect(hearingEditor(page)).toContainText('senalamiento-del-acto.pdf');
    const command = state.calls.findLast((call) => call.path.endsWith('/prepare')).body;
    expect(command.resource).toEqual(reference(state.activities.original));
    expect(command.expected_resource_revision).toBe(3);
    expect(command.act).toEqual({
      id: state.activities.act.act.id,
      revision: 1,
      resource_revision: 2,
      capture_digest: state.activities.act.receipt.capture_digest,
    });
    expect(command.values.kind).toBe('appeal_arguments');
    expect(command.values.scheduled_at).toBe('2026-10-10T09:02:03-06:00');
    expect(command.values.participants).toEqual([{ participant_id: participant.id, revision: 1 }]);
    expect(command.values.scheduling_basis.support).toEqual({
      document_id: state.activities.act.act.supports[0].document_id,
      version: state.activities.act.act.supports[0].version,
      digest: '8'.repeat(64),
    });
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    await page.screenshot({ path: info.outputPath('resource-hearing-review.png'), fullPage: true });
    await submitHearing(page);
    await expect(hearingEditor(page)).toHaveCount(0);
    await expect(hearingDetail(page)).toContainText('Sala propia del recurso');
    await expect(hearingDetail(page)).toContainText('09:02:03 / UTC-06:00');
    expect(state.submissions).toHaveLength(1);
    expect(state.committed.size).toBe(1);
    expect(state.activities.submissions).toHaveLength(0);
    expect(state.resources.facts.results.scheduling.submissions).toHaveLength(0);
    expect(
      state.resources.facts.results.scheduling.calls.some((call) =>
        call.path.includes('scheduling-context'),
      ),
    ).toBe(false);
    await expect(
      hearingDetail(page).getByRole('button', { name: 'Cancelar audiencia', exact: true }),
    ).toHaveCount(0);
    await activityPanel(page)
      .getByRole('button', {
        name: `Consultar v\u00ednculo ${command.association_id}`,
        exact: true,
      })
      .click();
    await expect(activityDetail(page)).toContainText('Sala propia del recurso');
    await expect(activityDetail(page)).toContainText('senalamiento-del-acto.pdf');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: info.outputPath('resource-hearing-created.png'),
      fullPage: true,
    });
  });
}

test('requires a declared basis before preparing and offers only already captured supports', async ({
  page,
}) => {
  const state = await setupResourceHearing(page);
  await openHearingForm(page, state);
  await fillHearingForm(page);
  const editor = hearingEditor(page);
  await editor.getByLabel('Base de senalamiento', { exact: true }).fill('');
  const select = editor.getByRole('combobox', {
    name: 'Soporte admitido del senalamiento',
    exact: true,
  });
  await expect(
    select.locator('option').filter({ hasText: 'senalamiento-del-acto.pdf' }),
  ).toHaveCount(0);
  await expect(editor.getByRole('button', { name: /Subir|Cargar archivo/ })).toHaveCount(0);
  await editor.getByRole('button', { name: 'Preparar audiencia y vinculo', exact: true }).click();
  await expect(
    editor.getByRole('region', { name: 'Revision de audiencia y vinculo', exact: true }),
  ).toHaveCount(0);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(0);
  await editor
    .getByLabel('Base de senalamiento', { exact: true })
    .fill('Senalamiento de recurso sin acto');
  await prepareHearing(page);
  const command = state.calls.findLast((call) => call.path.endsWith('/prepare')).body;
  expect(command.act).toBeNull();
  expect(command.values.participants).toEqual([]);
  const { document_id, version, digest } = state.activities.original.sources.supports[0];
  expect(command.values.scheduling_basis.support).toEqual({ document_id, version, digest });
});

test('accepts a changed head explicitly while retaining historical selections and entered values', async ({
  page,
}) => {
  const state = await setupResourceHearing(page);
  await openHearingForm(page, state);
  await fillHearingForm(page);
  await selectActSupportAndParticipant(page, state);
  await prepareHearing(page);
  const first = structuredClone(
    state.calls.findLast((call) => call.path.endsWith('/prepare')).body,
  );
  const correction = resourceCommandFixture('correct');
  correction.resource_id = state.activities.resource.id;
  correction.change.values = structuredClone(state.activities.resource.values);
  correction.change.values.title = 'Cabeza posterior a la revision de audiencia';
  expect(state.resources.commit(state.resources.prepare(correction)).revision).toBe(4);
  await submitHearing(page);
  const editor = hearingEditor(page);
  await expect(
    editor.getByRole('heading', { name: 'El registro cambi\u00f3', exact: true }),
  ).toBeVisible();
  expect(state.committed.size).toBe(0);
  await editor.getByRole('button', { name: 'Comparar con registro actual', exact: true }).click();
  await expect(editor).toContainText(correction.change.values.title);
  expect(state.calls.filter((call) => call.path.endsWith('/prepare'))).toHaveLength(1);
  await editor
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  await expect(editor.getByLabel('Sede o enlace', { exact: true })).toHaveValue(
    'Sala propia del recurso',
  );
  await prepareHearing(page);
  const next = state.calls.findLast((call) => call.path.endsWith('/prepare')).body;
  expect(next.expected_resource_revision).toBe(4);
  expect(next.resource).toEqual(first.resource);
  expect(next.act).toEqual(first.act);
  expect(next.values).toEqual(first.values);
  await submitHearing(page);
  await expect(hearingDetail(page)).toBeVisible();
  expect(state.submissions).toHaveLength(1);
});
