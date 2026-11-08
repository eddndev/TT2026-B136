// Controlled HTTP for the bilateral contract interface. Request details and
// prepared statements come from the shared contract vectors, patched with the
// certificate of a key generated for the test, so the page decodes and
// checks the exact CTRTREQ1 and CTRTSIG1 bytes and signs them with the real
// local cryptography module.
import { expect } from '@playwright/test';
import { setup, caseId as staffCaseId } from './helpers.mjs';
import { absoluteSession } from '../fixtures/session.mjs';
import { binding as bindingFixture } from '../individual-signing-fixture.mjs';
import {
  inputs,
  pdf,
  requestDetail,
  preparedStatement,
  signatureReceipt,
} from '../contract-fixture.mjs';

export const contractId = '2d7c1a90-5b8e-4f61-9a3c-7e2b4d6f8a10';
const uuid = (value) => Buffer.from(value.replaceAll('-', ''), 'hex');

function bindingFor(user, material) {
  const value = bindingFixture(true);
  const bytes = Buffer.from(value.statement_base64, 'base64');
  uuid(user.id).copy(bytes, 62);
  Buffer.from(material.fingerprint, 'hex').copy(bytes, 110);
  value.statement_base64 = bytes.toString('base64');
  value.account_id = user.id;
  value.certificate = {
    ...value.certificate,
    der_base64: material.certificate.toString('base64'),
    fingerprint: material.fingerprint,
  };
  return value;
}

export async function contractSetup(page, role, material, options = {}) {
  const position = role === 'client' ? 1 : 2;
  const user = {
    id: position === 1 ? inputs.client_account_id : inputs.firm_account_id,
    email: position === 1 ? 'cliente@example.test' : 'litigante@example.test',
    name: position === 1 ? 'Cliente Ejemplo' : 'Litigante Ejemplo',
    role,
  };
  const expiresAt = Math.floor(Date.now() / 1000) + 7 * 86400;
  const state = {
    user,
    writes: [],
    detail: requestDetail({
      status: options.status ?? (position === 1 ? 'awaiting_client' : 'awaiting_firm'),
      clientSignedAt: position === 2 ? '2026-11-02T01:00:00Z' : null,
      expiresAt,
      ...(position === 1
        ? { clientFingerprint: material.fingerprint }
        : { firmFingerprint: material.fingerprint }),
    }),
    prepared: null,
    refuseSign: options.refuseSign ?? null,
    binding: options.binding === null ? null : bindingFor(user, material),
  };
  if (options.caseRequester) state.detail.requester_id = user.id;
  await setup(page, role, []);
  await page.route('**/api/v1/auth/**', (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith('/me')) return route.fulfill({ json: user });
    if (path.includes('/mfa/'))
      return route.fulfill({ json: absoluteSession(user, 'contract-token') });
    return route.fallback();
  });
  await page.route('**/api/v1/identity/signing-certificates/current', (route) =>
    route.fulfill({ json: { binding: state.binding } }),
  );
  await page.route('**/api/v1/contract-requests**', (route) => contractRequests(route, state));
  await page.route(`**/api/v1/cases/${staffCaseId}/contracts**`, (route) =>
    caseContracts(route, state),
  );
  return state;
}

async function contractRequests(route, state) {
  const request = route.request();
  const path = new URL(request.url()).pathname;
  const detail = state.detail;
  if (request.method() === 'POST') state.writes.push({ path, body: request.postDataJSON() });
  if (path === '/api/v1/contract-requests')
    return route.fulfill({ json: { requests: [{ ...detail, statement_base64: undefined }] } });
  if (!path.startsWith(`/api/v1/contract-requests/${detail.request_id}`))
    return route.fulfill({ status: 404, json: { error: { code: 'contract_not_found' } } });
  if (path.endsWith('/content'))
    return route.fulfill({
      body: pdf,
      contentType: 'application/octet-stream',
      headers: { 'X-Document-Digest': detail.document_digest },
    });
  if (path.includes('/evidence/'))
    return route.fulfill({ body: 'zip', contentType: 'application/zip' });
  if (path.endsWith('/prepare')) {
    const position = state.user.id === detail.client_id ? 1 : 2;
    state.prepared = preparedStatement(position, detail, request.postDataJSON().operation_id);
    return route.fulfill({ json: state.prepared });
  }
  if (path.endsWith('/sign') && state.refuseSign) {
    const { status, code } = state.refuseSign;
    state.refuseSign = null;
    return route.fulfill({ status, json: { error: { code, message: code } } });
  }
  if (path.endsWith('/sign')) {
    const body = request.postDataJSON();
    const kind = state.prepared.position === 1 ? 'client_signed' : 'completed';
    const receipt = signatureReceipt(detail, state.prepared, body.signature_base64, kind);
    state.detail = {
      ...detail,
      status: kind === 'completed' ? 'completed' : 'awaiting_firm',
      client_signed_at: detail.client_signed_at ?? '2026-11-02T01:00:00Z',
      completed_at: kind === 'completed' ? '2026-11-02T02:00:00Z' : null,
      revision: detail.revision + 1,
    };
    return route.fulfill({ json: receipt });
  }
  if (path.endsWith('/close')) {
    const outcome = request.postDataJSON().outcome;
    const kind = { reject: 'rejected', decline: 'declined', cancel: 'cancelled' }[outcome];
    state.detail = { ...detail, status: kind, revision: detail.revision + 1 };
    return route.fulfill({ json: { contract_id: contractId, event: { kind } } });
  }
  return route.fulfill({ json: detail });
}

function contractView(state) {
  const { statement_base64, client_certificate, firm_certificate, signatures, ...row } =
    state.detail;
  return {
    contract_id: contractId,
    case_id: staffCaseId,
    document_id: row.document_id,
    title: row.title,
    created_by: row.requester_id,
    created_by_email: 'litigante@example.test',
    created_at: '2026-11-02T00:00:00Z',
    state:
      row.status === 'awaiting_client' || row.status === 'awaiting_firm' ? 'en_firma' : 'borrador',
    revision: row.revision,
    derived_at_unix: Math.floor(Date.now() / 1000),
    events: [
      { revision: 1, kind: 'created', actor_label: 'litigante@example.test', at: row.created_at },
      { revision: 2, kind: 'requested', actor_label: 'litigante@example.test', at: row.created_at },
    ],
    requests: [row],
  };
}

function caseContracts(route, state) {
  const path = new URL(route.request().url()).pathname;
  if (path.endsWith('/contracts'))
    return route.fulfill({
      json: {
        contracts: [contractView(state)],
        firm_signers: [
          {
            account_id: inputs.firm_account_id,
            email: 'litigante@example.test',
            role: 'litigator',
          },
        ],
      },
    });
  return route.fulfill({ json: contractView(state) });
}

export async function clientLogin(page) {
  await page.getByLabel('Correo electr\u00f3nico').fill('cliente@example.test');
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('C\u00f3digo de 6 d\u00edgitos', { exact: true }).fill('123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
}

export async function openRequest(page, heading) {
  await expect(page.getByRole('heading', { name: heading, exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Abrir Contrato de prestacion de servicios' }).click();
  const region = page.getByRole('region', { name: 'Solicitud de firma' });
  await expect(
    region.getByRole('heading', { name: 'Contrato de prestacion de servicios' }),
  ).toBeVisible();
  return region;
}
