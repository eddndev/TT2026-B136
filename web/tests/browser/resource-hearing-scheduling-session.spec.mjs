import { test, expect } from '@playwright/test';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  setupHearingSession,
  login,
  openHearingScope,
  holdHearingAuthority,
  resumeHearing,
} from './resource-hearing-scheduling-session-helpers.mjs';
import {
  openHearingForm,
  fillHearingForm,
  selectActSupportAndParticipant,
  prepareHearing,
  submitHearing,
  hearingEditor,
  hearingDetail,
  activityPanel,
  clone,
} from './resource-hearing-scheduling-helpers.mjs';

test('same-account expiry restores raw fields and exact selections only after fresh authority', async ({
  page,
}) => {
  const state = await setupHearingSession(page);
  await openHearingForm(page, state);
  await fillHearingForm(page);
  await selectActSupportAndParticipant(page, state);
  const raw = '  Senalamiento pendiente\n conservar declaracion  ';
  await hearingEditor(page).getByLabel('Base de senalamiento', { exact: true }).fill(raw);
  await hearingEditor(page).getByLabel('Desfase UTC', { exact: true }).fill('-0');
  await expire(page, state, await hearingEditor(page).elementHandle());
  await login(page, false, false);
  await openHearingScope(page, state);
  const gate = holdHearingAuthority(state);
  try {
    await activityPanel(page)
      .getByRole('button', { name: 'Retomar borrador de audiencia', exact: true })
      .click();
    await expect.poll(() => gate.entered).toBe(true);
    expect(
      await page.locator('input,textarea').evaluateAll((nodes) => nodes.map((node) => node.value)),
    ).not.toContain(raw);
    gate.release();
    state.gate = null;
    await expect(
      hearingEditor(page).getByLabel('Base de senalamiento', { exact: true }),
    ).toHaveValue(raw);
    await expect(hearingEditor(page).getByLabel('Desfase UTC', { exact: true })).toHaveValue('-0');
    await expect(hearingEditor(page)).toContainText('Interposicion original declarada');
    await expect(hearingEditor(page)).toContainText('senalamiento-del-acto.pdf');
    await expect(
      hearingEditor(page).getByRole('button', {
        name: 'Confirmar audiencia y vinculo',
        exact: true,
      }),
    ).toHaveCount(0);
    expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(0);
    const stored = await page.evaluate(() =>
      JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)]),
    );
    expect(stored).not.toContain(raw);
  } finally {
    gate.release();
    state.gate = null;
  }
});

test('uncertain submission survives reauthentication without automatic resend or new preparation', async ({
  page,
}) => {
  const state = await setupHearingSession(page);
  let sent;
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/submit') || sent) return false;
    sent = clone(call.body);
    state.submissions.push(sent);
    await route.abort('failed');
    return true;
  };
  await openHearingForm(page, state);
  await fillHearingForm(page);
  await prepareHearing(page);
  await submitHearing(page);
  await expect(
    hearingEditor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  await expire(page, state, await hearingEditor(page).elementHandle());
  await login(page, false, false);
  await openHearingScope(page, state);
  await resumeHearing(page);
  await expect(
    hearingEditor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  await expect(
    hearingEditor(page).getByRole('button', { name: 'Reintentar envio exacto', exact: true }),
  ).toHaveCount(0);
  expect(state.submissions).toEqual([sent]);
  await hearingEditor(page)
    .getByRole('button', { name: 'Consultar resultado', exact: true })
    .click();
  await expect(hearingEditor(page)).toContainText('El resultado sigue incierto');
  expect(state.submissions).toEqual([sent]);
  await hearingEditor(page)
    .getByRole('button', { name: 'Reintentar envio exacto', exact: true })
    .click();
  await expect(hearingDetail(page)).toBeVisible();
  expect(state.submissions).toEqual([sent, sent]);
  expect(state.calls.filter((call) => call.path.endsWith('/prepare'))).toHaveLength(1);
});

for (const transition of ['different account', 'explicit logout']) {
  test(`${transition} discards suspended own hearing drafts`, async ({ page }) => {
    const state = await setupHearingSession(page);
    await openHearingForm(page, state);
    await fillHearingForm(page);
    await expire(page, state, await hearingEditor(page).elementHandle());
    if (transition === 'different account') await signInOther(page);
    else await login(page, false, false);
    await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
    await login(page, false, false);
    await openHearingScope(page, state);
    await expect(
      activityPanel(page).getByRole('button', {
        name: 'Retomar borrador de audiencia',
        exact: true,
      }),
    ).toHaveCount(0);
    await activityPanel(page)
      .getByRole('button', { name: 'Crear audiencia de recurso', exact: true })
      .click();
    await expect(hearingEditor(page).getByLabel('Sede o enlace', { exact: true })).toHaveValue('');
    await expect(
      hearingEditor(page).getByLabel('Base de senalamiento', { exact: true }),
    ).toHaveValue('');
    expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(0);
  });
}
