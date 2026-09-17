<script>
  import Icon from './Icon.svelte';
  import {
    hearingKinds,
    hearingModalities,
    hearingStatus,
    hearingTimeLabel,
  } from '../lib/hearings.mjs';
  export let rows = [],
    onselect,
    disabled = false,
    showCase = false;
</script>

<div class="hearing-list">
  {#each rows as record (`${record.case_id}:${record.id}:${record.revision}`)}
    <button
      class="hearing-row case-card"
      {disabled}
      onclick={() => onselect(record)}
      aria-label={`Consultar audiencia ${record.id}`}
    >
      <span class="tile-icon"><Icon name="calendar" /></span>
      <span class="hearing-row-text">
        <strong>{hearingKinds[record.kind]?.label || record.kind}</strong>
        <span>{hearingTimeLabel(record.scheduled_at)}</span>
        {#if showCase}<span>{record.case_title} / {record.case_reference}</span>{/if}
        <small
          >{hearingModalities[record.modality]} / Revisi&#243;n {record.revision} / {record.participant_count}
          participantes vinculados</small
        >
        {#if record.case_status === 'closed'}<small>Expediente cerrado administrativamente</small
          >{/if}
      </span>
      <span class="badge" class:info={record.status === 'scheduled'}
        >{hearingStatus[record.status]}</span
      >
      <Icon name="arrow" size={18} />
    </button>
  {/each}
</div>
