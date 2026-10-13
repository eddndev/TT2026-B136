import { test, expect } from '@playwright/test';
import {
  setupHearings,
  openHearings,
  hearingEditor,
  fillHearing,
  confirmHearing,
} from './hearing-helpers.mjs';
import { hearingPrepared, hearingRecord } from '../fixtures/hearings.mjs';

for (const delayedAbsence of [false, true]) {
  test(`lost response reconciles the exact receipt without another write${delayedAbsence ? ' after a provisional 404' : ''}`, async ({
    page,
  }) => {
    const { state } = await setupHearings(page);
    let absent = delayedAbsence;
    state.handle = async (route, call) => {
      if (call.method === 'POST' && call.path.endsWith('/hearings')) {
        state.submissions.push(call.body.command);
        state.commit(hearingPrepared(call.body.command));
        await route.abort('failed');
        return true;
      }
      if (absent && call.path.includes('/revisions/')) {
        await route.fulfill({ status: 404, json: { error: { code: 'hearing_not_found' } } });
        return true;
      }
      return false;
    };
    await openHearings(page);
    await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
    await fillHearing(page);
    await confirmHearing(page);
    const editor = hearingEditor(page);
    await expect(editor).toContainText('Resultado del env\u00edo pendiente de confirmar');
    await expect(
      editor.getByRole('button', { name: 'Revisar registro', exact: true }),
    ).toBeDisabled();
    await editor
      .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
      .click();
    if (delayedAbsence) {
      await expect(editor).toContainText('El resultado sigue incierto');
      await expect(editor.getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
        'Sala privada declarada',
      );
      absent = false;
      await editor
        .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
        .click();
    }
    await expect(editor).toHaveCount(0);
    expect(state.submissions).toHaveLength(1);
    const exact = state.calls.filter((call) => call.path.includes('/revisions/'));
    expect(exact.every((call) => call.path.endsWith('/revisions/1'))).toBeTruthy();
  });
}

test('a different receipt at the target revision stays a conflict and preserves the draft', async ({
  page,
}) => {
  const { state } = await setupHearings(page);
  state.handle = async (route, call) => {
    if (call.method !== 'POST' || !call.path.endsWith('/hearings')) return false;
    state.submissions.push(call.body.command);
    const prepared = hearingPrepared(call.body.command);
    prepared.actor_id = 'other-actor';
    state.commit(prepared);
    await route.abort('failed');
    return true;
  };
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  await confirmHearing(page);
  const editor = hearingEditor(page);
  await editor
    .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
    .click();
  await expect(editor).toContainText('pertenece a otro env\u00edo');
  await expect(editor.getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
    'Sala privada declarada',
  );
  await expect(
    editor.getByRole('button', { name: 'Revisar registro', exact: true }),
  ).toBeDisabled();
  expect(state.submissions).toHaveLength(1);
});

test('an advanced head does not hide the matching original submission revision', async ({
  page,
}) => {
  const { state } = await setupHearings(page);
  state.handle = async (route, call) => {
    if (call.method !== 'POST' || !call.path.endsWith('/hearings')) return false;
    const prepared = hearingPrepared(call.body.command),
      record = state.commit(prepared);
    state.submissions.push(call.body.command);
    state.records.get(record.id).push({
      ...structuredClone(record),
      revision: 2,
      receipt: { ...record.receipt, operation_id: 'other' },
    });
    await route.abort('failed');
    return true;
  };
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  await confirmHearing(page);
  await hearingEditor(page)
    .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
    .click();
  await expect(hearingEditor(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  await expect(
    page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
  ).toContainText('Revisi\u00f3n 1');
  await expect(
    page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
  ).toContainText('consultada exactamente');
});
