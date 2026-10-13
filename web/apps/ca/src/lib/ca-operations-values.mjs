const hashPattern = /^[a-f0-9]{64}$/;
const serialPattern = /^[1-9A-F][0-9A-F]{0,39}$/;
const uuidPattern = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
const encoder = new TextEncoder();

export const emptyDraft = () => ({ csr_pem: '', purpose: 'partner', holder: '', reference: '', criterion: '' });
export const emptyPage = () => ({ items: [], total: 0, offset: 0, limit: 20, loaded: false });
export const purposeLabel = (purpose) => ({ partner: 'Partner',
  'document-signing': 'Firma documental', 'time-stamping': 'Sellado de tiempo' })[purpose];
export const publishedLabel = (status) => ({ unknown: 'Desconocida',
  'not-revoked': 'No incluido en la CRL de Qadra', pending: 'Pendiente de publicar en Qadra',
  revoked: 'Revocado en la CRL de Qadra' })[status];
export const dateLabel = (value) => new Date(value * 1000).toLocaleString('es-MX');
export const validId = (value) => typeof value === 'string' && uuidPattern.test(value)
  && value !== '00000000-0000-0000-0000-000000000000';
const hash = (value) => typeof value === 'string' && hashPattern.test(value);
const serial = (value) => typeof value === 'string' && serialPattern.test(value);
const purpose = (value) => ['partner', 'document-signing'].includes(value);
const text = (value, max = 500) => typeof value === 'string' && value.trim().length > 0
  && encoder.encode(value).length <= max && !/[\x00-\x1f\x7f]/.test(value);
const timestamp = (value) => Number.isSafeInteger(value);
const publicPem = (value, kind) => typeof value === 'string' && value.length <= 16384
  && /^[\x00-\x7f]+$/.test(value) && !/PRIVATE KEY/.test(value)
  && value.trim().startsWith(`-----BEGIN ${kind}-----`)
  && value.trim().endsWith(`-----END ${kind}-----`);
const pick = (value, names) => Object.fromEntries(names.map((name) => [name, value[name]]));
const requireValue = (valid) => { if (!valid) throw new Error('Invalid public CA response'); };

export function validDraft(value) {
  return publicPem(value.csr_pem, 'CERTIFICATE REQUEST') && purpose(value.purpose)
    && ['holder', 'reference', 'criterion'].every((name) => text(value[name]));
}

export function previewValue(value, expectedPurpose) {
  requireValue(value && value.purpose === expectedPurpose && purpose(value.purpose)
    && ['csr_sha256', 'spki_sha256', 'root_fingerprint'].every((name) => hash(value[name]))
    && text(value.subject, 4096));
  return pick(value, ['csr_sha256', 'spki_sha256', 'root_fingerprint', 'subject', 'purpose']);
}

export function certificateValue(value, expected) {
  requireValue(value && serial(value.serial) && hash(value.fingerprint)
    && value.serial === expected.serial && value.fingerprint === expected.fingerprint
    && publicPem(value.certificate_pem, 'CERTIFICATE'));
  return pick(value, ['serial', 'fingerprint', 'certificate_pem']);
}

export function receiptValue(value, operationId, expected = null) {
  requireValue(value && value.operation_id === operationId && validId(value.operator_id)
    && serial(value.serial) && hash(value.fingerprint) && purpose(value.purpose)
    && timestamp(value.recorded_at) && value.recorded_at > 0
    && ['holder', 'reference', 'criterion'].every((name) => text(value[name]))
    && (!expected || ['purpose', 'holder', 'reference', 'criterion']
      .every((name) => value[name] === expected[name])));
  certificateValue(value, value);
  return pick(value, ['operation_id', 'serial', 'fingerprint', 'purpose', 'certificate_pem',
    'recorded_at', 'operator_id', 'holder', 'reference', 'criterion']);
}

function inventoryItem(value) {
  requireValue(value && serial(value.serial) && hash(value.fingerprint) && hash(value.root_fingerprint)
    && text(value.subject, 4096) && (purpose(value.purpose) || value.purpose === 'time-stamping')
    && timestamp(value.not_before) && timestamp(value.not_after) && value.not_before <= value.not_after
    && ['not-yet-valid', 'valid', 'expired'].includes(value.validity)
    && ['issued', 'revoked', 'expired'].includes(value.local_status)
    && ['unknown', 'not-revoked', 'pending', 'revoked'].includes(value.published_status));
  return pick(value, ['serial', 'fingerprint', 'root_fingerprint', 'subject', 'purpose',
    'not_before', 'not_after', 'validity', 'local_status', 'published_status']);
}

function historyItem(value) {
  requireValue(value && validId(value.operation_id)
    && ['pending', 'completed'].includes(value.state) && timestamp(value.recorded_at)
    && value.recorded_at > 0 && validId(value.operator_id));
  const common = ['operation_id', 'kind', 'state', 'recorded_at', 'operator_id'];
  if (value.kind === 'revoke') {
    requireValue(serial(value.serial) && hash(value.fingerprint) && text(value.reason));
    return pick(value, [...common, 'serial', 'fingerprint', 'reason']);
  }
  if (value.kind === 'publication') {
    requireValue(Number.isSafeInteger(value.expected_revision) && value.expected_revision > 0
      && value.expected_revision <= 0xffffffff
      && (value.state === 'pending' ? value.revision == null
        : value.revision === value.expected_revision + 1));
    return { ...pick(value, [...common, 'expected_revision']), revision: value.revision ?? null };
  }
  requireValue(value.kind === 'issue' && ['holder', 'reference', 'criterion'].every((name) => text(value[name]))
    && (value.state === 'pending' ? value.serial === null && value.fingerprint === null
      : serial(value.serial) && hash(value.fingerprint)));
  return pick(value, [...common, 'holder', 'reference', 'criterion', 'serial', 'fingerprint']);
}

export function pageValue(value, kind, offset) {
  requireValue(value && Array.isArray(value.items) && value.items.length <= 20
    && value.offset === offset && value.limit === 20 && Number.isSafeInteger(value.total)
    && value.total >= 0 && (!value.items.length || value.total >= offset + value.items.length));
  return { items: value.items.map(kind === 'inventory' ? inventoryItem : historyItem),
    total: value.total, offset, limit: 20, loaded: true };
}
