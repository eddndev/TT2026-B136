// Spanish messages for the findings about the evidence manifest of
// individual signing v2 and its acceptance receipt. The codes are the stable
// codes of crates/evidence-verify; tests/evidence-messages.test.mjs keeps the
// table complete. Rationale: the section "The evidence manifest" of
// docs/adr/0085-in-browser-evidence-validation.md.

export const manifestMessages = {
  'manifest.schema': 'El manifiesto de evidencia no tiene la forma esperada.',
  'manifest.artifacts_unsorted':
    'El manifiesto de evidencia no lista los archivos en el orden esperado.',
  'manifest.duplicate_artifact': 'El manifiesto de evidencia lista un archivo m\u00e1s de una vez.',
  'manifest.artifact_missing':
    'El paquete tiene un archivo que el manifiesto de evidencia no lista.',
  'manifest.artifact_unexpected':
    'El manifiesto de evidencia lista un archivo que el paquete no incluye.',
  'manifest.artifact_mismatch':
    'Un archivo del paquete no coincide con el manifiesto de evidencia, que guarda su resumen y su tama\u00f1o.',
  'manifest.role_mismatch':
    'El manifiesto de evidencia asigna a un archivo una funci\u00f3n que no le corresponde.',
  'manifest.document_mismatch':
    'El manifiesto de evidencia no corresponde a este documento o a su consentimiento.',
  'manifest.signer_mismatch':
    'Los datos de quien firm\u00f3 en el manifiesto de evidencia no coinciden con su consentimiento o sus archivos.',
  'manifest.signers_not_supported':
    'El manifiesto de evidencia incluye a m\u00e1s de una persona firmante; esta versi\u00f3n de Qadra solo revisa la firma de una persona.',
};

export const receiptMessages = {
  'receipt.schema': 'El recibo de aceptaci\u00f3n no tiene la forma esperada.',
  'receipt.seconds_mismatch':
    'El recibo de aceptaci\u00f3n indica un instante de aceptaci\u00f3n distinto del que guarda el resumen de la firma.',
  'receipt.manifest_mismatch':
    'El recibo de aceptaci\u00f3n se refiere a otro manifiesto de evidencia.',
  'receipt.operation_mismatch':
    'El recibo de aceptaci\u00f3n se refiere a otra operaci\u00f3n de firma.',
};

export const manifestFallbacks = {
  manifest: 'El manifiesto de evidencia tiene un problema.',
  receipt: 'El recibo de aceptaci\u00f3n tiene un problema.',
};
