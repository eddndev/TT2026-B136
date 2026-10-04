<script>
  import { onDestroy } from 'svelte';
  import HearingDate from './HearingDate.svelte';
  import HearingParticipants from './HearingParticipants.svelte';
  import HearingParticipantPicker from './HearingParticipantPicker.svelte';
  import ResourceActivityActPicker from './ResourceActivityActPicker.svelte';
  import { resourceHearingKinds } from '../lib/resource-hearing-values.mjs';
  import { hearingModalities } from '../lib/hearings.mjs';
  import { hearingSupports, supportKey } from './resource-hearing-editor-values.mjs';
  export let api,
    caseId,
    resource,
    fields,
    act,
    participants,
    ondenied,
    disabled = false,
    pending = false,
    inputs = null,
    canApply = () => true;
  const directory = api.caseParticipants(caseId),
    typed = api.caseTypedParticipants(caseId);
  let choosingAct = inputs?.choosingAct ?? false,
    choosingParticipant = inputs?.choosingParticipant ?? false,
    actPicker,
    participantPicker,
    actBusy = false,
    participantBusy = false,
    actInputs = inputs?.actPicker ?? null,
    participantInputs = inputs?.participantPicker ?? null,
    supports = [],
    supportError = '';
  $: pending = actBusy || participantBusy;
  $: {
    try {
      supports = hearingSupports(resource, act);
      supportError = '';
    } catch (failure) {
      supports = [];
      supportError = failure.message;
    }
  }
  export function captureInputs() {
    return {
      choosingAct,
      choosingParticipant,
      actPicker: actPicker?.captureInputs() ?? actInputs,
      participantPicker: participantPicker?.captureDraft() ?? participantInputs,
    };
  }
  function chooseAct(row) {
    if (!canApply()) return;
    act = row;
    choosingAct = false;
    actInputs = null;
  }
  function clearAct() {
    act = null;
    choosingAct = false;
    actInputs = null;
    if (!resource.sources.supports.some((row) => supportKey(row) === fields.supportKey))
      fields.supportKey = resource.sources.supports[0]
        ? supportKey(resource.sources.supports[0])
        : '';
  }
  function closeParticipant() {
    participantInputs = participantPicker?.captureDraft() ?? participantInputs;
    choosingParticipant = false;
  }
  function chooseParticipant(row) {
    if (
      !canApply() ||
      participants.length >= 32 ||
      participants.some((value) => value.id === row.id)
    )
      return;
    participants = [...participants, row].sort((a, b) => a.id.localeCompare(b.id));
    closeParticipant();
  }
  onDestroy(() => {
    directory.dispose();
    typed.dispose();
    pending = false;
  });
</script>

<div class="stack hearing-fields">
  <p>Captura del recurso: {resource.values.title} / Revisi&#243;n {resource.revision}</p>
  <p class="hint">
    El recurso seleccionado conserva su historia. La cabeza esperada se revisa por separado.
  </p>
  <div class="case-field-grid">
    <label
      >Tipo de audiencia de recurso<select value={fields.kind || ''} disabled>
        {#if fields.kind}<option value={fields.kind}>{resourceHearingKinds[fields.kind]}</option>
        {:else}<option value="">Recurso sin modalidad escrita compatible</option>{/if}
      </select></label
    >
    <label
      >Modalidad<select bind:value={fields.modality} disabled={disabled || pending}>
        {#each Object.entries(hearingModalities) as [value, label]}<option {value}>{label}</option
          >{/each}
      </select></label
    >
  </div>
  <HearingDate bind:value={fields} disabled={disabled || pending} />
  <label
    >Sede o enlace<input
      maxlength="500"
      bind:value={fields.venue}
      disabled={disabled || pending}
    /></label
  >
  <label
    >Nota<textarea rows="3" maxlength="1000" bind:value={fields.note} disabled={disabled || pending}
    ></textarea></label
  >
  <section class="case-comparison" aria-label="Acto opcional del recurso">
    <h3>Acto del recurso (opcional)</h3>
    {#if act}<p>Acto revisi&#243;n {act.act.revision} / Recurso revisi&#243;n {act.revision}</p>
      <p class="case-multiline">{act.act.values.statement}</p>
      <button type="button" class="text-button" disabled={disabled || pending} onclick={clearAct}
        >Quitar acto</button
      >
    {:else}<p>Sin acto seleccionado.</p>{/if}
    {#if choosingAct}<ResourceActivityActPicker
        {api}
        {caseId}
        {resource}
        {ondenied}
        bind:this={actPicker}
        savedInputs={actInputs}
        disabled={disabled || participantBusy}
        bind:busy={actBusy}
        onselected={chooseAct}
        oncancel={() => {
          actInputs = actPicker?.captureInputs();
          choosingAct = false;
        }}
      />
    {:else}<button
        type="button"
        class="secondary"
        disabled={disabled || pending}
        onclick={() => (choosingAct = true)}>Elegir acto del recurso</button
      >{/if}
  </section>
  <fieldset class="case-offenses" disabled={disabled || pending}>
    <legend>Base de se&#241;alamiento</legend>
    <label
      >Base de senalamiento<textarea rows="3" maxlength="1000" bind:value={fields.statement}
      ></textarea></label
    >
    <label
      >Soporte admitido del senalamiento<select bind:value={fields.supportKey}>
        <option value="">Selecciona una version exacta admitida</option>
        {#each supports as row (supportKey(row))}<option value={supportKey(row)}
            >{row.name} / Version {row.version}</option
          >{/each}
      </select></label
    >
    {#if supportError}<p class="notice error" role="alert">{supportError}</p>{/if}
    <p class="hint">
      Solo se ofrecen soportes ya admitidos en las capturas seleccionadas del recurso y del acto.
    </p>
  </fieldset>
  <fieldset class="case-offenses">
    <legend>Participantes previstos ({participants.length}/32)</legend>
    <HearingParticipants
      rows={participants}
      disabled={disabled || pending}
      onremove={(id) => (participants = participants.filter((row) => row.id !== id))}
    />
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending || participants.length >= 32}
      onclick={() => (choosingParticipant = true)}>Elegir participante</button
    >
    {#if choosingParticipant}<HearingParticipantPicker
        api={directory}
        typedApi={typed}
        bind:this={participantPicker}
        draft={participantInputs}
        {canApply}
        {ondenied}
        selectedIds={participants.map((row) => row.id)}
        disabled={disabled || actBusy}
        bind:busy={participantBusy}
        onselected={chooseParticipant}
        oncancel={closeParticipant}
      />{/if}
  </fieldset>
</div>
