<script>
  import { onDestroy } from 'svelte';
  import MeasureTimeFields from './MeasureTimeFields.svelte';
  import ParticipantSubjectPicker from './ParticipantSubjectPicker.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  export let api,
    caseId,
    fields,
    action,
    subject,
    ondenied,
    inputs = {},
    disabled = false,
    pending = false,
    canApply = () => true;
  const typed = action === 'replace_entered_in_error' ? api.caseTypedParticipants(caseId) : null;
  let choosingSubject = inputs?.choosingSubject ?? false,
    subjectInputs = inputs?.subjectPicker ?? null,
    subjectPicker,
    startFields,
    endFields,
    subjectBusy = false;
  $: pending = subjectBusy;
  export function captureInputs() {
    return {
      choosingSubject,
      subjectPicker: subjectPicker?.captureDraft() ?? subjectInputs,
      start: startFields?.captureInputs() ?? inputs?.start ?? null,
      end: endFields?.captureInputs() ?? inputs?.end ?? null,
    };
  }
  function closeSubject() {
    subjectInputs = subjectPicker?.captureDraft() ?? subjectInputs;
    choosingSubject = false;
  }
  function chooseSubject(row) {
    if (!canApply() || row.case_id !== caseId) return;
    subject = structuredClone(row);
    closeSubject();
  }
  onDestroy(() => {
    typed?.dispose();
    pending = false;
  });
</script>

<div class="stack">
  <label
    >Motivo de rectificacion<textarea
      rows="3"
      maxlength="1000"
      bind:value={fields.reason}
      disabled={disabled || pending}></textarea></label
  >
  {#if action === 'correct'}
    <p class="hint">La rectificacion conserva la identidad, clase y procedencia judicial.</p>
    <label
      >Condiciones<textarea
        rows="3"
        maxlength="1000"
        bind:value={fields.conditions}
        disabled={disabled || pending}></textarea></label
    >
    <MeasureTimeFields
      bind:this={startFields}
      bind:value={fields.validity.start}
      label="inicio de vigencia"
      inputs={inputs?.start ?? null}
      disabled={disabled || pending}
    />
    <label
      >Declaracion de vigencia<textarea
        rows="3"
        maxlength="1000"
        bind:value={fields.validity.statement}
        disabled={disabled || pending}></textarea></label
    >
    {#if fields.validity.end !== null}
      <MeasureTimeFields
        bind:this={endFields}
        bind:value={fields.validity.end}
        label="fin de vigencia"
        inputs={inputs?.end ?? null}
        disabled={disabled || pending}
      />
    {:else}<p>Sin fin declarado en el registro seleccionado.</p>{/if}
    <label
      >Texto de supervision<textarea
        rows="3"
        maxlength="1000"
        bind:value={fields.supervisionText}
        disabled={disabled || pending}></textarea></label
    >
  {:else if action === 'replace_entered_in_error'}
    {#if subject}<ParticipantSubjectSummary record={subject} />
    {:else}<p>Selecciona la identidad exacta del sujeto para el registro de reemplazo.</p>{/if}
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => {
        if (canApply()) choosingSubject = true;
      }}>Elegir sujeto</button
    >
    {#if choosingSubject}<ParticipantSubjectPicker
        api={typed}
        {ondenied}
        {canApply}
        bind:this={subjectPicker}
        draft={subjectInputs}
        {disabled}
        bind:busy={subjectBusy}
        onselected={chooseSubject}
        oncancel={closeSubject}
      />{/if}
  {/if}
</div>
