<script>
  import FactPersonFields from './FactPersonFields.svelte';
  import FactProvenanceFields from './FactProvenanceFields.svelte';
  export let value,
    api,
    caseId,
    ondenied,
    disabled = false,
    pending = false;
  let representedBusy = false,
    representativeBusy = false,
    provenanceBusy = false;
  $: pending = representedBusy || representativeBusy || provenanceBusy;
  function kind(next) {
    value =
      next === 'not_recorded'
        ? { kind: next, reason: '' }
        : next === 'declared'
          ? {
              kind: next,
              represented: { kind: '' },
              representative: { kind: '' },
              scope: '',
              provenance: { kind: '' },
            }
          : { kind: '' };
  }
</script>

<fieldset class="case-offenses fact-representation-fields" {disabled}>
  <legend>Representaci&#243;n declarada</legend>
  <label
    >Representaci&#243;n declarada<select
      value={value?.kind || ''}
      onchange={(event) => kind(event.currentTarget.value)}
      disabled={pending}
    >
      <option value="">Selecciona lo declarado</option><option value="not_recorded"
        >No registrada</option
      ><option value="declared">V&#237;nculo declarado expresamente</option>
    </select></label
  >
  {#if value?.kind === 'not_recorded'}<label
      >Motivo: Representaci&#243;n declarada<textarea
        rows="2"
        bind:value={value.reason}
        disabled={pending}></textarea></label
    >
  {:else if value?.kind === 'declared'}
    <FactPersonFields
      bind:value={value.represented}
      label="Persona representada"
      {api}
      {caseId}
      {ondenied}
      declared={false}
      disabled={disabled || representativeBusy || provenanceBusy}
      bind:pending={representedBusy}
    />
    <FactPersonFields
      bind:value={value.representative}
      label="Persona representante"
      {api}
      {caseId}
      {ondenied}
      declared={false}
      disabled={disabled || representedBusy || provenanceBusy}
      bind:pending={representativeBusy}
    />
    <label
      >Alcance de la representaci&#243;n<textarea
        rows="3"
        bind:value={value.scope}
        disabled={pending}></textarea></label
    >
    <FactProvenanceFields
      bind:value={value.provenance}
      label="Procedencia de la representaci&#243;n"
      {api}
      {caseId}
      {ondenied}
      disabled={disabled || representedBusy || representativeBusy}
      bind:pending={provenanceBusy}
    />
    <p class="hint">
      El v&#237;nculo es una declaraci&#243;n expresa. No se deduce del rol de las fichas ni
      acredita facultades legales.
    </p>
  {/if}
</fieldset>
