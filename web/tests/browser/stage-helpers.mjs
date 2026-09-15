import { expect } from '@playwright/test';
import { setup, login, navigate, caseId, caseRecord, document } from './helpers.mjs';
import { administration, overview, profile } from './case-administration-helpers.mjs';
export { caseId, document };
export const initial = {
  kind: 'initial',
  case_id: caseId,
  stage_revision: 1,
  stage: 'investigation',
  administration_revision: 1,
  administration_digest: 'c'.repeat(64),
  recorded_at: '2026-09-01T12:00:00Z',
  recorded_by: { id: 'owner', email: 'original@example.com' },
};
export function entry(payload, revision = 2) {
  const { expected_revision, target, ...values } = payload;
  const adoption = !target;
  const refs = ['support', 'accusation', 'opening_order', 'receipt_support'].flatMap((key) =>
    values[key]
      ? [{ ...values[key], name: document.name, format: 'pdf', policy: 'pdf_docx_v1' }]
      : [],
  );
  return {
    ...initial,
    kind: 'change',
    stage_revision: revision,
    stage: target || values.stage,
    from_stage: adoption ? null : target === 'intermediate' ? 'investigation' : 'intermediate',
    values_digest: 'd'.repeat(64),
    values: { kind: adoption ? 'adoption' : `to_${target}`, ...values },
    supports: refs,
  };
}
export async function setupStages(
  page,
  { current = initial, role = 'owner', complete = true, closed = false } = {},
) {
  const requests = await setup(page, role);
  const detail = administration(
    caseRecord,
    1,
    complete ? profile : null,
    closed ? 'closed' : 'active',
  );
  detail.initial_stage = current && current.kind === 'initial' ? initial : null;
  await page.route('**/api/v1/case-administrations?*', (route) =>
    route.fulfill({
      json: {
        cases: [overview(detail)],
        has_more: false,
        next_after_id: null,
      },
    }),
  );
  await page.route(`**/api/v1/cases/${caseId}/administration`, (route) =>
    route.fulfill({ json: detail }),
  );
  const state = { current, history: current ? [current] : [], posts: [], requests: [] };
  await page.route(`**/api/v1/cases/${caseId}/stage**`, async (route) => {
    const request = route.request(),
      path = new URL(request.url()).pathname;
    state.requests.push({ path, method: request.method() });
    if (request.method() === 'POST') {
      const body = request.postDataJSON();
      state.posts.push(body);
      state.current = entry(body, (state.current?.stage_revision || 0) + 1);
      state.history.unshift(state.current);
      return route.fulfill({ status: 201, json: { case_id: caseId, current: state.current } });
    }
    return route.fulfill({
      json: path.endsWith('/history')
        ? { entries: state.history, has_more: false, next_before_revision: null }
        : { case_id: caseId, current: state.current },
    });
  });
  return { state, requests, detail };
}
export async function openStages(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Etapas', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Etapas del expediente', exact: true }),
  ).toBeVisible();
  await expect(page.locator('.case-stages')).toHaveAttribute('aria-busy', 'false');
}
export async function chooseSupport(page, label = 'Acusaci\u00f3n') {
  const field = page.getByRole('group', { name: label, exact: true });
  await field.getByRole('button', { name: 'Elegir documento', exact: true }).click();
  const picker = field.getByRole('region', { name: 'Seleccionar soporte exacto' });
  await picker.getByRole('button', { name: /contrato.pdf/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1/ }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
}
export async function fillDate(page, group) {
  const field = page.getByRole('group', { name: group, exact: true });
  await field.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  await field.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
}
