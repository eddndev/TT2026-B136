<script>
  export let value;
</script>

<div class="stack details">
  <p>Cuenta firmante: <code>{value.account_id}</code></p>
  <p>Expediente: <code>{value.case_id}</code></p>
  <p>Documento: <code>{value.document_id}</code>; version {value.version}</p>
  <p>Resumen SHA-256 del PDF: <code>{value.document_digest}</code></p>
  <p>Huella del certificado: <code>{value.certificate_fingerprint}</code></p>
  <p>Vinculo: <code>{value.binding_id}</code>; revision de confianza {value.trust_revision}</p>
  <p>Operacion: <code>{value.operation_id}</code></p>
  {#if value.accepted_at}<p>Aceptada: {value.accepted_at}</p>{/if}
</div>

<style>
  .details {
    overflow-wrap: anywhere;
  }
  code {
    word-break: break-all;
  }
</style>
