<script>
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import UploadDocument from './UploadDocument.svelte';
  export let api,
    caseId,
    draft,
    ondenied,
    onreviewed = () => {},
    disabled = false,
    pending = false;
  let picking = false,
    pickerBusy = false,
    uploadBusy = false,
    uploader;
  $: pending = pickerBusy || uploadBusy;
  function select(record) {
    if (record.case_id !== caseId) return;
    draft.support = record;
    picking = false;
    onreviewed();
  }
</script>

<fieldset class="case-offenses hearing-antecedent" {disabled}>
  <legend>Antecedente de condena declarado</legend>
  <label
    >Declaraci&#243;n del antecedente<textarea
      rows="3"
      bind:value={draft.statement}
      disabled={disabled || pending}></textarea></label
  >
  <p class="hint">
    Declara el antecedente y su soporte exacto. La etapa Juicio por s&#237; sola no demuestra una
    condena.
  </p>
  {#if draft.support}<StageSupportSummary record={draft.support} />{/if}
  {#if draft.support?.uploaded}<p class="notice" role="status">
      El soporte se guard&#243;. Falta confirmar la audiencia.
    </p>{/if}
  <div class="action-row">
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => (picking = true)}>Elegir soporte del antecedente</button
    >
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => uploader.open()}>Cargar soporte del antecedente</button
    >
  </div>
  {#if picking}<StagePicker
      {api}
      {caseId}
      {ondenied}
      {disabled}
      bind:busy={pickerBusy}
      onselected={select}
      oncancel={() => (picking = false)}
    />{/if}
</fieldset>
<UploadDocument
  {api}
  {ondenied}
  {disabled}
  bind:this={uploader}
  bind:busy={uploadBusy}
  onuploaded={(record) => select({ ...record, uploaded: true })}
/>
