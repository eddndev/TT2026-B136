import { test, expect } from '@playwright/test';
import {
  participantSetup,
  participant,
  participantId,
  otherParticipantId,
  directory,
  detail,
  openParticipant,
} from './participant-helpers.mjs';
import { caseId, otherCaseId, navigate, openSection } from './helpers.mjs';
const endpoint = `**/cases/${caseId}/participants/${participantId}`;
const unchangedReplacement = {
  expected_revision: participant.revision,
  display_name: participant.display_name,
  procedural_role: participant.procedural_role,
  organization: participant.organization,
  legal_status: participant.legal_status,
  directory_status: participant.directory_status,
};
async function settle(page) {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
}

for (const deniedPart of ['detail', 'history', 'edit']) {
  test(`${deniedPart} access denial clears participant data and any pending views`, async ({
    page,
  }) => {
    await participantSetup(page);
    await openParticipant(page);
    const deniedWrites = [];
    if (deniedPart === 'history') {
      await page.route('**/participants/*/history?*', (route) =>
        route.fulfill({ status: 403, json: { error: { code: 'forbidden' } } }),
      );
      await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
    } else {
      await page.route(endpoint, (route) => {
        if (deniedPart === 'edit') {
          if (route.request().method() !== 'PUT') return route.fallback();
          deniedWrites.push(route.request().postDataJSON());
        }
        return route.fulfill({ status: 404, json: { error: { code: 'participant_not_found' } } });
      });
      if (deniedPart === 'detail')
        await directory(page)
          .getByRole('button', { name: `Abrir ${participant.display_name}`, exact: true })
          .click();
      else {
        await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
        const modal = page.getByRole('dialog', { name: 'Editar participante', exact: true });
        await expect(modal.getByLabel('Nombre del participante', { exact: true })).toHaveValue(
          participant.display_name,
        );
        const submit = modal.getByRole('button', { name: 'Guardar participante', exact: true });
        await expect(submit).toBeEnabled();
        await submit.click();
      }
    }
    await expect(page.getByRole('alert')).toBeVisible();
    await expect(detail(page)).toHaveCount(0);
    await expect(directory(page).locator('.participant-row')).toHaveCount(0);
    await expect(page.getByRole('dialog')).toHaveCount(0);
    expect(deniedWrites).toEqual(deniedPart === 'edit' ? [unchangedReplacement] : []);
  });
}
