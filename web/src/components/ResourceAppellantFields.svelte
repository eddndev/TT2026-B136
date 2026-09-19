<script>
  import { onDestroy } from 'svelte';
  import FactDeclarationFields from './FactDeclarationFields.svelte';
  import HistoricalParticipantPicker from './HistoricalParticipantPicker.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  export let api,
    caseId,
    value,
    number,
    selectedIds = [],
    ondenied,
    disabled = false,
    busy = false;
  const participants = api.caseParticipants(caseId),
    typed = api.caseTypedParticipants(caseId);
  let choosing = false,
    selected = null;
  function select(row) {
    value = {
      ...value,
      name: row.display_name,
      participant: { id: row.id, revision: row.revision },
    };
    selected = row;
    choosing = false;
  }
  onDestroy(() => {
    participants.dispose();
    typed.dispose();
    busy = false;
  });
</script>

<fieldset class="case-offenses" disabled={disabled || busy}>
  <legend>Persona recurrente {number}</legend>
  <label>Nombre de recurrente {number}<input bind:value={value.name} maxlength="200" /></label>
  <FactDeclarationFields
    bind:value={value.role}
    label={`Rol de recurrente ${number}`}
    disabled={disabled || busy}
  />
  {#if value.participant}
    {#if selected}<ParticipantSummary record={selected} />
    {:else}<p>Ficha capturada en la revisi&#243;n {value.participant.revision}.</p>{/if}
    <details>
      <summary>Referencia de recurrente {number}</summary><code>{value.participant.id}</code>
    </details>
    <button
      type="button"
      class="text-button"
      onclick={() => {
        value = { ...value, participant: null };
        selected = null;
      }}>Quitar ficha de recurrente {number}</button
    >
  {:else}<p class="hint">
      Sin ficha vinculada. El nombre y rol se conservan como declaraci&#243;n.
    </p>{/if}
  <button type="button" class="secondary" onclick={() => (choosing = true)}
    >Elegir ficha de recurrente {number}</button
  >
</fieldset>
{#if choosing}
  <HistoricalParticipantPicker
    api={participants}
    typedApi={typed}
    {ondenied}
    {disabled}
    selectedIds={selectedIds.filter((id) => id !== value.participant?.id)}
    bind:busy
    onselected={select}
    oncancel={() => (choosing = false)}
    selectLabel="Vincular esta ficha recurrente"
  />
{/if}
