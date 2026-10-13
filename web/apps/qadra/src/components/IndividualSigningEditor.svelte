<script>
  import { download } from '../lib/documents.mjs';
  import { signingStatementDownload } from '../lib/individual-signing-values.mjs';
  import SigningPublicFile from './SigningPublicFile.svelte';
  import SigningExternalInstructions from './SigningExternalInstructions.svelte';
  import SigningConsentDetails from './SigningConsentDetails.svelte';
  export let state, pdfUrl, busy, reviewed, confirmed;
  export let ondownload, onpdfsignature, onconsentsignature, onprepare, onsubmit, oncancel, onerror;
</script>

<section class="card stack personal-signing">
  <h3>Firma personal de esta version</h3>
  <p>Revisa el PDF exacto y firma cada archivo descargado en tu equipo.</p>
  <button class="secondary" disabled={busy || !!state.prepared} onclick={ondownload}
    >Descargar PDF para firmar</button
  >
  {#if pdfUrl}
    <iframe src={pdfUrl} title="PDF exacto de esta version"></iframe>
    <label
      ><input type="checkbox" bind:checked={reviewed} disabled={busy || !!state.prepared} />He
      revisado este PDF y confirmo su contenido</label
    >
    <SigningExternalInstructions kind="pdf" />
    <SigningPublicFile
      label="Firma externa del PDF"
      disabled={busy || !reviewed || !!state.prepared}
      onvalue={onpdfsignature}
      {onerror}
    />
    {#if !state.prepared}<button
        class="secondary"
        disabled={busy || !reviewed || !state.pdfSignature}
        onclick={onprepare}>Preparar consentimiento</button
      >{/if}
  {/if}
  {#if state.prepared}
    <SigningConsentDetails value={state.prepared} />
    <button
      class="secondary"
      disabled={busy}
      onclick={() =>
        download(
          signingStatementDownload(state.prepared.consent_base64, 'consent'),
          'document-consent.bin',
        )}>Descargar consentimiento</button
    >
    <SigningExternalInstructions kind="consent" />
    <SigningPublicFile
      label="Firma externa del consentimiento"
      disabled={busy}
      onvalue={onconsentsignature}
      {onerror}
    />
    <label
      ><input
        type="checkbox"
        bind:checked={confirmed}
        disabled={busy || !state.consentSignature}
      />Confirmo firmar esta version con mi certificado</label
    >
    <button
      class="primary"
      disabled={busy || !reviewed || !confirmed || !state.consentSignature}
      onclick={onsubmit}>Enviar firma personal</button
    >
  {/if}
  <button class="secondary" disabled={busy} onclick={oncancel}>Cancelar firma personal</button>
</section>

<style>
  .personal-signing {
    min-width: 0;
  }
  iframe {
    width: 100%;
    height: 360px;
    border: 1px solid var(--border, #d7dce0);
    border-radius: 8px;
  }
</style>
