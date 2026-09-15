<script>
  import ParticipantSummary from './ParticipantSummary.svelte';
  import ParticipantHistory from './ParticipantHistory.svelte';
  import ParticipantStatus from './ParticipantStatus.svelte';
  import { canParticipants } from '../lib/participants.mjs';
  export let api;
  export let user;
  export let record;
  export let onedit;
  export let onobserved;
  export let onstatus;
  export let ondenied;
  let showHistory = false;
  let statusDialog;
</script>

<section class="card participant-detail" aria-label="Datos del participante">
  <div class="section-heading">
    <div>
      <span class="eyebrow">DATOS DEL PARTICIPANTE</span>
      <h2>{record.display_name}</h2>
    </div>
    {#if canParticipants(user.role, 'manage')}<div class="action-row">
        <button class="secondary" onclick={() => onedit(record)}>Editar participante</button><button
          class="text-button"
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
    onclick={() => (showHistory = !showHistory)}
    >{showHistory ? 'Ocultar historial de cambios' : 'Ver historial de cambios'}</button
  >
  {#if showHistory}{#key record.revision}<ParticipantHistory
        {api}
        id={record.id}
        {ondenied}
      />{/key}{/if}
</section>
{#if canParticipants(user.role, 'manage')}<ParticipantStatus
    bind:this={statusDialog}
    {api}
    current={record}
    {onobserved}
    onconfirmed={onstatus}
    {ondenied}
  />{/if}
