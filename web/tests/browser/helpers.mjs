import { administration, overview, otherAdministration } from './case-administration-helpers.mjs';
import { expect } from '@playwright/test';

export const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
export const otherCaseId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
export const id = '78ac67b1-ab36-49ea-9b08-f951f341f081';
export const emptyMetadata = {
  metadata_revision: 0,
  document_type: null,
  classification: null,
  tags: [],
};
export const document = {
  case_id: caseId,
  id,
  name: 'contrato.pdf',
  version: 1,
  digest: 'a'.repeat(64),
  sealed: false,
};
export const caseRecord = {
  id: caseId,
  title: 'Defensa inicial',
  reference: 'NUC-123',
  created_by: id,
};
const component = { status: 'passed', detail: 'Verified by test backend' };
export const validReport = {
  id,
  version: 1,
  document_digest: document.digest,
  verdict: 'valid',
  integrity: component,
  signature: component,
  certificate: component,
  timestamp: component,
};

export async function setup(page, role = 'owner', initialDocuments = [document]) {
  const requests = [];
  let documents = initialDocuments.map((record) => ({
    ...record,
    current_metadata: record.current_metadata || emptyMetadata,
  }));
  let sequence = 0;
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    requests.push({
      path,
      method: request.method(),
      headers: request.headers(),
      body: request.postData(),
      search: url.search,
    });
    const user = { id: 'user', email: 'hatz@example.com', role };
    if (path.endsWith('/login'))
      return route.fulfill({ json: { challenge_token: 'challenge', expires_in_seconds: 300 } });
    if (/\/mfa\//.test(path))
      return route.fulfill({
        json: { access_token: `test-token-${++sequence}`, user, expires_in_seconds: 86400 },
      });
    if (path.endsWith('/me')) return route.fulfill({ json: user });
    if (path.endsWith('/logout')) return route.fulfill({ status: 204 });
    if (path.endsWith('/case-administrations'))
      return route.fulfill({
        json: {
          cases: [overview(administration()), overview(otherAdministration())],
          has_more: false,
          next_after_id: null,
        },
      });
    if (path.endsWith(`/cases/${caseId}/administration`))
      return route.fulfill({ json: administration() });
    if (path.endsWith(`/cases/${otherCaseId}/administration`))
      return route.fulfill({ json: otherAdministration() });
    if (path.endsWith('/administration/history'))
      return route.fulfill({
        json: {
          revisions: [administration().administration],
          has_more: false,
          next_before_revision: null,
        },
      });
    if (path.endsWith('/cases'))
      return route.fulfill({
        json:
          request.method() === 'POST'
            ? caseRecord
            : [caseRecord, { ...caseRecord, id: otherCaseId, title: 'Otro expediente' }],
      });
    if (path.endsWith(`/cases/${caseId}`)) return route.fulfill({ json: caseRecord });
    if (path.endsWith(`/cases/${otherCaseId}`))
      return route.fulfill({ json: { ...caseRecord, id: otherCaseId, title: 'Otro expediente' } });
    if (path.endsWith('/audit/verify'))
      return route.fulfill({ json: { valid: true, entries: 12, first_broken_index: null } });
    if (path.endsWith('/users') || path.endsWith('/bootstrap'))
      return route.fulfill({
        json: {
          user,
          totp_secret_base32: 'TESTSECRET',
          otpauth_uri: 'otpauth://totp/test',
          recovery_codes: ['recovery-one', 'recovery-two'],
        },
      });
    if (!path.startsWith(`/api/v1/cases/${caseId}/documents`))
      return route.fulfill({ status: 404 });
    if (path.endsWith('/metadata'))
      return route.fulfill({
        json: {
          case_id: caseId,
          id,
          ...(documents.find((item) => item.id === id)?.current_metadata || emptyMetadata),
        },
      });
    if (path.endsWith('/metadata/history'))
      return route.fulfill({
        json: { case_id: caseId, id, revisions: [], has_more: false, next_before_revision: null },
      });
    if (path.endsWith('/documents/with-metadata')) {
      const form = await new Response(request.postDataBuffer(), {
        headers: { 'Content-Type': request.headers()['content-type'] },
      }).formData();
      const metadata = JSON.parse(await form.get('metadata').text());
      const uploaded = {
        ...document,
        name: request.headers()['x-document-name'],
        current_metadata: { ...metadata, metadata_revision: 1 },
      };
      documents = [uploaded];
      return route.fulfill({ json: uploaded, status: 201 });
    }
    if (path.endsWith('/documents')) {
      if (request.method() === 'POST') {
        const uploaded = { ...document, name: request.headers()['x-document-name'] };
        documents = [uploaded];
        return route.fulfill({ json: uploaded, status: 201 });
      }
      const name = url.searchParams.get('name')?.toLowerCase() || '';
      const sealed = url.searchParams.get('sealed');
      const matches = documents.filter(
        (item) =>
          item.name.toLowerCase().includes(name) &&
          (sealed === null || item.sealed === (sealed === 'true')),
      );
      return route.fulfill({ json: { documents: matches, has_more: false } });
    }
    if (path.endsWith('/versions'))
      return route.fulfill({
        json: {
          versions: documents.length ? documents : [document],
          has_more: false,
          next_before_version: null,
          first_available_version: 1,
        },
      });
    if (/\/versions\/\d+$/.test(path))
      return route.fulfill({ json: documents.find((item) => item.id === id) || document });
    if (path.endsWith(`/documents/${id}`))
      return route.fulfill({ json: documents.find((item) => item.id === id) || document });
    if (path.endsWith('/seal')) {
      const sealed = { ...(documents.find((item) => item.id === id) || document), sealed: true };
      documents = [sealed];
      return route.fulfill({ json: sealed });
    }
    if (path.endsWith('/verify')) return route.fulfill({ json: validReport });
    if (path.endsWith('/evidence'))
      return route.fulfill({
        contentType: 'application/zip',
        body: 'test archive',
        headers: { 'X-Document-Id': id, 'X-Document-Version': '1' },
      });
    return route.fulfill({ status: 404 });
  });
  await page.goto('/');
  // Initial client hydration includes cold compilation by the development server.
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible({
    timeout: 20000,
  });
  return requests;
}

export async function navigate(page, name) {
  if (await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).isVisible())
    await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).click();
  await page.getByRole('navigation').getByRole('button', { name, exact: true }).click();
}

export async function selectCase(page, title = 'Defensa inicial') {
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: new RegExp(title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  if (await page.getByRole('link', { name: 'Documentos', exact: true }).count())
    await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  else await navigate(page, 'Documentos');
}

export async function login(page, recovery = false, openDocuments = true) {
  await page.getByLabel('Correo electr\u00f3nico').fill('hatz@example.com');
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  if (recovery)
    await page.getByRole('button', { name: 'Usar c\u00f3digo de recuperaci\u00f3n' }).click();
  await page
    .getByLabel(recovery ? 'C\u00f3digo de recuperaci\u00f3n' : 'C\u00f3digo de 6 d\u00edgitos', {
      exact: true,
    })
    .fill(recovery ? 'recovery-one' : '123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  if (openDocuments) await selectCase(page);
}

export async function openDocument(page) {
  await page.locator('.reference-panel').getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'contrato.pdf', exact: true })).toBeVisible();
}
