// Plain-language labels and gating of the bilateral contract interface.
// A version under contract is never presented as sealed or pending seal: the
// seal and individual signature refuse it with contract_version_frozen.
// Rationale: docs/adr/0086-bilateral-contract-signing.md.

export const CLIENT_CONSENT_TEXT =
  'Tu firma solo formaliza el contrato si el despacho firma antes de la fecha indicada. ' +
  'Una vez firmada no podr\u00e1s retirarla. Solo la revocaci\u00f3n de tu certificado, ' +
  'publicada por la autoridad certificadora antes de que el despacho firme, impide que se complete.';
export const CLIENT_SIGNED_NOTICE =
  'Tu firma qued\u00f3 registrada; el contrato a\u00fan no est\u00e1 formalizado.';
export const PARTIAL_NOTICE =
  'El acuse parcial demuestra tu firma; no acredita un contrato formalizado.';

const STATES = {
  borrador: { label: 'Borrador', tone: 'neutral' },
  en_firma: { label: 'En firma', tone: 'warning' },
  firmado: { label: 'Firmado por ambas partes', tone: 'success' },
  abandonado: { label: 'Abandonado', tone: 'neutral' },
};
const STATUSES = {
  awaiting_client: { label: 'Espera la firma del Cliente', tone: 'warning', open: true },
  awaiting_firm: { label: 'Espera la firma del despacho', tone: 'warning', open: true },
  completed: { label: 'Formalizado', tone: 'success', open: false },
  rejected: { label: 'Rechazado por el Cliente', tone: 'danger', open: false },
  declined: { label: 'Declinado por el despacho', tone: 'danger', open: false },
  cancelled: { label: 'Cancelado', tone: 'neutral', open: false },
  expired: { label: 'Vencido', tone: 'neutral', open: false },
};
const EVENTS = {
  created: 'Contrato creado',
  requested: 'Firmas solicitadas',
  client_signed: 'Firm\u00f3 el Cliente',
  completed: 'Firm\u00f3 el despacho; contrato formalizado',
  rejected: 'El Cliente rechaz\u00f3',
  declined: 'El despacho declin\u00f3',
  cancelled: 'Solicitud cancelada',
  expired: 'Solicitud vencida',
  abandoned: 'Contrato abandonado',
};
export const BLOCKERS = {
  client_not_ready:
    'La cuenta del Cliente no est\u00e1 lista: debe existir por invitaci\u00f3n, estar activa y tener su firma vinculada en Mi firma.',
  firm_signer_not_ready:
    'Quien firma por el despacho no tiene una firma documental vinculada y vigente, o no es miembro del expediente.',
  document_not_ready:
    'La versi\u00f3n elegida no puede contratarse: debe ser la versi\u00f3n actual, un PDF y no estar sellada.',
};

export const contractStateView = (state) =>
  STATES[state] ?? { label: 'Estado por confirmar', tone: 'neutral' };
export const requestStatusView = (status) =>
  STATUSES[status] ?? { label: 'Estado por confirmar', tone: 'neutral', open: false };
export const eventLabel = (kind) => EVENTS[kind] ?? kind;
export const blockerMessages = (blockers) =>
  (Array.isArray(blockers) ? blockers : []).map((code) => BLOCKERS[code] ?? code);

export function underContract(document) {
  return (
    document?.sealed !== true && ['en_firma', 'firmado'].includes(document?.contract_state ?? '')
  );
}

export function contractDocumentStatus(document) {
  if (!underContract(document)) return null;
  return document.contract_state === 'firmado'
    ? { label: 'Contrato firmado', tone: 'success', key: 'contract' }
    : { label: 'En contrato', tone: 'info', key: 'contract' };
}

export function contractable(document, role) {
  return (
    ['owner', 'litigator'].includes(role) &&
    document?.sealed === false &&
    !document.contract_state &&
    /\.pdf$/i.test(document.name ?? '')
  );
}

export function expiryText(unixSeconds) {
  return new Date(unixSeconds * 1000).toLocaleString('es-MX', {
    dateStyle: 'long',
    timeStyle: 'short',
  });
}

export function declaredName(certificate) {
  const subject = certificate?.summary?.subject ?? '';
  const match = /(?:^|[,/]\s*)CN\s*=\s*([^,/]+)/.exec(subject);
  return match ? match[1].trim() : subject || 'Sin nombre declarado';
}

// What the account may do with one request: its position, if any, and the
// downloads and closings the access rules allow.
export function requestActions(detail, user, nowSeconds = Date.now() / 1000) {
  const status = detail.status;
  const live = requestStatusView(status).open && detail.expires_at_unix > nowSeconds;
  const isClient = user.id === detail.client_id;
  const isFirm = user.id === detail.firm_signer_id;
  const staff = ['owner', 'litigator', 'paralegal'].includes(user.role) && !isClient;
  return {
    position: isClient ? 1 : isFirm ? 2 : null,
    sign:
      live &&
      ((isClient && status === 'awaiting_client') || (isFirm && status === 'awaiting_firm')),
    reject: live && isClient && status === 'awaiting_client',
    decline: live && isFirm && status === 'awaiting_firm',
    cancel:
      live &&
      (user.role === 'owner' || (user.role === 'litigator' && user.id === detail.requester_id)),
    partial: detail.client_signed_at !== null && (isClient || isFirm || staff),
    final: status === 'completed',
    signedByMe: isClient
      ? detail.client_signed_at !== null
      : isFirm && detail.firm_signed_at !== null,
  };
}

// The Client signs only after the exact PDF was loaded and its digest
// recomputed, the plain-language consequences were confirmed and a
// statement for this operation was prepared and signed.
export function clientSignReady({ pdfChecked, confirmed, prepared, signature }) {
  return pdfChecked === true && confirmed === true && !!prepared && !!signature;
}

export function certificateExpired(binding, nowSeconds = Date.now() / 1000) {
  const notAfter = binding?.certificate?.summary?.not_after_unix;
  return Number.isSafeInteger(notAfter) && notAfter <= nowSeconds;
}

export function certificateRenewalDue(binding, nowSeconds = Date.now() / 1000) {
  const notAfter = binding?.certificate?.summary?.not_after_unix;
  return (
    Number.isSafeInteger(notAfter) && notAfter > nowSeconds && notAfter - nowSeconds <= 30 * 86400
  );
}
