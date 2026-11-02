// Spanish messages for the findings about the request, the party statements,
// the manifest and the receipts of bilateral contract signing. The codes are
// the stable codes of crates/evidence-verify. Rationale:
// docs/adr/0086-bilateral-contract-signing.md.

export const contractMessages = {
  'contract.request_schema': 'La solicitud de firma del contrato no tiene el formato esperado.',
  'statement.position':
    'Una declaraci\u00f3n firmada del contrato no indica un orden de firma v\u00e1lido.',
  'contract.statement_mismatch':
    'Una declaraci\u00f3n firmada no coincide con la solicitud, el contrato o su orden de firma.',
  'contract.party_mismatch':
    'Una declaraci\u00f3n firmada no corresponde a la cuenta o al certificado que la solicitud fija para esa parte.',
  'contract.same_account': 'Las dos partes del contrato son la misma cuenta.',
  'contract.same_key': 'Las dos partes del contrato usan el mismo certificado o la misma clave.',
  'contract.root_mismatch':
    'Las partes del contrato no est\u00e1n vinculadas a la misma ra\u00edz de confianza del paquete.',
  'contract.chain_mismatch':
    'La firma del despacho no se hizo sobre la firma y el sello del Cliente que trae el paquete.',
  'contract.token_order':
    'Los sellos de tiempo del contrato no siguen el orden de firma: Cliente, contrato y cierre.',
  'contract.before_issue':
    'El sello de la firma del Cliente es anterior a la creaci\u00f3n de la solicitud.',
  'contract.after_expiry':
    'Una firma del contrato qued\u00f3 sellada despu\u00e9s de la vigencia de la solicitud.',
  'contract.counterparty_revoked':
    'El certificado del Cliente aparece revocado antes de la firma del despacho.',
  'contract.partial':
    'Es un acuse parcial: solo trae la firma del Cliente y no acredita un contrato firmado por ambas partes.',
  'manifest.contract_schema':
    'El manifiesto del contrato no tiene la forma exacta que Qadra escribe.',
  'receipt.contract_schema': 'El recibo del contrato no tiene la forma exacta que Qadra escribe.',
  'receipt.contract_partial_schema':
    'El acuse parcial del contrato no tiene la forma exacta que Qadra escribe.',
};

export const contractFallbacks = {
  contract: 'La firma bilateral del contrato tiene un problema.',
};
