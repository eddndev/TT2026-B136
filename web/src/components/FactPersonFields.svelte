<script>
  import { onDestroy } from 'svelte';
  import HistoricalParticipantPicker from './HistoricalParticipantPicker.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  export let value,
    label,
    api,
    caseId,
    declared = true,
    ondenied,
    disabled = false,
    pending = false;
  const participants = api.caseParticipants(caseId),
    typed = api.caseTypedParticipants(caseId);
  let choosing = false,
    selected = null;
  $: person = declared ? value?.value : value;
  $: mode = declared && value?.kind === 'unknown' ? 'unknown' : person?.kind || '';
  function change(kind) {
    selected = null;
    choosing = false;
    if (declared && kind === 'unknown') {
      value = { kind: 'unknown', reason: '' };
      return;
    }
    const next =
      kind === 'participant'
        ? { kind, id: '', revision: undefined }
        : kind === 'unlinked'
          ? { kind, label: '', description: '' }
          : { kind: '' };
    value = declared ? { kind: kind ? 'known' : '', value: next } : next;
  }
  function select(row) {
    const next = { kind: 'participant', id: row.id, revision: row.revision };
    value = declared ? { kind: 'known', value: next } : next;
    selected = row;
    choosing = false;
  }
  function text(field, content) {
    const next = { ...person, [field]: content };
    value = declared ? { kind: 'known', value: next } : next;
  }
  onDestroy(() => {
    participants.dispose();
    typed.dispose();
  });
</script>

<fieldset class="case-offenses fact-person-fields" {disabled}>
  <legend>{label}</legend>
  <label
    >{label}<select
      value={mode}
      onchange={(event) => change(event.currentTarget.value)}
      disabled={pending}
    >
      <option value="">Selecciona lo declarado</option>
      {#if declared}<option value="unknown">No consta</option>{/if}
      <option value="participant">Ficha del expediente</option><option value="unlinked"
        >Persona sin ficha vinculada</option
      >
    </select></label
  >
  {#if mode === 'unknown'}<label
      >Motivo: {label}<textarea rows="2" bind:value={value.reason} disabled={pending}
      ></textarea></label
    >
  {:else if mode === 'unlinked'}
    <label
      >Nombre declarado: {label}<input
        value={person.label}
        oninput={(event) => text('label', event.currentTarget.value)}
        disabled={pending}
      /></label
    >
    <label
      >Descripci&#243;n: {label}<textarea
        rows="2"
        value={person.description}
        oninput={(event) => text('description', event.currentTarget.value)}
        disabled={pending}></textarea></label
    >
  {:else if mode === 'participant'}
    {#if selected}<ParticipantSummary record={selected} />
    {:else if person.id}<p>
        Ficha vinculada en la revisi&#243;n {person.revision}. Su identidad exacta se muestra al
        preparar el registro.
      </p>{/if}
    {#if person.id}<details>
        <summary>Referencia de la ficha</summary><code>{person.id}</code>
        <p>Revision {person.revision}</p>
      </details>{/if}
    <button type="button" class="secondary" disabled={pending} onclick={() => (choosing = true)}
      >Elegir ficha: {label}</button
    >
    {#if choosing}<HistoricalParticipantPicker
        api={participants}
        typedApi={typed}
        {ondenied}
        {disabled}
        bind:busy={pending}
        onselected={select}
        oncancel={() => (choosing = false)}
        selectLabel="Vincular esta revisi&#243;n"
      />{/if}
  {/if}
  <p class="hint">
    La referencia conserva la revisi&#243;n elegida. Una ficha o una comparecencia no acredita por
    s&#237; misma la notificaci&#243;n.
  </p>
</fieldset>
