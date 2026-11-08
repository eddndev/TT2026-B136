<script>
  import Button from './Button.svelte';
  // The review step of a contract signature: the exact PDF, whose SHA-256 is
  // recomputed in this browser and compared with the signed request before
  // it is shown, the counterparty with its declared name and certificate
  // fingerprint, and the expiry of the request.
  // Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { declaredName, expiryText } from '../lib/contract-presentation.mjs';
  export let detail,
    position,
    pdfUrl = '',
    busy = false,
    onload,
    ondownload;
  $: counterparty = position === 1 ? detail.firm_certificate : detail.client_certificate;
</script>

<section class="stack contract-review" aria-label="Revisa el contrato">
  <h3>1. Revisa</h3>
  <dl class="contract-facts">
    <dt>Contrato</dt>
    <dd>{detail.title}, versi&#243;n {detail.version}</dd>
    <dt>{position === 1 ? 'Firma por el despacho' : 'Cliente'}</dt>
    <dd>
      {declaredName(counterparty)}
      <small class="fingerprint"
        >Huella del certificado: <code>{counterparty.fingerprint}</code></small
      >
    </dd>
    <dt>Vence</dt>
    <dd>{expiryText(detail.expires_at_unix)}</dd>
    <dt>Folio de la hoja de huella</dt>
    <dd>{detail.ceremony_reference}</dd>
  </dl>
  <p class="hint">
    El nombre es el declarado en el certificado; la huella es la que debe coincidir con la hoja
    firmada a mano.
  </p>
  {#if !pdfUrl}<Button variant="primary" disabled={busy} onclick={onload}
      >Cargar y comprobar el PDF</Button
    >
  {:else}
    <p class="notice success" role="status">
      PDF comprobado: su huella SHA-256 recalculada en este navegador coincide con la solicitud.
    </p>
    <code class="fingerprint">{detail.document_digest}</code>
    <!-- A sandboxed frame blocks the browser PDF viewer, so the frame shows
      only bytes whose digest was checked. See
      docs/adr/0086-bilateral-contract-signing.md. -->
    <iframe src={pdfUrl} title="PDF exacto del contrato"></iframe>
    <Button variant="secondary" disabled={busy} onclick={ondownload}>Descargar PDF</Button>
  {/if}
</section>

<style>
  .contract-review {
    min-width: 0;
  }
  .contract-facts {
    display: grid;
    gap: 4px;
    overflow-wrap: anywhere;
  }
  .contract-facts dt {
    font-weight: 600;
  }
  .contract-facts dd {
    margin: 0 0 8px;
  }
  .fingerprint {
    display: block;
    overflow-wrap: anywhere;
    word-break: break-all;
  }
  iframe {
    width: 100%;
    height: 420px;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
</style>
