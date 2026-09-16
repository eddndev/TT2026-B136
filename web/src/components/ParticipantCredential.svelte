<script>
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { download } from '../lib/documents.mjs';
  import { credentialSelection } from '../lib/participant-credential-selection.mjs';
  // Scope changes include case, session or selected identity; basis changes include values/revisions.
  export let scopeKey;
  export let basisKey;
  export let disabled = false;
  export let mandatory = false;
  export let pending = false;
  export let value = null;
  const selection = credentialSelection();
  let certificateInput;
  let signatureInput;
  $: selection.updateContext(scopeKey, basisKey);
  $: value = $selection;
  $: pending = $selection.busy;
  $: if (!$selection.certificate && certificateInput) certificateInput.value = '';
  $: if (!$selection.signature && signatureInput) signatureInput.value = '';

  // This ticket coordinates local completion; it is not an API draft or authentication token.
  export function beginPreparation() {
    return selection.beginPreparation();
  }
  export function acceptPreparation(ticket, statementBytes, receiptBytes) {
    return selection.acceptPreparation(ticket, statementBytes, receiptBytes);
  }
  export function failPreparation(ticket, message) {
    return selection.failPreparation(ticket, message);
  }
  onDestroy(() => {
    selection.dispose();
    value = null;
    pending = false;
  });
</script>

<fieldset class="case-offenses participant-credential" aria-busy={pending}>
  <legend>Certificado y firma de la declaraci&#243;n</legend>
  <p class="hint">
    Perfil de demostraci&#243;n: CA interna. Firma personal {mandatory ? 'obligatoria' : 'opcional'} para
    este tipo.
  </p>
  <p>
    Selecciona el certificado p&#250;blico de la persona representada. Esa persona firma la
    declaraci&#243;n exacta con su herramienta y conserva su clave privada fuera de Qadra.
  </p>
  <label
    >Certificado p&#250;blico PEM o DER<span class="participant-file-picker">
      <input
        aria-label="Certificado p&#250;blico PEM o DER"
        bind:this={certificateInput}
        type="file"
        accept=".pem,.der,.cer,.crt"
        disabled={disabled || pending || !$selection.available}
        onchange={(event) => selection.selectCertificate(event.currentTarget.files[0])}
      />
      <span class="secondary" aria-hidden="true"
        ><Icon name="upload" size={17} />Elegir certificado</span
      >
    </span></label
  >
  <small
    >Un certificado, hasta 16 KiB. No se admiten claves privadas, PFX ni contrase&#241;as.</small
  >
  {#if $selection.certificate}<p class="hint participant-provenance">
      {$selection.certificate.name} / {$selection.certificate.blob.size} bytes seleccionados
    </p>
    <button
      class="text-button"
      type="button"
      disabled={disabled || pending || !$selection.available}
      onclick={() => selection.selectCertificate(null)}>Quitar certificado</button
    >{/if}
  <p class="hint">
    El servidor comprobar&#225; el certificado y la firma al registrar. Seleccionar archivos no
    acredita identidad ni FIREL oficial.
  </p>
  {#if $selection.prepared}<section
      class="participant-comparison"
      aria-label="Declaraci&#243;n preparada"
    >
      <h3>Declaraci&#243;n preparada</h3>
      <p>Descarga los bytes exactos para firmar y el recibo para revisar los datos.</p>
      <div class="action-row">
        <button
          class="secondary"
          type="button"
          disabled={disabled || pending || !$selection.available}
          onclick={() => download($selection.prepared.statement, 'declaracion-participante.bin')}
          ><Icon name="download" size={17} />Descargar declaraci&#243;n binaria</button
        >
        <button
          class="secondary"
          type="button"
          disabled={disabled || pending || !$selection.available}
          onclick={() => download($selection.prepared.receipt, 'recibo-participante.json')}
          >Descargar recibo</button
        >
      </div>
      <label
        >Firma separada<span class="participant-file-picker">
          <input
            aria-label="Firma separada"
            bind:this={signatureInput}
            type="file"
            accept=".sig"
            disabled={disabled || pending || !$selection.available}
            onchange={(event) => selection.selectSignature(event.currentTarget.files[0])}
          />
          <span class="secondary" aria-hidden="true"
            ><Icon name="upload" size={17} />Elegir firma</span
          >
        </span></label
      >
      <small>RSA-3072, PKCS#1 v1.5 y SHA-256. Archivo binario de exactamente 384 bytes.</small>
      {#if $selection.signature}<p class="hint participant-provenance">
          {$selection.signature.name} / 384 bytes seleccionados
        </p>
        <button
          class="text-button"
          type="button"
          disabled={disabled || pending || !$selection.available}
          onclick={() => selection.selectSignature(null)}>Quitar firma seleccionada</button
        >{/if}
    </section>
  {:else}<p class="hint">
      Completa los datos del participante y prepara su declaraci&#243;n para descargarla.
    </p>{/if}
  <p class="hint">
    Cambiar los datos, la revisi&#243;n base o el certificado requiere preparar y firmar otra
    declaraci&#243;n. La preparaci&#243;n no guarda una ficha.
  </p>
  {#if pending}<p class="hint" role="status">
      Preparando materiales de la declaraci&#243;n...
    </p>{/if}
  {#if $selection.error}<p class="notice error" role="alert">{$selection.error}</p>{/if}
</fieldset>
