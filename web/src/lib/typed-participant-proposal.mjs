import { subjectValues, roleValues, validateSupportSet } from './typed-participant-values.mjs';
import { needsCredential } from './typed-participant-fields.mjs';
const reference = (record) => ({
  id: record.id,
  revision: record.revision,
  values_digest: record.values_digest,
});
export function proposalRequest({ selected, subject, role, original, natural }, certificate) {
  const identity = selected
    ? { operation: 'keep', reference: reference(selected) }
    : { operation: 'create', values: subjectValues(subject) };
  const values = roleValues(role);
  if (
    values.profile.kind === 'trial_court' &&
    (selected?.values.kind || subject.kind) !== 'institutional_body'
  )
    throw new Error('El tribunal requiere una identidad de \u00f3rgano institucional.');
  if (!['trial_court', 'other'].includes(values.profile.kind) && !natural)
    throw new Error('Este tipo requiere la identidad de una persona.');
  if (needsCredential(values.profile.kind) && !certificate)
    throw new Error('Este tipo requiere seleccionar el certificado p\u00fablico personal.');
  validateSupportSet(identity, selected?.values, values);
  return {
    subject: identity,
    participant: original
      ? { operation: 'existing', id: original.id, expected_revision: original.revision }
      : { operation: 'create' },
    role: values,
    certificate_base64: certificate,
  };
}

export async function resolveCandidate(row, { api, manualApi, original, selected }) {
  const record =
    row.reference.kind === 'subject'
      ? await api.subject(row.reference.id)
      : await manualApi.get(row.reference.id);
  if (row.reference.kind === 'subject') {
    if (original?.profile && original.subject.id !== record.id)
      throw new Error('La ficha tipificada conserva su identidad vinculada.');
    return { original, selected: record };
  }
  if (record.profile)
    throw new Error('La ficha ya fue tipificada. Consulta sus datos actuales en el directorio.');
  return { original: record, selected };
}
