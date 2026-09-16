<script>
  import StagePicker from './StagePicker.svelte';
  import UploadDocument from './UploadDocument.svelte';
  export let api,
    caseId,
    label,
    value = null,
    ondenied,
    disabled = false,
    pending = false;
  let picking = false,
    pickerBusy = false,
    uploadBusy = false,
    uploader,
    uploaded = false;
  $: pending = pickerBusy || uploadBusy;
  function selected(record) {
    value = {
      document_id: record.id,
      version: record.version,
      digest: record.digest,
      locator: '',
      name: record.name,
    };
    picking = false;
  }
  function loaded(record) {
    if (record.case_id !== caseId) return;
    selected(record);
    uploaded = true;
  }
</script>

<fieldset class="case-offenses stage-support" aria-label={label}>
  <legend>{label}</legend>
  {#if value}<p>{value.name || 'Documento seleccionado'} / versi&#243;n {value.version}</p>
    <p class="hint participant-provenance">SHA-256: {value.digest}</p>
    <label
      >P&#225;gina o secci&#243;n<input
        bind:value={value.locator}
        disabled={disabled || pending}
      /></label
    >
  {/if}
  {#if uploaded}<p class="notice" role="status">
      Documento guardado. El registro del participante a&#250;n requiere confirmaci&#243;n; un
      rechazo conservar&#225; esta carga.
    </p>{/if}
  <p class="hint">
    Selecciona la versi&#243;n exacta y localiza la informaci&#243;n dentro de ella. M&#225;ximo dos
    versiones documentales distintas por registro.
  </p>
  <div class="action-row">
    <button
      class="secondary"
      type="button"
      disabled={disabled || pending}
      onclick={() => (picking = true)}>Seleccionar documento</button
    >
    <button
      class="text-button"
      type="button"
      disabled={disabled || pending}
      onclick={() => uploader.open()}>Cargar soporte</button
    >
  </div>
  {#if picking}<StagePicker
      {api}
      {caseId}
      {ondenied}
      {disabled}
      bind:busy={pickerBusy}
      onselected={selected}
      oncancel={() => (picking = false)}
    />{/if}
</fieldset>
<UploadDocument
  {api}
  {ondenied}
  {disabled}
  bind:this={uploader}
  bind:busy={uploadBusy}
  onuploaded={loaded}
/>
