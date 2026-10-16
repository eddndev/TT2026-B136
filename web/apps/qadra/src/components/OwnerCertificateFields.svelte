<script>
  import LocalSignature from './LocalSignature.svelte';
  import { ownerStatementDownload } from '../lib/owner-certificate-files.mjs';
  export let prepared, signatureName, selectedName, selected, busy, mode, blocked;
  export let oncertificate, onprepare, onsignature, onstatement;
  export let localContext, onlocalsignature, onlocalinvalidate;
  $: localInput = prepared ? ownerStatementDownload(prepared.statement_base64) : null;
</script>

<label
  >Certificado p&#250;blico PEM
  <input
    type="file"
    accept=".pem,.crt,.cer"
    disabled={busy || !!prepared || mode !== 'editing'}
    onchange={oncertificate}
  />
</label>
<small
  >Un bloque CERTIFICATE, hasta 16 KiB en total. No adjuntes claves, PFX ni contrase&#241;as.</small
>
{#if selectedName}<p class="hint">{selectedName}</p>{/if}
<div class="action-row">
  <button
    class="secondary"
    disabled={busy || !selected || !!prepared || blocked}
    onclick={onprepare}>Preparar v&#237;nculo</button
  >
</div>
{#if prepared}
  <div class="prepared stack">
    <h3>Declaraci&#243;n preparada</h3>
    <p>Firma la declaraci&#243;n exacta para vincular este certificado a tu cuenta.</p>
    <dl>
      <dt>Certificado</dt>
      <dd>{prepared.certificate.summary.subject}</dd>
      <dt>Huella SHA-256</dt>
      <dd>{prepared.certificate.fingerprint}</dd>
      <dt>Revisi&#243;n de cuenta</dt>
      <dd>{prepared.account_revision}</dd>
      <dt>Revisi&#243;n de confianza</dt>
      <dd>{prepared.trust_revision}</dd>
    </dl>
    {#if mode === 'editing'}{#key localContext}<LocalSignature
          contextKey={localContext}
          input={localInput}
          certificateDerBase64={prepared.certificate.der_base64}
          certificateFingerprint={prepared.certificate.fingerprint}
          purpose="vinculo Owner"
          disabled={busy || blocked}
          onsignature={onlocalsignature}
          oninvalidate={onlocalinvalidate}
        />{/key}{/if}
    <details open>
      <summary>Usar una firma externa</summary>
      <p>Firma los 150 bytes exactos con RSA-3072, PKCS#1 v1.5 y SHA-256.</p>
      <div class="action-row">
        <button class="secondary" disabled={busy} onclick={onstatement}
          >Descargar bytes para firma</button
        >
      </div>
      <label
        >Firma separada<input
          type="file"
          accept=".sig"
          disabled={busy || mode !== 'editing'}
          onchange={onsignature}
        /></label
      >
      <small>Archivo binario de exactamente 384 bytes.</small>
    </details>
    {#if signatureName}<p class="hint">{signatureName}</p>{/if}
  </div>
{/if}

<style>
  .prepared {
    background: #f8fafc;
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 20px;
  }
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
  input[type='file'] {
    max-width: 100%;
  }
</style>
