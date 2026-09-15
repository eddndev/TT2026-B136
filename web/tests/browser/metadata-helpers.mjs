import { expect } from '@playwright/test';
import { caseId, id, document, setup, login, openDocument } from './helpers.mjs';
export const values = {
  document_type: 'Escrito',
  classification: 'Civil',
  tags: ['acci\u00f3n, prueba'],
};
export const emptyMetadata = {
  metadata_revision: 0,
  document_type: null,
  classification: null,
  tags: [],
};
export async function metadataSetup(page, { role = 'owner', revision = 1, conflict = false } = {}) {
  const calls = await setup(page, role, [{ ...document, sealed: true }]);
  const state = {
    current: revision ? { ...values, metadata_revision: revision } : { ...emptyMetadata },
    records: [{ ...document, sealed: true }],
    history: [],
  };
  function historical(current) {
    return {
      ...current,
      changed_by: { id, email: 'historic@example.com' },
      changed_at: '2026-09-15T10:30:00Z',
      metadata_digest: 'b'.repeat(64),
    };
  }
  state.history = revision ? [historical(state.current)] : [];
  const response = () => ({ case_id: caseId, id, ...state.current });
  await page.route(`**/cases/${caseId}/documents?**`, (route) => {
    const query = new URL(route.request().url()).searchParams;
    calls.push({ path: `/api/v1/cases/${caseId}/documents`, search: `?${query}`, method: 'GET' });
    const matches = ['document_type', 'classification', 'tag'].every(
      (key) =>
        !query.has(key) ||
        (key === 'tag'
          ? state.current.tags.includes(query.get(key))
          : state.current[key] === query.get(key)),
    );
    return route.fulfill({
      json: {
        documents: matches ? [{ ...state.records.at(-1), current_metadata: state.current }] : [],
        has_more: false,
      },
    });
  });
  await page.route(`**/cases/${caseId}/documents/**`, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const suffix = url.pathname.split(`/documents/${id}`)[1];
    calls.push({
      path: url.pathname,
      method: request.method(),
      body: request.postData(),
      search: url.search,
      headers: request.headers(),
    });
    if (suffix === '/metadata') {
      if (request.method() === 'PUT') {
        if (conflict) {
          conflict = false;
          state.current = {
            ...state.current,
            metadata_revision: state.current.metadata_revision + 1,
            classification: 'Otro cambio',
          };
          state.history.unshift(historical(state.current));
          return route.fulfill({
            status: 409,
            json: { error: { code: 'document_metadata_conflict' } },
          });
        }
        const body = request.postDataJSON();
        state.current = {
          document_type: body.document_type,
          classification: body.classification,
          tags: body.tags,
          metadata_revision: state.current.metadata_revision + 1,
        };
        state.history.unshift(historical(state.current));
      }
      return route.fulfill({ json: response() });
    }
    if (suffix === '/metadata/history')
      return route.fulfill({
        json: {
          case_id: caseId,
          id,
          revisions: state.history,
          has_more: false,
          next_before_revision: null,
        },
      });
    if (suffix === '')
      return route.fulfill({ json: { ...state.records.at(-1), current_metadata: state.current } });
    return route.fallback();
  });
  await login(page);
  if (role !== 'client') {
    await openDocument(page);
    await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
    await expect(
      page.getByRole('button', { name: 'Actualizar historial', exact: true }),
    ).toBeEnabled();
  }
  return { calls, state, response, historical };
}
