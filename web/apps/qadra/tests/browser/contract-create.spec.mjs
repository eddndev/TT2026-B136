import { test, expect } from '@playwright/test';
import { setup, login, openDocument, caseId, document } from './helpers.mjs';
import { contractId } from './contract-helpers.mjs';
import { inputs } from '../contract-fixture.mjs';

const operationReceipt = (kind, revision, operationId) => ({
  contract_id: contractId,
  case_id: caseId,
  event: {
    revision,
    kind,
    request_id: null,
    actor_id: 'user',
    actor_label: 'hatz@example.com',
    reason: null,
    at: '2026-11-02T00:00:00Z',
    operation_id: operationId,
  },
  request_statement_base64: null,
  signature: null,
  manifest_sha256: null,
});

async function contractRoutes(page) {
  const state = { writes: [], refuse: true };
  await page.route(`**/api/v1/cases/${caseId}/contracts**`, (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (request.method() === 'GET')
      return route.fulfill({
        json: {
          contracts: [],
          firm_signers: [
            {
              account_id: inputs.firm_account_id,
              email: 'litigante@example.test',
              role: 'litigator',
            },
          ],
        },
      });
    const body = request.postDataJSON();
    state.writes.push({ path, body });
    if (path.endsWith('/contracts'))
      return route.fulfill({
        status: 201,
        json: operationReceipt('created', 1, body.operation_id),
      });
    if (state.refuse) {
      state.refuse = false;
      return route.fulfill({
        status: 422,
        json: {
          error: {
            code: 'contract_party_ineligible',
            message: 'not ready',
            blockers: ['client_not_ready'],
          },
        },
      });
    }
    return route.fulfill({
      status: 201,
      json: operationReceipt('requested', 2, body.operation_id),
    });
  });
  return state;
}

test('a litigator creates a contract from an unsealed PDF and sees readiness blockers first', async ({
  page,
}) => {
  await setup(page, 'litigator');
  const state = await contractRoutes(page);
  await login(page);
  await openDocument(page);
  await page.getByRole('button', { name: 'Crear contrato', exact: true }).click();
  const form = page.getByRole('region', { name: 'Crear contrato' });
  await expect(form.getByLabel('T\u00edtulo del contrato')).toHaveValue('contrato');
  await form.getByLabel('T\u00edtulo del contrato').fill('Contrato de servicios');
  await form.getByRole('button', { name: 'Crear contrato', exact: true }).click();
  await expect(form.getByText('Contrato creado en borrador.')).toBeVisible();
  expect(state.writes[0].body).toMatchObject({
    document_id: document.id,
    title: 'Contrato de servicios',
  });
  await expect(form.getByLabel('Vigencia en d\u00edas')).toHaveValue('7');
  await form.getByLabel('Correo de la cuenta del Cliente').fill('cliente@example.test');
  await form.getByLabel('Firma por el despacho').selectOption(inputs.firm_account_id);
  await form.getByLabel('Folio de la hoja de huella firmada').fill('HF-0001');
  await form.getByRole('button', { name: 'Solicitar firmas', exact: true }).click();
  const blockers = form.getByRole('region', { name: 'Lista de preparacion' });
  await expect(blockers).toContainText('La cuenta del Cliente no est\u00e1 lista');
  await expect(blockers).toContainText('su vigencia no ha empezado');
  expect(state.writes).toHaveLength(2);
  expect(state.writes[1].body).toMatchObject({
    expected_revision: 1,
    version: 1,
    client_email: 'cliente@example.test',
    firm_signer_id: inputs.firm_account_id,
    ttl_days: 7,
    ceremony_reference: 'HF-0001',
  });
  await form.getByRole('button', { name: 'Solicitar firmas', exact: true }).click();
  await expect(
    page.getByText('Firmas solicitadas. Consulta su avance en Contratos del expediente.'),
  ).toBeVisible();
  expect(state.writes).toHaveLength(3);
  await expect(page.getByRole('button', { name: 'Crear contrato', exact: true })).toHaveCount(0);
});

for (const [name, role, record] of [
  ['a paralegal', 'paralegal', document],
  ['a sealed PDF', 'litigator', { ...document, sealed: true }],
  ['a document that is not a PDF', 'owner', { ...document, name: 'acta.docx' }],
  ['a version under contract', 'owner', { ...document, contract_state: 'en_firma' }],
])
  test(`contract creation is not offered for ${name}`, async ({ page }) => {
    await setup(page, role, [record]);
    await login(page);
    await openDocument(page).catch(() => {});
    await expect(page.getByRole('heading', { name: record.name, exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Crear contrato', exact: true })).toHaveCount(0);
    if (record.contract_state) {
      await expect(page.getByText('En contrato').first()).toBeVisible();
      await expect(page.getByRole('button', { name: 'Sellar documento' })).toHaveCount(0);
    }
  });
