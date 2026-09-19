<script>
  import { onDestroy } from 'svelte';
  import FactDeclarationFields from './FactDeclarationFields.svelte';
  import FactTimeFields from './FactTimeFields.svelte';
  import FactSupportFields from './FactSupportFields.svelte';
  import ResourceModeFields from './ResourceModeFields.svelte';
  import ResourceResolutionFields from './ResourceResolutionFields.svelte';
  import ResourceAppellantFields from './ResourceAppellantFields.svelte';
  import { resourceKinds } from '../lib/procedural-resource-values.mjs';
  export let api,
    caseId,
    user,
    values,
    ondenied,
    disabled = false,
    busy = false;
  let resolutionBusy = false,
    supportBusy = false,
    appellantBusy = [];
  $: busy = resolutionBusy || supportBusy || appellantBusy.some(Boolean);
  $: selectedIds = values.appellants.map((v) => v.participant?.id).filter(Boolean);
  function addAppellant() {
    if (disabled || busy || values.appellants.length >= 32) return;
    values = {
      ...values,
      appellants: [...values.appellants, { name: '', role: { kind: '' }, participant: null }],
    };
  }
  function removeAppellant(index) {
    if (disabled || busy || values.appellants.length <= 1) return;
    values = { ...values, appellants: values.appellants.filter((_, i) => i !== index) };
    appellantBusy = [];
  }
  onDestroy(() => (busy = false));
</script>

<div class="case-field-grid">
  <label
    >Tipo de recurso<select bind:value={values.kind} disabled={disabled || busy}>
      <option value="">Selecciona el tipo declarado</option>
      {#each Object.entries(resourceKinds) as [key, label]}<option value={key}>{label}</option
        >{/each}
    </select></label
  >
  <ResourceModeFields
    bind:value={values.mode}
    label="Modalidad del recurso"
    disabled={disabled || busy}
  />
</div>
<label
  >Titulo organizativo<input
    bind:value={values.title}
    maxlength="200"
    disabled={disabled || busy}
  /></label
>
{#key `${caseId}:${user?.id}`}
  <ResourceResolutionFields
    {api}
    {caseId}
    bind:value={values.resolution}
    {ondenied}
    disabled={disabled || supportBusy || appellantBusy.some(Boolean)}
    bind:busy={resolutionBusy}
    onselected={() => (values.resolution_evidence = null)}
  />
  <FactSupportFields
    {api}
    {caseId}
    bind:value={values.resolution_evidence}
    {ondenied}
    required
    label="la resoluci&#243;n impugnada"
    disabled={disabled || resolutionBusy || appellantBusy.some(Boolean)}
    bind:pending={supportBusy}
  />
{/key}
<div class="case-field-grid">
  <FactDeclarationFields
    bind:value={values.resolution_reference}
    label="Referencia de la resoluci&#243;n"
    disabled={disabled || busy}
  />
  <FactDeclarationFields
    bind:value={values.issuing_authority}
    label="Autoridad emisora"
    disabled={disabled || busy}
  />
</div>
<label class="checkbox-label"
  ><input
    type="checkbox"
    checked={values.receiving_authority !== null}
    disabled={disabled || busy}
    onchange={(event) =>
      (values.receiving_authority = event.currentTarget.checked ? { kind: '' } : null)}
  />
  Registrar autoridad receptora</label
>
{#if values.receiving_authority !== null}<FactDeclarationFields
    bind:value={values.receiving_authority}
    label="Autoridad receptora"
    disabled={disabled || busy}
  />{/if}
<FactTimeFields
  bind:value={values.resolution_at}
  label="la resoluci&#243;n"
  disabled={disabled || busy}
/>
<label class="checkbox-label"
  ><input
    type="checkbox"
    checked={values.notification_at !== null}
    disabled={disabled || busy}
    onchange={(event) =>
      (values.notification_at = event.currentTarget.checked ? { precision: '' } : null)}
  />
  Registrar tiempo declarado de notificaci&#243;n</label
>
{#if values.notification_at !== null}<FactTimeFields
    bind:value={values.notification_at}
    label="la notificaci&#243;n"
    disabled={disabled || busy}
  />{/if}
<label
  >Parte impugnada<textarea
    rows="3"
    bind:value={values.challenged_part}
    maxlength="1000"
    disabled={disabled || busy}></textarea></label
>
<label
  >Motivos del recurso<textarea
    rows="3"
    bind:value={values.grounds}
    maxlength="1000"
    disabled={disabled || busy}></textarea></label
>
{#key `${caseId}:${user?.id}`}
  {#each values.appellants as appellant, index (appellant)}
    <ResourceAppellantFields
      {api}
      {caseId}
      bind:value={values.appellants[index]}
      number={index + 1}
      {selectedIds}
      {ondenied}
      disabled={disabled ||
        resolutionBusy ||
        supportBusy ||
        appellantBusy.some((v, i) => v && i !== index)}
      bind:busy={appellantBusy[index]}
    />
    {#if values.appellants.length > 1}<button
        type="button"
        class="text-button"
        disabled={disabled || busy}
        onclick={() => removeAppellant(index)}>Quitar recurrente {index + 1}</button
      >{/if}
  {/each}
{/key}
<button
  type="button"
  class="secondary"
  disabled={disabled || busy || values.appellants.length >= 32}
  onclick={addAppellant}>Agregar persona recurrente</button
>
<p class="hint">
  El registro organiza datos declarados y conserva sus fuentes. La interposici&#243;n se registra
  como acto separado.
</p>
