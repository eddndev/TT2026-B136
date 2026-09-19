<script>
  import { hearingKinds, hearingStatus, hearingTimeLabel } from '../lib/hearings.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { agendaRecord } from './agenda-presentation.mjs';
  export let item,
    offset = '+00:00',
    onselect,
    disabled = false,
    compact = false;
  $: record = agendaRecord(item);
  $: hearing = item.kind === 'hearing';
  $: context = hearing ? record : item;
  $: minutes =
    (Number(offset.slice(1, 3)) * 60 + Number(offset.slice(4))) * (offset[0] === '-' ? -1 : 1);
  $: projected = deadlineInstantLabel({ ...item.at, offset_seconds: minutes * 60 });
</script>

<button
  class="case-card agenda-item-card"
  class:compact
  {disabled}
  data-agenda-kind={item.kind}
  aria-label={`Consultar ${hearing ? 'audiencia' : 'plazo'} ${record.id}`}
  onclick={() => onselect(item)}
>
  <span class="agenda-item-header">
    <span class="badge info">{hearing ? 'Audiencia' : 'Plazo'}</span>
    <small>Revisi&#243;n {record.revision}</small>
  </span>
  <strong>{hearing ? hearingKinds[record.kind]?.label || record.kind : record.title}</strong>
  <span class="agenda-item-time"
    >{hearing ? 'Hora en la agenda' : 'Vencimiento operativo'}: {projected}</span
  >
  <span class="agenda-item-case">{context.case_title} / {context.case_reference}</span>
  {#if hearing}
    <small>{hearingStatus[record.status]} / {hearingTimeLabel(record.scheduled_at)}</small>
  {:else}
    <small>Responsable: {record.responsible.email}</small>
    <small
      >{record.attention_recorded ? 'Atenci\u00f3n declarada' : 'Atenci\u00f3n pendiente'}</small
    >
  {/if}
  {#if context.case_status === 'closed'}<small>Expediente cerrado administrativamente</small>{/if}
</button>
