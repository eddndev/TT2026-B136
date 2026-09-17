<script>
  import StageDate from './StageDate.svelte';
  import HearingResultAttendance from './HearingResultAttendance.svelte';
  import HearingResultAgreements from './HearingResultAgreements.svelte';
  import HearingResultSource from './HearingResultSource.svelte';
  import { resultOccurrence, resultExtent } from '../lib/hearing-result-errors.mjs';
  export let draft,
    rows,
    candidates,
    caseId,
    participants,
    typed,
    documents,
    ondenied,
    disabled = false,
    pending = false;
  let attendanceBusy = false,
    sourceBusy = false;
  $: pending = attendanceBusy || sourceBusy;
</script>

<div class="hearing-fields">
  <fieldset class="case-offenses" disabled={disabled || pending}>
    <legend>Lo ocurrido</legend>
    <div class="case-field-grid">
      <label
        >Ocurrencia<select bind:value={draft.occurrence}
          ><option value="">Selecciona lo informado</option
          >{#each Object.entries(resultOccurrence) as [value, label]}<option {value}>{label}</option
            >{/each}</select
        ></label
      >
      <label
        >Alcance declarado<select bind:value={draft.extent}
          ><option value="">Selecciona el alcance</option
          >{#each Object.entries(resultExtent) as [value, label]}<option
              {value}
              disabled={draft.occurrence === 'not_started' && value !== 'unspecified'}
              >{label}</option
            >{/each}</select
        ></label
      >
    </div>
    <p class="hint">
      El alcance corresponde a esta sesi&#243;n o acto. Si no se inici&#243;, selecciona No consta.
    </p>
    <StageDate label="Tiempo del hecho informado" bind:value={draft.time} />
    <p class="hint">La fecha del hecho informado es independiente de la cita y de esta captura.</p>
    <label>Relato del operador<textarea rows="4" bind:value={draft.summary}></textarea></label>
  </fieldset>
  <HearingResultSource
    bind:draft
    {caseId}
    {documents}
    {ondenied}
    disabled={disabled || attendanceBusy}
    bind:pending={sourceBusy}
  />
  <HearingResultAttendance
    bind:draft
    bind:rows
    {candidates}
    api={participants}
    typedApi={typed}
    {ondenied}
    disabled={disabled || sourceBusy}
    bind:pending={attendanceBusy}
  />
  <HearingResultAgreements bind:draft disabled={disabled || pending} />
</div>
