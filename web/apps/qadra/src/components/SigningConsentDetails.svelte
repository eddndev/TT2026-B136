<script>
  export let value;
</script>

<div class="stack details">
  <p>Cuenta firmante: <code>{value.account_id}</code></p>
  <p>Expediente: <code>{value.case_id}</code></p>
  <p>Documento: <code>{value.document_id}</code>; versi&#243;n {value.version}</p>
  <p>Resumen SHA-256 del PDF: <code>{value.document_digest}</code></p>
  <p>Huella del certificado: <code>{value.certificate_fingerprint}</code></p>
  <p>
    V&#237;nculo: <code>{value.binding_id}</code>; revisi&#243;n de confianza {value.trust_revision}
  </p>
  <p>Operaci&#243;n: <code>{value.operation_id}</code></p>
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
