<script>
  import { onDestroy } from 'svelte';
  import PrecautionaryReviewTargets from './PrecautionaryReviewTargets.svelte';
  import HearingDate from './HearingDate.svelte';
  import HearingParticipants from './HearingParticipants.svelte';
  import HearingParticipantPicker from './HearingParticipantPicker.svelte';
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import { hearingModalities } from '../lib/hearings.mjs';
  import { precautionaryHearingPurposes } from '../lib/precautionary-hearing-presentation.mjs';
  export let api,
    caseId,
    action,
    fields,
    participants,
    support,
    reviewTargets,
    ondenied,
    inputs = null,
    disabled = false,
    pending = false,
    canApply = () => true;
  const directory = api.caseParticipants(caseId),
    typed = api.caseTypedParticipants(caseId),
    documents = api.caseDocuments(caseId);
  let choosingParticipant = inputs?.choosingParticipant ?? false,
    choosingSupport = inputs?.choosingSupport ?? false,
    participantInputs = inputs?.participantPicker ?? null,
    supportInputs = inputs?.supportPicker ?? null,
    targetPicker,
    targetBusy = false,
    participantPicker,
    supportPicker,
    participantBusy = false,
    supportBusy = false;
  $: pending = participantBusy || supportBusy || targetBusy;
  $: participantRows = participants.map((row) => ({
    ...row,
    kind: row.profile?.kind ?? row.kind ?? null,
  }));
  export function captureInputs() {
    return {
      ...targetPicker?.captureInputs(),
      choosingParticipant,
      choosingSupport,
      participantPicker: participantPicker?.captureDraft() ?? participantInputs,
      supportPicker: supportPicker?.captureDraft() ?? supportInputs,
    };
  }
  function closeParticipant() {
    participantInputs = participantPicker?.captureDraft() ?? participantInputs;
    choosingParticipant = false;
  }
  function closeSupport() {
    supportInputs = supportPicker?.captureDraft() ?? supportInputs;
    choosingSupport = false;
  }
  function selectParticipant(row) {
    if (
      !canApply() ||
      row.case_id !== caseId ||
      participants.length >= 32 ||
      participants.some((value) => value.id === row.id)
    )
      return;
    participants = [...participants, structuredClone(row)].sort((a, b) => a.id.localeCompare(b.id));
    closeParticipant();
  }
  function selectSupport(row) {
    if (!canApply() || row.case_id !== caseId) return;
    support = structuredClone(row);
    closeSupport();
  }
  function removeParticipant(id) {
    if (!canApply() || disabled || pending) return;
    participants = participants.filter((row) => row.id !== id);
  }
  onDestroy(() => {
    directory.dispose();
    typed.dispose();
    documents.dispose();
    pending = false;
  });
</script>

<div class="stack hearing-fields">
  {#if action !== 'cancel'}
    <div class="case-field-grid">
      <label
        >Prop&#243;sito<select bind:value={fields.purpose} disabled={disabled || pending}>
          <option value="imposition">Imposici&#243;n de medidas cautelares</option>
          <option value="review">Revisi&#243;n de medidas cautelares</option>
        </select></label
      >
      <label
        >Modalidad<select bind:value={fields.modality} disabled={disabled || pending}>
          {#each Object.entries(hearingModalities) as [value, label]}
            <option {value}>{label}</option>
          {/each}
        </select></label
      >
    </div>
    {#if fields.purpose === 'review'}
      <PrecautionaryReviewTargets
        {api}
        {caseId}
        {ondenied}
        {canApply}
        {inputs}
        bind:this={targetPicker}
        bind:references={reviewTargets}
        bind:pending={targetBusy}
        disabled={disabled || participantBusy || supportBusy}
      />
    {:else if reviewTargets.length}
      <p class="notice">La imposicion requiere quitar las referencias de revision.</p>
      <button class="secondary" disabled={disabled || pending} onclick={() => (reviewTargets = [])}
        >Quitar referencias de revision</button
      >
    {/if}
    <HearingDate bind:value={fields} disabled={disabled || pending} />
    <label
      >Sede o enlace<input
        maxlength="500"
        bind:value={fields.venue}
        disabled={disabled || pending}
      /></label
    >
    <label
      >Nota<textarea
        rows="3"
        maxlength="1000"
        bind:value={fields.note}
        disabled={disabled || pending}></textarea></label
    >
    <fieldset class="case-offenses">
      <legend>Base de se&#241;alamiento</legend>
      <label
        >Base de se&#241;alamiento<textarea
          rows="3"
          maxlength="1000"
          bind:value={fields.statement}
          disabled={disabled || pending}></textarea></label
      >
      <label
        >Localizador<textarea
          rows="2"
          maxlength="1000"
          bind:value={fields.locator}
          disabled={disabled || pending}></textarea></label
      >
      <StageSupportSummary record={support} />
      <button
        type="button"
        class="secondary"
        disabled={disabled || pending}
        onclick={() => {
          closeParticipant();
          choosingSupport = true;
        }}>Elegir soporte</button
      >
      {#if choosingSupport}<StagePicker
          api={documents}
          {caseId}
          {ondenied}
          {canApply}
          bind:this={supportPicker}
          draft={supportInputs}
          disabled={disabled || participantBusy}
          bind:busy={supportBusy}
          onselected={selectSupport}
          oncancel={closeSupport}
        />{/if}
    </fieldset>
    <fieldset class="case-offenses">
      <legend>Participantes previstos ({participants.length}/32)</legend>
      <HearingParticipants
        rows={participantRows}
        onremove={removeParticipant}
        disabled={disabled || pending}
      />
      <button
        type="button"
        class="secondary"
        disabled={disabled || pending || participants.length >= 32}
        onclick={() => {
          closeSupport();
          choosingParticipant = true;
        }}>Elegir participante</button
      >
      {#if choosingParticipant}<HearingParticipantPicker
          api={directory}
          typedApi={typed}
          {ondenied}
          {canApply}
          selectedIds={participants.map((row) => row.id)}
          bind:this={participantPicker}
          draft={participantInputs}
          disabled={disabled || supportBusy}
          bind:busy={participantBusy}
          onselected={selectParticipant}
          oncancel={closeParticipant}
        />{/if}
    </fieldset>
  {:else}
    <p class="notice">
      La cancelaci&#243;n conserva el horario, los participantes y el soporte de la convocatoria
      anterior. Revisa esos datos antes de confirmar.
    </p>
    <section class="case-comparison" aria-label="Convocatoria retenida">
      <h3>{precautionaryHearingPurposes[fields.purpose]}</h3>
      <p>{fields.date} / {fields.time} / UTC{fields.offset}</p>
      <p>{hearingModalities[fields.modality]} / {fields.venue}</p>
      <StageSupportSummary record={support} />
      <HearingParticipants rows={participantRows} />
    </section>
  {/if}
  {#if action !== 'schedule'}
    <label
      >Motivo del cambio<textarea
        rows="3"
        maxlength="1000"
        bind:value={fields.reason}
        disabled={disabled || pending}></textarea></label
    >
  {/if}
</div>
