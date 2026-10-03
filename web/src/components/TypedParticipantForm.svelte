<script>
  import ParticipantSubjectFields from './ParticipantSubjectFields.svelte';
  import ParticipantRoleFields from './ParticipantRoleFields.svelte';
  import ParticipantSubjectPicker from './ParticipantSubjectPicker.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  export let api,
    docs,
    caseId,
    subject,
    selected = null,
    role,
    fixedSubject = false,
    ondenied,
    disabled = false,
    pending = false;
  export let draft = null,
    supportContext = () => null,
    discardPath = () => {},
    canApply = () => true;
  let picker,
    pickerDraft = draft?.picker ?? null;
  let identityBusy = false,
    roleBusy = false,
    pickerBusy = false,
    picking = false;
  $: pending = identityBusy || roleBusy || pickerBusy;
  export function captureDraft() {
    return { picker: picker?.captureDraft() ?? pickerDraft };
  }
  function closePicker() {
    pickerDraft = picker?.captureDraft() ?? pickerDraft;
    picking = false;
  }
  function selectIdentity(record) {
    if (!canApply()) return;
    discardPath(['subject']);
    selected = record;
    closePicker();
  }
</script>

<div class="stack">
  {#if !fixedSubject}<div class="action-row">
      <button
        type="button"
        class="secondary"
        disabled={disabled || pending}
        onclick={() => (picking = true)}>Elegir identidad existente</button
      >
      {#if selected}<button
          type="button"
          class="text-button"
          disabled={disabled || pending}
          onclick={() => {
            if (canApply()) {
              discardPath(['subject']);
              selected = null;
            }
          }}>Registrar otra identidad</button
        >{/if}
    </div>{/if}
  {#if picking}<ParticipantSubjectPicker
      {api}
      {ondenied}
      bind:this={picker}
      draft={pickerDraft}
      {canApply}
      disabled={disabled || identityBusy || roleBusy}
      bind:busy={pickerBusy}
      onselected={selectIdentity}
      oncancel={closePicker}
    />{/if}
  {#if selected}<section class="participant-comparison">
      <h3>Identidad elegida</h3>
      <ParticipantSubjectSummary record={selected} />
      <p class="hint">
        Esta ficha conservar&#225; la revisi&#243;n elegida. Los cambios de identidad se registran
        por separado.
      </p>
    </section>
  {:else}<ParticipantSubjectFields
      bind:draft={subject}
      {docs}
      {caseId}
      {ondenied}
      draftContext={supportContext(['subject', 'identity_support'])}
      disabled={disabled || roleBusy || pickerBusy}
      bind:pending={identityBusy}
    />{/if}
  <ParticipantRoleFields
    bind:draft={role}
    {docs}
    {caseId}
    {ondenied}
    {supportContext}
    {discardPath}
    disabled={disabled || identityBusy || pickerBusy}
    bind:pending={roleBusy}
  />
</div>
