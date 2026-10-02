import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { typedDraftSetup, checkTypedRequests } from './session-typed-draft-fixtures.mjs';
import {
  createTyped,
  fillTypedOwn,
  prepareTypedFresh,
  confirmTyped,
  rawTyped,
} from './session-typed-draft-ui.mjs';

test.afterEach(async ({ page }) => checkTypedRequests(page));

async function downloadBytes(page, button) {
  const downloading = page.waitForEvent('download');
  await button.click();
  const stream = await (await downloading).createReadStream(),
    chunks = [];
  for await (const chunk of stream) chunks.push(chunk);
  return Buffer.concat(chunks);
}

test('public credential materials survive as inert evidence and a changed declaration requires a newly selected signature', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  await login(page);
  let modal = await createTyped(page);
  await fillTypedOwn(modal);
  await modal.getByLabel('Tipo de participante', { exact: true }).selectOption('control_judge');
  await modal.getByLabel('\u00d3rgano jurisdiccional', { exact: true }).fill('  Juzgado inicial  ');
  await modal
    .getByLabel('Certificado p\u00fablico PEM o DER', { exact: true })
    .setInputFiles('tests/fixtures/participant-public-certificate.pem');
  await expect(modal).toContainText('participant-public-certificate.pem');
  await prepareTypedFresh(page, modal, state);
  const originalStatement = await downloadBytes(
    page,
    modal.getByRole('button', {
      name: 'Descargar declaraci\u00f3n binaria',
      exact: true,
    }),
  );
  const originalReceipt = await downloadBytes(
    page,
    modal.getByRole('button', {
      name: 'Descargar recibo',
      exact: true,
    }),
  );
  const signature = Buffer.alloc(384, 7);
  await modal.getByLabel('Firma separada', { exact: true }).setInputFiles({
    name: 'original-public-signature.sig',
    mimeType: 'application/octet-stream',
    buffer: signature,
  });
  await expect(confirmTyped(modal)).toBeEnabled();
  const certificate = state.typedPreparations[0].values.certificate_base64;
  await expire(page, state, await modal.elementHandle());
  await login(page);
  modal = await createTyped(page);
  await expect(modal.getByLabel('Organizaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    rawTyped.organization,
  );
  await expect(modal).toContainText('participant-public-certificate.pem');
  await expect(confirmTyped(modal)).toBeDisabled();
  const retained = modal.getByRole('region', { name: 'Material de firma conservado', exact: true });
  await expect(retained).toContainText('original-public-signature.sig');
  expect(
    await downloadBytes(
      page,
      retained.getByRole('button', {
        name: 'Descargar declaraci\u00f3n anterior',
        exact: true,
      }),
    ),
  ).toEqual(originalStatement);
  expect(
    await downloadBytes(
      page,
      retained.getByRole('button', {
        name: 'Descargar firma anterior',
        exact: true,
      }),
    ),
  ).toEqual(signature);
  expect(
    await downloadBytes(
      page,
      retained.getByRole('button', {
        name: 'Descargar recibo anterior',
        exact: true,
      }),
    ),
  ).toEqual(originalReceipt);
  expect(state.typedReviews).toHaveLength(1);
  expect(state.typedPreparations).toHaveLength(1);
  expect(state.typedCommits).toEqual([]);
  await modal.getByLabel('\u00d3rgano jurisdiccional', { exact: true }).fill('  Otro juzgado  ');
  await prepareTypedFresh(page, modal, state);
  expect(state.typedPreparations[1].values.certificate_base64).toBe(certificate);
  const replacement = await downloadBytes(
    page,
    modal.getByRole('button', {
      name: 'Descargar declaraci\u00f3n binaria',
      exact: true,
    }),
  );
  expect(replacement).not.toEqual(originalStatement);
  await expect(confirmTyped(modal)).toBeDisabled();
  await expect(modal.getByLabel('Firma separada', { exact: true })).toHaveValue('');
  const nextSignature = Buffer.alloc(384, 9);
  await modal.getByLabel('Firma separada', { exact: true }).setInputFiles({
    name: 'replacement-public-signature.sig',
    mimeType: 'application/octet-stream',
    buffer: nextSignature,
  });
  state.nextTypedCommit = {};
  await confirmTyped(modal).click();
  await expect(modal).toBeHidden();
  expect(state.typedCommits).toHaveLength(1);
  expect(Buffer.from(state.typedCommits[0].values.signature_base64, 'base64')).toEqual(
    nextSignature,
  );
  expect(state.typedCommits[0].values.prepared.proposal.values.role.profile.court).toBe(
    'Otro juzgado',
  );
});
