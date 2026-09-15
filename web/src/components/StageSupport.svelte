<script>
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import UploadDocument from './UploadDocument.svelte';
  export let api,
    caseId,
    label,
    value,
    ondenied,
    disabled = false,
    optional = false,
    pending = false;
  let picking = false,
    pickerBusy = false,
    uploadBusy = false,
    uploader,
    uploaded = false;
  $: pending = pickerBusy || uploadBusy;
  function selected(record) {
    value = record;
    picking = false;
    uploaded = false;
  }
  function loaded(record) {
    if (record.case_id !== caseId) return;
    value = { ...record, uploaded: true };
    uploaded = true;
    picking = false;
  }
</script>

<fieldset class="case-offenses stage-support" aria-label={label}>
  <legend>{label}{optional ? ' (opcional)' : ''}</legend>
  {#if value}<StageSupportSummary record={value} />{/if}
  {#if uploaded}<p class="notice" role="status">
      Documento guardado: {value.name} / versi&#243;n {value.version}. Falta registrar la etapa.
    </p>{/if}
  <p class="hint">
    Selecciona una versi&#243;n exacta de un PDF o DOCX. El servidor validar&#225; el soporte al
    registrar la etapa.
  </p>
  <div class="action-row">
    <button
      class="secondary"
      type="button"
      disabled={disabled || pending}
      onclick={() => (picking = true)}>Elegir documento</button
    >
    <button
      class="secondary"
      type="button"
      disabled={disabled || pending}
      onclick={() => uploader.open()}>Cargar soporte</button
    >
    {#if optional && value}<button
        class="text-button"
        type="button"
        disabled={disabled || pending}
        onclick={() => {
          value = null;
          uploaded = false;
        }}>Quitar soporte opcional</button
      >{/if}
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
