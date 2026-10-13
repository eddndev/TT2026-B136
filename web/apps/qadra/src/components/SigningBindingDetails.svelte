<script>
  export let value;
</script>

<div class="stack details">
  <strong>{value.account_label || value.account_id}</strong>
  <p>Cuenta: <code>{value.account_id}</code></p>
  <p>Vinculo: <code>{value.binding_id}</code></p>
  <p>Huella SHA-256 del certificado: <code>{value.certificate.fingerprint}</code></p>
  <p>Revision de confianza: {value.trust_revision}</p>
  <details class="technical-detail">
    <summary>Identidad y confianza del certificado</summary>
    <p>Emisor: {value.certificate.summary.issuer}</p>
    <p>Titular declarado: {value.certificate.summary.subject}</p>
    <p>
      La cuenta firmante se identifica en los bytes canonicos, no por el nombre del certificado.
    </p>
    <p>
      Vigencia: {new Date(value.certificate.summary.not_before_unix * 1000).toISOString()}
      a {new Date(value.certificate.summary.not_after_unix * 1000).toISOString()}
    </p>
    <p>Raiz: <code>{value.root_fingerprint}</code></p>
    <p>Despliegue: <code>{value.deployment_id}</code></p>
  </details>
  {#if value.registered_at}<p>Registrado: {value.registered_at}</p>{/if}
  {#if value.withdrawn_at}<p>Retirado: {value.withdrawn_at}</p>{/if}
</div>

<style>
  .details {
    overflow-wrap: anywhere;
  }
  code {
    word-break: break-all;
  }
</style>
