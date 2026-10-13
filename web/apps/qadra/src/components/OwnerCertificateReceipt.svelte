<script>
  import { download } from '../lib/documents.mjs';
  export let receipt,
    busy = false,
    onread,
    onwithdraw,
    canWithdraw = true,
    admitted;
  function exportReceipt() {
    if (admitted())
      download(
        new Blob([JSON.stringify(receipt, null, 2)], { type: 'application/json' }),
        `recibo-vinculo-${receipt.binding_id}.json`,
      );
  }
</script>

<section class="card stack owner-receipt" aria-label="V&#237;nculo de mi cuenta">
  <div class="section-heading">
    <h2>V&#237;nculo de mi cuenta</h2>
    <span class:info={!receipt.withdrawal} class="badge"
      >{receipt.withdrawal ? 'Retirado' : 'Sin retirar'}</span
    >
  </div>
  <p class="identifier">{receipt.binding_id}</p>
  <p>
    Este recibo conserva lo comprobado al registrar. Estar sin retirar no acredita la vigencia
    actual del certificado.
  </p>
  <dl>
    <dt>Certificado</dt>
    <dd>{receipt.registration.certificate.summary.subject}</dd>
    <dt>Huella SHA-256</dt>
    <dd class="identifier">{receipt.registration.certificate.fingerprint}</dd>
    <dt>Registro</dt>
    <dd>{receipt.registration.registered_at}</dd>
    <dt>Revisi&#243;n del v&#237;nculo</dt>
    <dd>{receipt.revision}</dd>
    {#if receipt.withdrawal}<dt>Retiro</dt>
      <dd>{receipt.withdrawal.withdrawn_at}</dd>{/if}
  </dl>
  <div class="action-row">
    <button class="secondary" disabled={busy} onclick={onread}>Consultar recibo</button>
    <button class="secondary" disabled={busy} onclick={exportReceipt}>Descargar recibo</button>
    {#if !receipt.withdrawal && canWithdraw}<button
        class="secondary"
        disabled={busy}
        onclick={onwithdraw}>Retirar v&#237;nculo</button
      >{/if}
  </div>
</section>

<style>
  .owner-receipt {
    margin-top: 24px;
  }
  .identifier,
  dd {
    overflow-wrap: anywhere;
  }
  dl {
    margin: 0;
    display: grid;
    gap: 6px;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin: 0 0 8px;
  }
</style>
