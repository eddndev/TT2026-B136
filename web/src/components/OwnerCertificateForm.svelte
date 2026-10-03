<script>
  import OwnerCertificateFields from './OwnerCertificateFields.svelte';
  export let prepared, signatureBase64, signatureName, selectedName, selected, command;
  export let latest, bindingId, ready, busy, mode, error;
  export let onload, onclose, oncertificate, onprepare, onsignature, onstatement;
  export let oncheck, onregister, onretry;
  $: blocked = latest !== null && latest.binding_id !== bindingId;
</script>

<section
  class="card stack owner-registration"
  aria-label="Registro de mi certificado"
  aria-busy={busy}
>
  <div class="section-heading">
    <h2>Registro de mi certificado</h2>
    <span class="badge info">Firma externa</span>
  </div>
  <p>
    Tu clave privada permanece fuera de Qadra. Selecciona solamente tu certificado p&#250;blico.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !ready}
    <p role="status">Comprobando acceso y estado de la cuenta...</p>
    {#if !busy}<button class="secondary" onclick={onload}>Volver a consultar</button>{/if}
    <button class="secondary" disabled={busy} onclick={onclose}>Cancelar registro</button>
  {:else}
    {#if blocked}<p class="notice identifier">
        La cuenta tiene otro v&#237;nculo sin retirar: {latest.binding_id}. Conserva estos bytes
        para revisarlos; no se pueden enviar.
      </p>{/if}
    <p class="hint">Identificador de este intento: <span class="identifier">{bindingId}</span></p>
    <OwnerCertificateFields
      {prepared}
      {signatureName}
      {selectedName}
      {selected}
      {busy}
      {mode}
      {blocked}
      {oncertificate}
      {onprepare}
      {onsignature}
      {onstatement}
    />
    {#if command && mode !== 'editing'}
      <p class="notice">
        El intento conserva su declaraci&#243;n, certificado y firma originales. Consulta su recibo
        antes de decidir otro env&#237;o. Cancelar descarta este borrador; no revierte un registro
        confirmado en el servidor.
      </p>
      <div class="action-row">
        <button class="secondary" disabled={busy} onclick={oncheck}>Comprobar registro</button>
        {#if mode === 'absent'}<button class="primary" disabled={busy || blocked} onclick={onretry}
            >Reenviar este registro</button
          >{/if}
      </div>
    {/if}
    <p class="hint">
      Preparar no registra el v&#237;nculo. El servidor decide la admisi&#243;n al registrar; este
      flujo no habilita acceso por certificado ni firma documentos.
    </p>
    <div class="action-row">
      <button
        class="primary"
        disabled={busy || !prepared || !signatureBase64 || mode !== 'editing' || blocked}
        onclick={onregister}>Registrar v&#237;nculo</button
      >
      <button class="secondary" disabled={busy} onclick={onclose}>Cancelar registro</button>
    </div>
  {/if}
</section>

<style>
  .owner-registration {
    margin-top: 24px;
  }
  .identifier {
    overflow-wrap: anywhere;
  }
</style>
