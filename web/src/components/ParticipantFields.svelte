<script>
  import { participantFields, participantValues } from '../lib/participants.mjs';
  export let draft;
  export let disabled = false;
  let failure = null;
  const inputs = {};
  export function reset() {
    failure = null;
  }
  export function values() {
    failure = null;
    try {
      return participantValues(draft);
    } catch (error) {
      failure = error;
      inputs[error.field]?.focus();
      throw error;
    }
  }
</script>

{#each participantFields as field}
  <label
    >{field.label}<input
      bind:this={inputs[field.key]}
      bind:value={draft[field.key]}
      {disabled}
      aria-invalid={failure?.field === field.key ? 'true' : undefined}
      aria-describedby={`participant-${field.key}-hint${failure?.field === field.key ? ` participant-${field.key}-error` : ''}`}
    /></label
  >
  <small id={`participant-${field.key}-hint`}
    >Hasta {field.limit} caracteres{field.required ? '; campo obligatorio.' : '.'}</small
  >
  {#if failure?.field === field.key}<p
      class="notice error"
      role="alert"
      id={`participant-${field.key}-error`}
    >
      {failure.message}
    </p>{/if}
{/each}
