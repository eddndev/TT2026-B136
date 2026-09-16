<script>
  import ParticipantPlainField from './ParticipantPlainField.svelte';
  import ParticipantSupport from './ParticipantSupport.svelte';
  export let field,
    value,
    disabled = false,
    docs,
    caseId,
    ondenied,
    pending = false;
  function change(state) {
    value =
      state === 'known'
        ? { state, value: field.type === 'license' ? { number: '', issuer: '' } : '' }
        : state === 'documented'
          ? { state, support: null }
          : { state, reason: '' };
  }
</script>

{#if field.declared || ['contact', 'protection'].includes(field.type)}
  <label
    >Estado de {field.label}<select
      aria-label={`Estado de ${field.label}`}
      value={value.state}
      disabled={disabled || pending}
      onchange={(event) => change(event.currentTarget.value)}
    >
      {#if field.declared}<option value="known">Dato conocido</option>{:else}<option
          value="documented">Documentado</option
        ><option value={field.type === 'contact' ? 'not_recorded' : 'none_declared'}
          >{field.type === 'contact' ? 'No registrado' : 'Ninguna declarada'}</option
        >{/if}
      <option value="unknown">Dato desconocido</option>
    </select></label
  >
  {#if value.state === 'known'}<ParticipantPlainField {field} bind:value={value.value} {disabled} />
  {:else if value.state === 'documented'}<ParticipantSupport
      api={docs}
      {caseId}
      label={field.label}
      bind:value={value.support}
      {ondenied}
      {disabled}
      bind:pending
    />
  {:else}<label
      >Motivo de {field.label}<textarea rows="2" bind:value={value.reason} {disabled}
      ></textarea></label
    >{/if}
  {#if field.type === 'contact'}<p class="hint">
      El soporte describe un medio o una instrucci&#243;n declarada. Qadra no env&#237;a mensajes ni
      construye un canal de contacto.
    </p>{/if}
{:else}<ParticipantPlainField {field} bind:value {disabled} />{/if}
