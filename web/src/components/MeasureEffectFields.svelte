<script>
  import { onDestroy } from 'svelte';
  import MeasureRecordPicker from './MeasureRecordPicker.svelte';
  import MeasureRecordSummary from './MeasureRecordSummary.svelte';
  import MeasureProposalFields from './MeasureProposalFields.svelte';
  import { newMeasureProposal, proposalFromRecord } from './measure-decision-editor-values.mjs';
  export let api,
    caseId,
    row,
    index,
    inputs = null,
    disabled = false,
    pending = false,
    ondenied,
    canApply = () => true;
  const measures = api.caseMeasures(caseId);
  const actions = {
    impose: 'Imponer',
    confirm: 'Confirmar',
    modify: 'Modificar',
    revoke: 'Revocar',
    cease: 'Cesar',
    substitute: 'Sustituir',
  };
  let choosing = inputs?.choosing ?? false,
    proposalInputs = inputs?.proposal ?? null,
    proposalGeneration = 0,
    pickerBusy = false,
    proposalBusy = false,
    proposalFields,
    successorFields = {},
    successorBusy = {};
  $: pending = pickerBusy || proposalBusy || Object.values(successorBusy).some(Boolean);
  export function captureInputs() {
    return {
      choosing,
      proposal: proposalFields?.captureInputs() ?? proposalInputs,
      successors: Object.fromEntries(
        row.successors.map((value) => [
          value.id,
          successorFields[value.id]?.captureInputs() ?? inputs?.successors?.[value.id] ?? null,
        ]),
      ),
    };
  }
  function replaceProposal(proposal) {
    proposalInputs = null;
    proposalGeneration++;
    return proposal;
  }
  function action(next) {
    if (!canApply() || disabled || pending) return;
    choosing = false;
    if (next === 'modify' && row.previous)
      row = {
        ...row,
        action: next,
        proposal: replaceProposal(proposalFromRecord(row.previous)),
      };
    else if (next === 'impose' && row.action === 'modify')
      row = { ...row, action: next, proposal: replaceProposal(newMeasureProposal()) };
    else row = { ...row, action: next };
  }
  function choose(value) {
    if (!canApply() || disabled || value.case_id !== caseId || value.validity !== 'valid') return;
    if (row.action === 'substitute') {
      if (
        row.predecessors.length + row.successors.length >= 32 ||
        row.predecessors.some((prior) => prior.reference.id === value.reference.id)
      )
        return;
      row = { ...row, predecessors: [...row.predecessors, structuredClone(value)] };
    } else {
      row = {
        ...row,
        previous: structuredClone(value),
        ...(row.action === 'modify'
          ? { proposal: replaceProposal(proposalFromRecord(value)) }
          : {}),
      };
    }
    choosing = false;
  }
  function removePrior(id) {
    if (!canApply() || disabled || pending) return;
    row = { ...row, predecessors: row.predecessors.filter((value) => value.reference.id !== id) };
  }
  function addSuccessor() {
    if (!canApply() || disabled || pending || row.predecessors.length + row.successors.length >= 32)
      return;
    row = { ...row, successors: [...row.successors, newMeasureProposal()] };
  }
  function removeSuccessor(id) {
    if (!canApply() || disabled || pending) return;
    row = { ...row, successors: row.successors.filter((value) => value.id !== id) };
    delete successorBusy[id];
    successorBusy = { ...successorBusy };
  }
  onDestroy(() => {
    measures.dispose();
    pending = false;
  });
</script>

<fieldset class="case-offenses" {disabled}>
  <legend>Efecto {index}</legend>
  <label
    >Accion de medida {index}<select
      value={row.action}
      disabled={disabled || pending}
      onchange={(event) => action(event.currentTarget.value)}
    >
      {#each Object.entries(actions) as [value, label]}<option {value}>{label}</option>{/each}
    </select></label
  >
  {#if row.action === 'impose'}
    {#key proposalGeneration}
      <MeasureProposalFields
        {api}
        {caseId}
        {ondenied}
        {canApply}
        bind:this={proposalFields}
        bind:value={row.proposal}
        bind:pending={proposalBusy}
        disabled={disabled || pickerBusy}
        inputs={proposalInputs}
      />
    {/key}
  {:else}
    {#if row.action === 'substitute'}
      <p>Selecciona las medidas anteriores y sus sustitutas del mismo sujeto declarado.</p>
      {#each row.predecessors as previous (previous.reference.id)}
        <MeasureRecordSummary value={previous} />
        <button
          type="button"
          class="text-button"
          disabled={disabled || pending}
          onclick={() => removePrior(previous.reference.id)}>Quitar medida anterior</button
        >
      {/each}
    {:else if row.previous}<MeasureRecordSummary value={row.previous} />{/if}
    <button
      type="button"
      class="secondary"
      disabled={disabled ||
        pending ||
        (row.action === 'substitute' && row.predecessors.length + row.successors.length >= 32)}
      onclick={() => {
        if (canApply()) choosing = true;
      }}>Elegir medida</button
    >
    {#if choosing}<MeasureRecordPicker
        api={measures}
        {ondenied}
        {canApply}
        disabled={disabled || proposalBusy || Object.values(successorBusy).some(Boolean)}
        selectedIds={row.action === 'substitute'
          ? row.predecessors.map((value) => value.reference.id)
          : []}
        bind:busy={pickerBusy}
        onselected={choose}
        oncancel={() => (choosing = false)}
      />{/if}
    {#if row.action === 'modify' && row.previous}
      {#key proposalGeneration}
        <MeasureProposalFields
          {api}
          {caseId}
          {ondenied}
          {canApply}
          lockedIdentity={true}
          bind:this={proposalFields}
          bind:value={row.proposal}
          bind:pending={proposalBusy}
          disabled={disabled || pickerBusy}
          inputs={proposalInputs}
        />
      {/key}
    {:else if row.action === 'substitute'}
      {#each row.successors as successor, position (successor.id)}
        <fieldset class="case-offenses">
          <legend>Medida sustituta {position + 1}</legend>
          <MeasureProposalFields
            {api}
            {caseId}
            {ondenied}
            {canApply}
            bind:this={successorFields[successor.id]}
            bind:value={row.successors[position]}
            bind:pending={successorBusy[successor.id]}
            disabled={disabled || pickerBusy}
            inputs={inputs?.successors?.[successor.id] ?? null}
          />
          <button
            type="button"
            class="text-button"
            disabled={disabled || pending}
            onclick={() => removeSuccessor(successor.id)}>Quitar medida sustituta</button
          >
        </fieldset>
      {/each}
      <button
        type="button"
        class="secondary"
        disabled={disabled || pending || row.predecessors.length + row.successors.length >= 32}
        onclick={addSuccessor}>Agregar medida sustituta</button
      >
    {/if}
  {/if}
</fieldset>
