<script>
  import { onDestroy } from 'svelte';
  import FactDeclarationFields from './FactDeclarationFields.svelte';
  import FactTimeFields from './FactTimeFields.svelte';
  import FactSupportFields from './FactSupportFields.svelte';
  import ResourceModeFields from './ResourceModeFields.svelte';
  import { resourceActKinds } from '../lib/procedural-resource-values.mjs';
  export let api,
    caseId,
    user,
    values,
    ondenied,
    disabled = false,
    busy = false;
  let supportBusy = [];
  $: busy = supportBusy.some(Boolean);
  function add() {
    if (disabled || busy || values.evidence.length >= 2) return;
    values = { ...values, evidence: [...values.evidence, null] };
  }
  function remove(index) {
    if (disabled || busy || values.evidence.length <= 1 || index === 0) return;
    values = { ...values, evidence: values.evidence.filter((_, i) => i !== index) };
    supportBusy = [];
  }
  onDestroy(() => (busy = false));
</script>

<div class="case-field-grid">
  <label
    >Tipo de acto<select bind:value={values.kind} disabled={disabled || busy}>
      <option value="">Selecciona el acto declarado</option>
      {#each Object.entries(resourceActKinds) as [key, label]}<option value={key}>{label}</option
        >{/each}
    </select></label
  >
  <ResourceModeFields
    bind:value={values.mode}
    label="Modalidad del acto"
    disabled={disabled || busy}
  />
</div>
<FactTimeFields bind:value={values.occurred_at} label="este acto" disabled={disabled || busy} />
<FactDeclarationFields
  bind:value={values.authority}
  label="Autoridad del acto"
  disabled={disabled || busy}
/>
<label
  >Declaraci&#243;n del acto<textarea
    rows="4"
    bind:value={values.statement}
    maxlength="1000"
    disabled={disabled || busy}></textarea></label
>
{#key `${caseId}:${user?.id}`}
  {#each values.evidence as support, index}
    <FactSupportFields
      {api}
      {caseId}
      bind:value={values.evidence[index]}
      {ondenied}
      required
      label={`acto ${index + 1}`}
      disabled={disabled || supportBusy.some((v, i) => v && i !== index)}
      bind:pending={supportBusy[index]}
    />
    {#if index > 0}<button
        type="button"
        class="text-button"
        disabled={disabled || busy}
        onclick={() => remove(index)}>Quitar soporte adicional {index + 1}</button
      >{/if}
  {/each}
{/key}
<button
  type="button"
  class="secondary"
  disabled={disabled || busy || values.evidence.length >= 2}
  onclick={add}>Agregar soporte del acto</button
>
<p class="hint">
  Declara el acto y su constancia exacta. Modalidad oral no equivale a ausencia de soporte; el
  registro no certifica efectos procesales.
</p>
