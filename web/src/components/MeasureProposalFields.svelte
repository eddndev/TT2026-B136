<script>
  import { onDestroy } from 'svelte';
  import ParticipantSubjectPicker from './ParticipantSubjectPicker.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import HistoricalParticipantPicker from './HistoricalParticipantPicker.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import MeasureTimeFields from './MeasureTimeFields.svelte';
  import { measureKinds } from '../lib/measure-presentation.mjs';
  export let api,
    caseId,
    value,
    lockedIdentity = false,
    inputs = null,
    disabled = false,
    pending = false,
    ondenied,
    canApply = () => true;
  const typed = api.caseTypedParticipants(caseId),
    directory = api.caseParticipants(caseId);
  let choosingSubject = inputs?.choosingSubject ?? false,
    choosingSupervisor = inputs?.choosingSupervisor ?? false,
    subjectInputs = inputs?.subjectPicker ?? null,
    supervisorInputs = inputs?.supervisorPicker ?? null,
    subjectPicker,
    supervisorPicker,
    startFields,
    endFields,
    subjectBusy = false,
    supervisorBusy = false;
  $: pending = subjectBusy || supervisorBusy;
  export function captureInputs() {
    return {
      choosingSubject,
      choosingSupervisor,
      subjectPicker: subjectPicker?.captureDraft() ?? subjectInputs,
      supervisorPicker: supervisorPicker?.captureDraft() ?? supervisorInputs,
      start: startFields?.captureInputs() ?? inputs?.start ?? null,
      end: endFields?.captureInputs() ?? inputs?.end ?? null,
    };
  }
  function closeSubject() {
    subjectInputs = subjectPicker?.captureDraft() ?? subjectInputs;
    choosingSubject = false;
  }
  function closeSupervisor() {
    supervisorInputs = supervisorPicker?.captureDraft() ?? supervisorInputs;
    choosingSupervisor = false;
  }
  function chooseSubject(row) {
    if (!canApply() || lockedIdentity || row.case_id !== caseId) return;
    value = {
      ...value,
      subject: structuredClone(row),
      values: {
        ...value.values,
        subject: { id: row.id, revision: row.revision, values_digest: row.values_digest },
      },
    };
    closeSubject();
  }
  function chooseSupervisor(row) {
    if (!canApply() || row.case_id !== caseId) return;
    value = {
      ...value,
      supervisor: structuredClone(row),
      values: {
        ...value.values,
        supervision: {
          ...value.values.supervision,
          participant: { participant_id: row.id, revision: row.revision },
        },
      },
    };
    closeSupervisor();
  }
  function supervision(kind) {
    if (!canApply() || disabled || pending) return;
    closeSupervisor();
    value = {
      ...value,
      supervisor: null,
      values: {
        ...value.values,
        supervision: kind === 'unknown' ? { kind, reason: '' } : { kind, statement: '' },
      },
    };
  }
  function end(kind) {
    if (!canApply() || disabled || pending) return;
    value = {
      ...value,
      values: {
        ...value.values,
        validity: {
          ...value.values.validity,
          end: kind === 'absent' ? null : { precision: '' },
        },
      },
    };
  }
  onDestroy(() => {
    typed.dispose();
    directory.dispose();
    pending = false;
  });
</script>

<div class="stack">
  {#if value.subject}<ParticipantSubjectSummary record={value.subject} />
  {:else}<p>Selecciona la identidad exacta del sujeto.</p>{/if}
  {#if !lockedIdentity}
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => {
        if (!canApply()) return;
        closeSupervisor();
        choosingSubject = true;
      }}>Elegir sujeto</button
    >
    {#if choosingSubject}<ParticipantSubjectPicker
        api={typed}
        {ondenied}
        {canApply}
        bind:this={subjectPicker}
        draft={subjectInputs}
        disabled={disabled || supervisorBusy}
        bind:busy={subjectBusy}
        onselected={chooseSubject}
        oncancel={closeSubject}
      />{/if}
  {/if}
  <label
    >Clase de medida<select
      bind:value={value.values.kind}
      disabled={disabled || pending || lockedIdentity}
    >
      <option value="">Selecciona la clase declarada</option>
      {#each Object.entries(measureKinds) as [kind, label]}<option value={kind}>{label}</option
        >{/each}
    </select></label
  >
  {#if lockedIdentity}<p class="hint">
      La modificacion conserva el sujeto y la clase de la medida.
    </p>{/if}
  <label
    >Condiciones<textarea
      rows="3"
      maxlength="1000"
      bind:value={value.values.conditions}
      disabled={disabled || pending}></textarea></label
  >
  <MeasureTimeFields
    bind:this={startFields}
    bind:value={value.values.validity.start}
    label="inicio de vigencia"
    disabled={disabled || pending}
    inputs={inputs?.start ?? null}
  />
  <label
    >Declaracion de vigencia<textarea
      rows="3"
      maxlength="1000"
      bind:value={value.values.validity.statement}
      disabled={disabled || pending}></textarea></label
  >
  <label
    >Fin de vigencia<select
      value={value.values.validity.end === null ? 'absent' : 'declared'}
      disabled={disabled || pending}
      onchange={(event) => end(event.currentTarget.value)}
    >
      <option value="absent">Sin fin declarado</option><option value="declared"
        >Fin declarado</option
      >
    </select></label
  >
  {#if value.values.validity.end !== null}<MeasureTimeFields
      bind:this={endFields}
      bind:value={value.values.validity.end}
      label="fin de vigencia"
      disabled={disabled || pending}
      inputs={inputs?.end ?? null}
    />{/if}
  <label
    >Supervision<select
      value={value.values.supervision.kind}
      disabled={disabled || pending}
      onchange={(event) => supervision(event.currentTarget.value)}
    >
      <option value="unknown">No consta</option><option value="known">Ficha exacta conocida</option>
    </select></label
  >
  {#if value.values.supervision.kind === 'unknown'}
    <label
      >Motivo de supervision desconocida<textarea
        rows="2"
        maxlength="1000"
        bind:value={value.values.supervision.reason}
        disabled={disabled || pending}></textarea></label
    >
  {:else}
    {#if value.supervisor}<ParticipantSummary record={value.supervisor} />{/if}
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => {
        if (!canApply()) return;
        closeSubject();
        choosingSupervisor = true;
      }}>Elegir supervisor</button
    >
    {#if choosingSupervisor}<HistoricalParticipantPicker
        api={directory}
        typedApi={typed}
        {ondenied}
        {canApply}
        bind:this={supervisorPicker}
        draft={supervisorInputs}
        disabled={disabled || subjectBusy}
        bind:busy={supervisorBusy}
        onselected={chooseSupervisor}
        oncancel={closeSupervisor}
        selectLabel="Vincular esta ficha supervisora"
      />{/if}
    <label
      >Declaracion de supervision<textarea
        rows="2"
        maxlength="1000"
        bind:value={value.values.supervision.statement}
        disabled={disabled || pending}></textarea></label
    >
  {/if}
</div>
