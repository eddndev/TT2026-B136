<script>
  import Button from './Button.svelte';
  import OwnerCertificateFields from './OwnerCertificateFields.svelte';
  export let prepared, signatureBase64, signatureName, selectedName, selected, command;
  export let latest, bindingId, ready, busy, mode, error;
  export let onload, onclose, oncertificate, onprepare, onsignature, onstatement;
  export let oncheck, onregister, onretry;
  export let localContext, onlocalsignature, onlocalinvalidate;
  $: blocked = latest !== null && latest.binding_id !== bindingId;
</script>

<section
  class="card stack owner-registration"
  aria-label="Registro de mi certificado"
  aria-busy={busy}
>
  <div class="section-heading">
    <h2>Registro de mi certificado</h2>
    <span class="badge info">Firma personal</span>
  </div>
  <p>
    Tu clave privada permanece en tu equipo. Selecciona tu certificado p&#250;blico para preparar el
    v&#237;nculo.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !ready}
    <p role="status">Comprobando acceso y estado de la cuenta...</p>
    {#if !busy}<Button variant="secondary" onclick={onload}>Volver a consultar</Button>{/if}
    <Button variant="secondary" disabled={busy} onclick={onclose}>Cancelar registro</Button>
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
      {localContext}
      {onlocalsignature}
      {onlocalinvalidate}
    />
    {#if command && mode !== 'editing'}
      <p class="notice">
        El intento conserva su declaraci&#243;n, certificado y firma originales. Consulta su recibo
        antes de decidir otro env&#237;o. Cancelar descarta este borrador; no revierte un registro
        confirmado en el servidor.
      </p>
      <div class="action-row">
        <Button variant="secondary" disabled={busy} onclick={oncheck}>Comprobar registro</Button>
        {#if mode === 'absent'}<Button
            variant="primary"
            disabled={busy || blocked}
            onclick={onretry}>Reenviar este registro</Button
          >{/if}
      </div>
    {/if}
    <p class="hint">
      Preparar no registra el v&#237;nculo. El servidor decide la admisi&#243;n al registrar; este
      flujo no habilita acceso por certificado ni firma documentos.
    </p>
    <div class="action-row">
      <Button
        variant="primary"
        disabled={busy || !prepared || !signatureBase64 || mode !== 'editing' || blocked}
        onclick={onregister}>Registrar v&#237;nculo</Button
      >
      <Button variant="secondary" disabled={busy} onclick={onclose}>Cancelar registro</Button>
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
