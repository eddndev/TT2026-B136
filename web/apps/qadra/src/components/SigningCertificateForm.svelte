<script>
  import { download } from '../lib/documents.mjs';
  import { signingStatementDownload } from '../lib/individual-signing-values.mjs';
  import SigningPublicFile from './SigningPublicFile.svelte';
  import SigningExternalInstructions from './SigningExternalInstructions.svelte';
  import SigningBindingDetails from './SigningBindingDetails.svelte';
  export let state, busy, certificate, confirmed;
  export let oncertificate, onprepare, onsignature, onsubmit, onwithdraw, ondiscard, onerror;
</script>

{#if state.action === 'withdraw'}<section class="card stack">
    <h2>Retirar mi vinculo</h2>
    <label
      ><input type="checkbox" bind:checked={confirmed} disabled={busy} />
      Confirmo retirar el vinculo para futuras firmas</label
    >
    <div class="action-row">
      <button class="primary" disabled={busy || !confirmed} onclick={onwithdraw}
        >Confirmar retiro</button
      >
      <button class="secondary" disabled={busy} onclick={ondiscard}>Cancelar retiro</button>
    </div>
  </section>
{:else}<section class="card stack">
    <h2>Registrar certificado documental</h2>
    <SigningPublicFile
      kind="certificate"
      label="Certificado publico PEM"
      disabled={busy}
      onvalue={oncertificate}
      {onerror}
    />
    <button class="secondary" disabled={busy || !certificate} onclick={onprepare}
      >Preparar vinculo</button
    >
    {#if state.prepared}<SigningBindingDetails value={state.prepared} />
      <button
        class="secondary"
        disabled={busy}
        onclick={() =>
          download(
            signingStatementDownload(state.prepared.statement_base64, 'binding'),
            'certificate-binding.bin',
          )}>Descargar declaracion del vinculo</button
      >
      <SigningExternalInstructions kind="binding" />
      <SigningPublicFile
        label="Firma externa del vinculo"
        disabled={busy}
        onvalue={onsignature}
        {onerror}
      />
      <label
        ><input
          type="checkbox"
          bind:checked={confirmed}
          disabled={busy || !state.signature}
        />Confirmo vincular este certificado a mi cuenta</label
      >
      <button class="primary" disabled={busy || !confirmed || !state.signature} onclick={onsubmit}
        >Registrar vinculo</button
      >
    {/if}
  </section>{/if}
