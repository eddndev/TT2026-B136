import { caseRecord, otherCaseId, id } from './helpers.mjs';
export const profile = {
  nuc: 'NUC-PENAL',
  nuc_authority: 'Fiscalia registrada',
  judicial_case_number: 'CJ-PENAL',
  judicial_authority: 'Organo registrado',
  offenses: ['Descripcion inicial'],
  general_information: 'Primera linea\nSegunda linea',
  complementary_identifiers: null,
};
export function administration(record = caseRecord, revision = 1, penal = null, status = 'active') {
  return {
    id: record.id,
    created_by: record.created_by,
    created_at: '2026-09-15T00:00:00Z',
    administration: {
      case_id: record.id,
      title: record.title,
      reference: record.reference,
      revision,
      administrative_status: status,
      profile: penal,
      values_digest: revision ? 'c'.repeat(64) : null,
      changed_at: revision ? '2026-09-15T00:00:00Z' : null,
      changed_by: revision ? { id, email: 'owner@example.com' } : null,
    },
    initial_stage: null,
  };
}
export function overview(detail) {
  const { administration: a } = detail;
  return {
    id: detail.id,
    created_by: detail.created_by,
    created_at: detail.created_at,
    title: a.title,
    reference: a.reference,
    revision: a.revision,
    administrative_status: a.administrative_status,
    profile_status: a.profile ? 'complete' : 'pending',
    penal_identifiers: a.profile
      ? { nuc: a.profile.nuc, judicial_case_number: a.profile.judicial_case_number }
      : null,
    initial_stage: detail.initial_stage?.stage || null,
  };
}
export async function fillPenal(page, title = 'Alta penal') {
  await page.getByLabel('T\u00edtulo del expediente', { exact: true }).fill(title);
  await page.getByLabel('Referencia interna', { exact: true }).fill('REF-PENAL');
  for (const [label, value] of [
    ['NUC', 'NUC-PENAL'],
    ['Autoridad emisora del NUC', 'Fiscalia registrada'],
    ['Carpeta judicial', 'CJ-PENAL'],
    ['\u00d3rgano emisor de la carpeta', 'Organo registrado'],
  ])
    await page.getByLabel(label, { exact: true }).fill(value);
  await page
    .getByLabel('Nueva descripci\u00f3n de delito', { exact: true })
    .fill('Descripcion inicial');
  await page
    .getByLabel('Informaci\u00f3n general (opcional)', { exact: true })
    .fill('Primera linea\nSegunda linea');
}
export const otherAdministration = () =>
  administration({ ...caseRecord, id: otherCaseId, title: 'Otro expediente' });
