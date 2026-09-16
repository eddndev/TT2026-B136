<script>
  import ParticipantDeclaredField from './ParticipantDeclaredField.svelte';
  import ParticipantSupport from './ParticipantSupport.svelte';
  import { subjectDraft } from '../lib/typed-participant-values.mjs';
  export let draft,
    docs,
    caseId,
    ondenied,
    disabled = false,
    pending = false,
    fixedKind = false;
  function kind(value) {
    draft =
      value === 'natural_person'
        ? subjectDraft()
        : {
            kind: value,
            name: '',
            institutional_identifier: { state: 'unknown', reason: '' },
            identity_support: null,
          };
  }
</script>

<fieldset class="case-offenses">
  <legend>Datos de identidad</legend>
  <div class="stack">
    <label
      >Tipo de identidad<select
        aria-label="Tipo de identidad"
        value={draft.kind}
        disabled={fixedKind || disabled || pending}
        onchange={(event) => kind(event.currentTarget.value)}
        ><option value="natural_person">Persona</option><option value="institutional_body"
          >&#211;rgano institucional</option
        ></select
      ></label
    >
    {#if draft.kind === 'natural_person'}
      <label
        >Estado del nombre<select
          aria-label="Estado del nombre"
          value={draft.name.state}
          disabled={disabled || pending}
          onchange={(event) =>
            (draft.name =
              event.currentTarget.value === 'known'
                ? { state: 'known', value: '' }
                : { state: 'unidentified', label: '', reason: '' })}
          ><option value="known">Nombre conocido</option><option value="unidentified"
            >Persona sin identificar</option
          ></select
        ></label
      >
      {#if draft.name.state === 'known'}<label
          >Nombre de la persona<input
            bind:value={draft.name.value}
            disabled={disabled || pending}
          /></label
        >
      {:else}<label
          >Etiqueta de la persona<input
            bind:value={draft.name.label}
            disabled={disabled || pending}
          /></label
        ><label
          >Motivo de identidad desconocida<textarea
            bind:value={draft.name.reason}
            disabled={disabled || pending}></textarea></label
        >{/if}
      <ParticipantDeclaredField
        field={{ key: 'curp', label: 'CURP', type: 'text', declared: true }}
        bind:value={draft.curp}
        disabled={disabled || pending}
      />
    {:else}<label
        >Nombre del &#243;rgano<input
          bind:value={draft.name}
          disabled={disabled || pending}
        /></label
      >
      <ParticipantDeclaredField
        field={{
          key: 'institutional_identifier',
          label: 'Identificador institucional',
          type: 'text',
          declared: true,
        }}
        bind:value={draft.institutional_identifier}
        disabled={disabled || pending}
      />
    {/if}
    <ParticipantSupport
      api={docs}
      {caseId}
      label="Soporte de identidad"
      bind:value={draft.identity_support}
      {ondenied}
      {disabled}
      bind:pending
    />
  </div>
</fieldset>
