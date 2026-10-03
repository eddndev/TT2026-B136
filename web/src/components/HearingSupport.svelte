<script>
  import { getContext, onDestroy } from 'svelte';
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import UploadDocument from './UploadDocument.svelte';
  import { createStageSupportDraft, stageSupportKey } from '../lib/stage-support-draft.mjs';
  export let api,
    caseId,
    label,
    chooseLabel,
    uploadLabel,
    value,
    ondenied,
    disabled = false,
    optional = false,
    pending = false,
    draftContext = null,
    ondiscard = () => {},
    onselected = () => {};
  let picking = false,
    pickerBusy = false,
    uploadBusy = false,
    uploader,
    uploaded = false;
  const session = getContext('session-drafts');
  let recovery = null,
    recoveryKey = null,
    picker,
    pickerDraft = null;
  let preparing = false,
    error = '',
    alive = true,
    generation = 0;
  $: configure(draftContext);
  $: pending = preparing || pickerBusy || uploadBusy;
  $: childContext =
    draftContext && recovery
      ? {
          ...draftContext,
          ownerDraftKey: recovery.key,
          fieldPath: ['upload'],
          rowId: null,
          canApply: admitted,
        }
      : null;
  function configure(context) {
    const key = context && session ? stageSupportKey(context) : null;
    if (key === recoveryKey) return;
    recovery?.dispose();
    generation++;
    recoveryKey = key;
    pickerDraft = null;
    picking = false;
    recovery = key
      ? createStageSupportDraft({
          session,
          context: () => draftContext,
          capture: () => ({ picker: picker?.captureDraft() ?? pickerDraft }),
        })
      : null;
  }
  function admitted() {
    return alive && (!recovery || recovery.admitted());
  }
  async function prepare(operation) {
    if (pending || disabled || !admitted()) return;
    const ticket = generation,
      valid = () => admitted() && ticket === generation;
    preparing = true;
    error = '';
    try {
      if (recovery?.pending()) {
        const result = await recovery.restore((value) => {
          pickerDraft = value.picker;
        });
        if (!valid()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el selector de soporte.');
      } else if (recovery) {
        if (!(await recovery.authorize()) || !valid()) return;
        recovery.register();
      }
      if (valid()) await operation();
    } catch (failure) {
      if (valid()) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive && ticket === generation) preparing = false;
    }
  }
  function closePicker() {
    pickerDraft = picker?.captureDraft() ?? pickerDraft;
    picking = false;
  }
  function selected(record) {
    if (!admitted() || record.case_id !== caseId) return;
    value = {
      case_id: record.case_id,
      id: record.id,
      version: record.version,
      digest: record.digest,
      name: record.name,
      uploaded: false,
    };
    closePicker();
    uploaded = false;
    onselected(value);
  }
  function loaded(record) {
    if (!admitted() || record.case_id !== caseId) return;
    selected(record);
    value = { ...value, uploaded: true };
    uploaded = true;
    onselected(value);
  }
  function remove() {
    if (pending || disabled || !admitted()) return;
    recovery?.close();
    ondiscard();
    pickerDraft = null;
    picking = false;
    value = null;
    uploaded = false;
  }
  onDestroy(() => {
    alive = false;
    generation++;
    recovery?.dispose();
    pending = false;
  });
</script>

<fieldset class="case-offenses hearing-support" aria-label={label}>
  <legend>{label}{optional ? ' (opcional)' : ''}</legend>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if value}<StageSupportSummary record={value} />{/if}
  {#if uploaded || value?.uploaded}<p class="notice" role="status">
      Documento guardado: {value.name} / versi&#243;n {value.version}. Falta confirmar el registro
      propietario.
    </p>{/if}
  <p class="hint">
    Selecciona una versi&#243;n exacta de un PDF o DOCX. El servidor validar&#225; el soporte al
    confirmar el registro.
  </p>
  <div class="action-row">
    <button
      class="secondary"
      type="button"
      disabled={disabled || pending}
      onclick={() =>
        prepare(() => {
          picking = true;
        })}>{chooseLabel}</button
    >
    <button
      class="secondary"
      type="button"
      disabled={disabled || pending}
      onclick={() => prepare(() => uploader.open())}>{uploadLabel}</button
    >
    {#if optional && value}<button
        class="text-button"
        type="button"
        disabled={disabled || pending}
        onclick={remove}>Quitar soporte del borrador</button
      >{/if}
  </div>
  {#if picking}<StagePicker
      {api}
      {caseId}
      {ondenied}
      {disabled}
      bind:this={picker}
      draft={pickerDraft}
      canApply={admitted}
      bind:busy={pickerBusy}
      onselected={selected}
      oncancel={closePicker}
    />{/if}
</fieldset>
<UploadDocument
  {api}
  {ondenied}
  {disabled}
  draftContext={childContext}
  bind:this={uploader}
  bind:busy={uploadBusy}
  onuploaded={loaded}
/>
