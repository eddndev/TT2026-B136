<script>
  import ParticipantDeclaredField from './ParticipantDeclaredField.svelte';
  import ParticipantSupport from './ParticipantSupport.svelte';
  import { profileKinds, roleDraft } from '../lib/typed-participant-values.mjs';
  export let draft,
    docs,
    caseId,
    ondenied,
    disabled = false,
    pending = false;
  export let supportContext = () => null,
    discardPath = () => {};
  let supportBusy = false,
    fieldsBusy = {};
  $: pending = supportBusy || Object.values(fieldsBusy).some(Boolean);
  $: fields = profileKinds.find((item) => item.key === draft.profile.kind)?.fields || [];
  function kind(value) {
    discardPath(['role', 'profile']);
    draft = { ...draft, profile: roleDraft(value).profile };
    fieldsBusy = {};
  }
</script>

<fieldset class="case-offenses">
  <legend>Ficha de participaci&#243;n</legend>
  <div class="stack">
    <label
      >Tipo de participante<select
        aria-label="Tipo de participante"
        value={draft.profile.kind}
        disabled={disabled || pending}
        onchange={(event) => kind(event.currentTarget.value)}
        ><option value="">Selecciona un tipo</option>{#each profileKinds as profile}<option
            value={profile.key}>{profile.label}</option
          >{/each}</select
      ></label
    >
    {#key draft.profile.kind}{#each fields as field (field.key)}<ParticipantDeclaredField
          {field}
          bind:value={draft.profile[field.key]}
          {docs}
          {caseId}
          {ondenied}
          draftContext={supportContext([
            'role',
            'profile',
            draft.profile.kind,
            field.key,
            'support',
          ])}
          ondiscard={() => discardPath(['role', 'profile', draft.profile.kind, field.key])}
          disabled={disabled ||
            supportBusy ||
            Object.entries(fieldsBusy).some(([key, value]) => key !== field.key && value)}
          bind:pending={fieldsBusy[field.key]}
        />{/each}{/key}
    <label
      >Organizaci&#243;n (opcional)<input
        bind:value={draft.organization}
        disabled={disabled || pending}
      /></label
    >
    <label
      >Situaci&#243;n jur&#237;dica declarada (opcional)<input
        bind:value={draft.legal_status}
        disabled={disabled || pending}
      /></label
    >
    <ParticipantSupport
      api={docs}
      {caseId}
      label="Soporte del rol"
      draftContext={supportContext(['role', 'role_support'])}
      bind:value={draft.role_support}
      {ondenied}
      disabled={disabled || Object.values(fieldsBusy).some(Boolean)}
      bind:pending={supportBusy}
    />
  </div>
</fieldset>
