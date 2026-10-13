<script>
  import HearingSupport from './HearingSupport.svelte';
  export let api,
    caseId,
    draft,
    ondenied,
    onreviewed = () => {},
    disabled = false,
    pending = false,
    draftContext = null,
    ondiscard = () => {};
</script>

<fieldset class="case-offenses hearing-antecedent">
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
  <HearingSupport
    {api}
    {caseId}
    {ondenied}
    {disabled}
    {draftContext}
    {ondiscard}
    label="Soporte del antecedente"
    chooseLabel="Elegir soporte del antecedente"
    uploadLabel="Cargar soporte del antecedente"
    bind:value={draft.support}
    bind:pending
    onselected={onreviewed}
  />
</fieldset>
