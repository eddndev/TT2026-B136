<script>
  import OffenseFields from './OffenseFields.svelte';
  import { basicFields, profileFields, caseValues } from '../lib/case-administration.mjs';
  export let draft;
  export let complete = false;
  export let disabled = false;
  let offenses,
    container,
    error = '',
    invalid = '';
  export function values() {
    error = '';
    invalid = '';
    try {
      if (complete) offenses.flush();
      return caseValues(draft, complete);
    } catch (failure) {
      invalid = failure.field;
      error = failure.message;
      container?.querySelector(`[name="${invalid}"]`)?.focus();
      throw failure;
    }
  }
</script>

<div bind:this={container} class="stack">
  <div class="case-field-grid">
    {#each basicFields as field}<label
        >{field.label}<input
          name={field.key}
          bind:value={draft[field.key]}
          {disabled}
          aria-invalid={invalid === field.key}
        /></label
      >{/each}
  </div>
  {#if complete}<h3>Identificadores registrados</h3>
    <div class="case-field-grid">
      {#each profileFields.slice(0, 4) as field}<label
          >{field.label}<input
            name={field.key}
            bind:value={draft.profile[field.key]}
            {disabled}
            aria-invalid={invalid === field.key}
          /></label
        >{/each}
    </div>
    <p class="hint">
      Registra los identificadores y sus autoridades. Estos datos no se verifican ante
      instituciones.
    </p>
    <OffenseFields bind:this={offenses} bind:offenses={draft.profile.offenses} {disabled} />
    {#each profileFields.slice(4) as field}<label
        >{field.label}
        {#if field.multiline}<textarea
            name={field.key}
            rows="5"
            bind:value={draft.profile[field.key]}
            {disabled}
            aria-invalid={invalid === field.key}></textarea>
        {:else}<input
            name={field.key}
            bind:value={draft.profile[field.key]}
            {disabled}
            aria-invalid={invalid === field.key}
          />{/if}
      </label>{/each}
  {/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</div>
