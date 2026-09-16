<script>
  import HistoricalParticipantPicker from './HistoricalParticipantPicker.svelte';
  import HearingReference from './HearingReference.svelte';
  export let draft,
    rows,
    candidates = [],
    api,
    typedApi,
    ondenied,
    disabled = false,
    pending = false;
  let picking = false;
  function selected(row) {
    if (
      draft.attendees.length >= 32 ||
      draft.attendees.some((item) => item.participant_id === row.id)
    )
      return;
    rows = [...rows, row];
    draft.attendees = [
      ...draft.attendees,
      { participant_id: row.id, revision: row.revision, capacity: '', observation: '' },
    ];
    picking = false;
  }
  function remove(id) {
    draft.attendees = draft.attendees.filter((item) => item.participant_id !== id);
    rows = rows.filter((row) => row.id !== id);
  }
</script>

<details class="hearing-result-optional">
  <summary>Comparecencias informadas ({draft.attendees.length}/32)</summary>
  <p class="hint">
    Selecciona cada ficha y su calidad en esta sesi&#243;n. No seleccionarlas no significa ausencia;
    se admiten comparecencias aunque el acto no inicie.
  </p>
  {#if !draft.attendees.length}<p>Comparecencias no registradas.</p>{/if}
  {#each draft.attendees as item (item.participant_id)}
    {@const row = rows.find(
      (value) => value.id === item.participant_id && value.revision === item.revision,
    )}
    <div class="case-comparison" role="group" aria-label={`Comparecencia ${item.participant_id}`}>
      <strong>{row?.display_name || 'Ficha seleccionada'}</strong>
      <HearingReference row={row || { id: item.participant_id, revision: item.revision }} />
      <label
        >Calidad en esta sesi&#243;n<input
          bind:value={item.capacity}
          disabled={disabled || pending}
        /></label
      >
      <label
        >Observaci&#243;n de comparecencia (opcional)<textarea
          rows="2"
          bind:value={item.observation}
          disabled={disabled || pending}></textarea></label
      >
      <button
        type="button"
        class="text-button"
        disabled={disabled || pending}
        onclick={() => remove(item.participant_id)}>Quitar comparecencia</button
      >
    </div>
  {/each}
  <button
    type="button"
    class="secondary"
    disabled={disabled || pending || draft.attendees.length >= 32}
    onclick={() => (picking = true)}>Agregar comparecencia</button
  >
  {#if picking}<HistoricalParticipantPicker
      {api}
      {typedApi}
      {candidates}
      {ondenied}
      {disabled}
      selectedIds={draft.attendees.map((item) => item.participant_id)}
      bind:busy={pending}
      onselected={selected}
      oncancel={() => (picking = false)}
    />{/if}
</details>
