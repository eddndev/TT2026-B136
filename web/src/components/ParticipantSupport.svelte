<script>
  import { getContext, onDestroy } from 'svelte';
  import StagePicker from './StagePicker.svelte';
  import UploadDocument from './UploadDocument.svelte';
  import {
    createParticipantSupportDraft,
    participantSupportKey,
  } from '../lib/participant-support-draft.mjs';
  export let api,
    caseId,
    label,
    value = null,
    ondenied,
    disabled = false,
    pending = false,
    draftContext = null;
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
    alive = true;
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
    const key = context && session ? participantSupportKey(context) : null;
    if (key === recoveryKey) return;
    recovery?.dispose();
    recoveryKey = key;
    pickerDraft = null;
    picking = false;
    uploaded = false;
    recovery = key
      ? createParticipantSupportDraft({
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
    preparing = true;
    error = '';
    try {
      if (recovery?.pending()) {
        const restored = await recovery.restore((value) => {
          pickerDraft = value.picker;
        });
        if (restored.status !== 'restored') {
          if (admitted())
            error = 'No se pudo recuperar el selector. Vuelve a consultar el soporte.';
          return;
        }
      } else if (recovery) {
        if (!(await recovery.authorize())) return;
        recovery.register();
      }
      if (admitted()) await operation();
    } catch (failure) {
      if (admitted()) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) preparing = false;
    }
  }
  function closePicker() {
    pickerDraft = picker?.captureDraft() ?? pickerDraft;
    picking = false;
  }
  function selected(record) {
    if (!admitted()) return;
    value = {
      document_id: record.id,
      version: record.version,
      digest: record.digest,
      locator: '',
      name: record.name,
    };
    closePicker();
  }
  function loaded(record) {
    if (!admitted() || record.case_id !== caseId) return;
    selected(record);
    uploaded = true;
  }
  onDestroy(() => {
    alive = false;
    recovery?.dispose();
    pending = false;
  });
</script>

<fieldset class="case-offenses stage-support" aria-label={label}>
  <legend>{label}</legend>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
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
      onclick={() =>
        prepare(() => {
          picking = true;
        })}>Seleccionar documento</button
    >
    <button
      class="text-button"
      type="button"
      disabled={disabled || pending}
      onclick={() => prepare(() => uploader.open())}>Cargar soporte</button
    >
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
