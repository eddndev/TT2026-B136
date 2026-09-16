<script>
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import ParticipantSummary from './ParticipantSummary.svelte';
  import ParticipantHistory from './ParticipantHistory.svelte';
  import ParticipantStatus from './ParticipantStatus.svelte';
  import ParticipantSubjectReader from './ParticipantSubjectReader.svelte';
  import ParticipantCredentialEvidence from './ParticipantCredentialEvidence.svelte';
  export let typedApi, oncomplete, docs, caseId;
  let credentialBusy = false;
  let identityBusy = false;
  import { canParticipants } from '../lib/participants.mjs';
  export let api;
  export let user;
  export let record;
  export let onedit;
  export let onobserved;
  export let onstatus;
  export let ondenied;
  export let disabled = false;
  let showHistory = false;
  let statusDialog;
  let history;
  let historyBusy = false;
  export function refreshHistory(id) {
    if (record.id === id && showHistory) return history?.refresh();
  }
</script>

<section class="card participant-detail" aria-label="Datos del participante">
  <div class="section-heading">
    <div>
      <span class="eyebrow">DATOS DEL PARTICIPANTE</span>
      <h2>{record.display_name}</h2>
    </div>
    {#if canParticipants(user.role, 'manage')}<div class="action-row">
        <button
          class="secondary"
          disabled={$administration.closed ||
            disabled ||
            historyBusy ||
            identityBusy ||
            credentialBusy}
          onclick={() => onedit(record)}>Editar participante</button
        >{#if !record.profile}<button
            class="secondary"
            disabled={$administration.closed ||
              disabled ||
              historyBusy ||
              identityBusy ||
              credentialBusy}
            onclick={() => oncomplete(record)}>Completar perfil</button
          >{/if}<button
          class="text-button"
          disabled={$administration.closed ||
            disabled ||
            historyBusy ||
            identityBusy ||
            credentialBusy}
          onclick={() => statusDialog.open()}
          >{record.directory_status === 'active'
            ? 'Archivar participante'
            : 'Reactivar participante'}</button
        >
      </div>{/if}
  </div>
  <ParticipantSummary {record} showName={false} />
  <p class="hint participant-provenance">
    <span>Revisi&#243;n {record.revision}</span><span>{record.changed_by.email}</span><time
      datetime={record.changed_at}
      title={record.changed_at}>{new Date(record.changed_at).toLocaleString('es-MX')}</time
    >
  </p>
  <button
    class="text-button"
    aria-expanded={showHistory}
    disabled={disabled || historyBusy || identityBusy || credentialBusy}
    onclick={() => (showHistory = !showHistory)}
    >{showHistory ? 'Ocultar historial de cambios' : 'Ver historial de cambios'}</button
  >
  {#if record.subject}{#key record.subject.id}<ParticipantSubjectReader
        api={typedApi}
        {docs}
        {caseId}
        {user}
        record={record.subject}
        {ondenied}
        disabled={disabled || historyBusy || credentialBusy}
        bind:busy={identityBusy}
      />{/key}{/if}
  {#if record.credential_origin}{#key `${record.credential_origin.participant_revision}:${record.credential_origin.statement_digest}`}<ParticipantCredentialEvidence
        api={typedApi}
        reference={record.credential_origin}
        {ondenied}
        disabled={disabled || historyBusy || identityBusy}
        bind:busy={credentialBusy}
      />{/key}{/if}
  {#if showHistory}<ParticipantHistory
      bind:this={history}
      bind:busy={historyBusy}
      {api}
      {typedApi}
      disabled={disabled || identityBusy || credentialBusy}
      id={record.id}
      {ondenied}
    />{/if}
</section>
{#if canParticipants(user.role, 'manage')}<ParticipantStatus
    bind:this={statusDialog}
    {api}
    current={record}
    {onobserved}
    onconfirmed={onstatus}
    {ondenied}
  />{/if}
