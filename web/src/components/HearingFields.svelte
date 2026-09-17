<script>
  import HearingDate from './HearingDate.svelte';
  import HearingParticipants from './HearingParticipants.svelte';
  import HearingParticipantPicker from './HearingParticipantPicker.svelte';
  import HearingAntecedent from './HearingAntecedent.svelte';
  import { hearingKinds, hearingModalities } from '../lib/hearings.mjs';
  export let draft,
    rows,
    base,
    context,
    api,
    typedApi,
    documents,
    caseId,
    ondenied,
    onreviewed,
    disabled = false,
    pending = false;
  let picking = false,
    pickerBusy = false,
    supportBusy = false;
  $: pending = pickerBusy || supportBusy;
  function select(record) {
    if (
      draft.participants.length >= 32 ||
      draft.participants.some((item) => item.participant_id === record.id)
    )
      return;
    draft.participants = [
      ...draft.participants,
      { participant_id: record.id, revision: record.revision },
    ];
    rows = [
      ...rows,
      { ...record, profile: record.profile ? 'typed' : 'manual', kind: record.profile?.kind },
    ];
    picking = false;
    onreviewed('participants');
  }
  function remove(id) {
    draft.participants = draft.participants.filter((row) => row.participant_id !== id);
    rows = rows.filter((row) => row.id !== id);
    onreviewed('participants');
  }
</script>

<div class="stack hearing-fields">
  <div class="case-field-grid">
    <label
      >Tipo de audiencia<select
        bind:value={draft.kind}
        disabled={disabled || pending || !!base}
        onchange={() => {
          draft.statement = '';
          draft.support = null;
        }}
      >
        {#each Object.entries(hearingKinds) as [key, value]}{#if base || value.stage === context?.stage}<option
              value={key}>{value.label}</option
            >{/if}{/each}
      </select></label
    >
    <label
      >Modalidad<select bind:value={draft.modality} disabled={disabled || pending}
        >{#each Object.entries(hearingModalities) as [key, label]}<option value={key}
            >{label}</option
          >{/each}</select
      ></label
    >
  </div>
  <HearingDate bind:value={draft.time} disabled={disabled || pending} />
  <label
    >Sede o conexi&#243;n<input bind:value={draft.venue} disabled={disabled || pending} /></label
  >
  <label
    >Nota declarada (opcional)<textarea
      rows="3"
      bind:value={draft.note}
      disabled={disabled || pending}></textarea></label
  >
  <fieldset class="case-offenses">
    <legend>Participantes previstos ({draft.participants.length}/32)</legend>
    <HearingParticipants {rows} onremove={remove} disabled={disabled || pending} />
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending || draft.participants.length >= 32}
      onclick={() => (picking = true)}>Elegir participante</button
    >
    {#if picking}<HearingParticipantPicker
        {api}
        {typedApi}
        {ondenied}
        selectedIds={draft.participants.map((row) => row.participant_id)}
        disabled={disabled || supportBusy}
        bind:busy={pickerBusy}
        onselected={select}
        oncancel={() => (picking = false)}
      />{/if}
  </fieldset>
  {#if draft.kind === 'sentencing'}<HearingAntecedent
      api={documents}
      {caseId}
      bind:draft
      {ondenied}
      onreviewed={() => onreviewed('support')}
      disabled={disabled || pickerBusy}
      bind:pending={supportBusy}
    />{/if}
  {#if base}<label
      >Motivo del cambio<textarea rows="3" bind:value={draft.reason} disabled={disabled || pending}
      ></textarea></label
    >{/if}
</div>
