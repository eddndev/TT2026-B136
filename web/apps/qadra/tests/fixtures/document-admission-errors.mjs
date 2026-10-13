export const admissionRejections = [
  {
    code: 'document_format_unsupported',
    status: 422,
    explanation: [/formato/i, /no (?:es |est\u00e1 )?(?:admitido|permitido|compatible)/i],
  },
  {
    code: 'document_format_invalid',
    status: 422,
    explanation: [
      /(?:archivo|contenido|estructura)/i,
      /(?:inv\u00e1lid|no (?:es |est\u00e1 )?v\u00e1lid|da\u00f1ad|corrupt)/i,
    ],
  },
  {
    code: 'document_validation_limit',
    status: 422,
    explanation: [/validaci\u00f3n/i, /(?:l\u00edmite|presupuesto)/i],
  },
  {
    code: 'document_validator_unavailable',
    status: 503,
    explanation: [
      /(?:validador|validaci\u00f3n)/i,
      /(?:no (?:est\u00e1 )?disponible|temporal|no se pudo)/i,
    ],
  },
];
export const privateParserDetail = 'PRIVATE /srv/secret-parser/input-failure';
export const rejectionPayload = (code) => ({ error: { code, message: privateParserDetail } });
