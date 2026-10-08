<script>
  import { onDestroy } from 'svelte';
  import HearingAnchorPicker from './HearingAnchorPicker.svelte';
  import PrecautionaryAnchorPicker from './PrecautionaryAnchorPicker.svelte';
  import { hearingTimeLabel } from '../lib/hearings.mjs';
  export let api,
    caseId,
    value,
    ondenied,
    inputs = null,
    disabled = false,
    pending = false,
    canApply = () => true;
  const hearings = api.caseHearings(caseId);
  const initial = {
    ...hearings,
    async list(query) {
      const page = await hearings.list(query);
      return { ...page, hearings: page.hearings.filter((row) => row.kind === 'initial') };
    },
  };
  let kind = value?.kind ?? inputs?.kind ?? 'independent',
    choosing = inputs?.choosing ?? false;
  export function captureInputs() {
    return { kind, choosing };
  }
  function select(record) {
    if (!canApply() || disabled) return;
    if (kind === 'initial' && (record.case_id !== caseId || record.values.kind !== 'initial'))
      return;
    value = { kind, record: structuredClone(record) };
    choosing = false;
  }
  onDestroy(() => {
    hearings.dispose();
    pending = false;
  });
</script>

<fieldset class="case-offenses" {disabled}>
  <legend>Audiencia de origen</legend>
  <label
    >Vinculo de audiencia<select
      bind:value={kind}
      disabled={pending}
      onchange={(event) => {
        kind = event.currentTarget.value;
        value = null;
        choosing = kind !== 'independent';
      }}
    >
      <option value="independent">Decision independiente</option>
      <option value="initial">Audiencia inicial existente</option>
      <option value="precautionary">Audiencia cautelar existente</option>
    </select></label
  >
  {#if value}
    <p>
      {kind === 'initial' ? 'Audiencia inicial' : 'Audiencia cautelar'} / Revision
      {kind === 'initial' ? value.record.revision : value.record.capture.review.result_revision}
    </p>
    <p>
      {hearingTimeLabel(
        kind === 'initial'
          ? value.record.values.scheduled_at
          : value.record.capture.review.resolved_values.scheduled_at,
      )}
    </p>
  {/if}
  {#if kind !== 'independent'}<button
      class="secondary"
      disabled={disabled || pending}
      onclick={() => (choosing = true)}>Elegir audiencia de origen</button
    >{/if}
  {#if choosing && kind === 'initial'}<HearingAnchorPicker
      api={initial}
      {disabled}
      {ondenied}
      {canApply}
      bind:busy={pending}
      onselected={select}
      oncancel={() => (choosing = false)}
    />
  {:else if choosing && kind === 'precautionary'}<PrecautionaryAnchorPicker
      {api}
      {caseId}
      {disabled}
      {ondenied}
      {canApply}
      bind:busy={pending}
      onselected={select}
      oncancel={() => (choosing = false)}
    />{/if}
  {#if kind !== 'independent' && !value}<p class="notice">
      Selecciona la revision exacta de la audiencia.
    </p>{/if}
</fieldset>
