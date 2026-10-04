<script>
  import { hearingKinds, hearingStatus, hearingTimeLabel } from '../lib/hearings.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { agendaRecord } from './agenda-presentation.mjs';
  import { resourceHearingKinds } from '../lib/resource-hearing-values.mjs';
  export let item,
    offset = '+00:00',
    onselect,
    disabled = false,
    compact = false;
  $: record = agendaRecord(item);
  $: hearing = item.kind === 'hearing';
  $: resourceHearing = item.kind === 'resource_hearing';
  $: label = resourceHearing ? 'Audiencia de recurso' : hearing ? 'Audiencia' : 'Plazo';
  $: context = hearing ? record : item;
  $: minutes =
    (Number(offset.slice(1, 3)) * 60 + Number(offset.slice(4))) * (offset[0] === '-' ? -1 : 1);
  $: projected = deadlineInstantLabel({ ...item.at, offset_seconds: minutes * 60 });
  $: title = resourceHearing
    ? resourceHearingKinds[record.kind]
    : hearing
      ? hearingKinds[record.kind]?.label || record.kind
      : record.title;
  $: compactTime = projected.split(' ')[1].replace(/\.0{9}$/, '');
  $: descriptionId = `agenda-${item.kind}-${record.id}-description`;
</script>

<button
  class="case-card agenda-item-card"
  class:compact
  {disabled}
  data-agenda-kind={item.kind}
  aria-label={`Consultar ${label.toLowerCase()} ${record.id}`}
  aria-describedby={compact ? descriptionId : undefined}
  onclick={() => onselect(item)}
>
  <span class="agenda-item-header">
    <span class="badge info">{label}</span>
    <small>{compact ? `R${record.revision}` : `Revisi\u00f3n ${record.revision}`}</small>
  </span>
  {#if compact}
    <span class="agenda-item-time agenda-item-clock" title={projected}>{compactTime}</span>
    <strong class="agenda-item-title" {title}>{title}</strong>
    <span
      class="agenda-item-case agenda-item-reference"
      title={`${context.case_title} / ${context.case_reference}`}>{context.case_reference}</span
    >
    {#if hearing && record.status === 'cancelled'}
      <small class="agenda-item-state">Cancelada</small>
    {/if}
  {:else}
    <strong>{title}</strong>
    <span class="agenda-item-time"
      >{hearing || resourceHearing ? 'Hora en la agenda' : 'Vencimiento operativo'}: {projected}</span
    >
    <span class="agenda-item-case">{context.case_title} / {context.case_reference}</span>
    {#if hearing}
      <small>{hearingStatus[record.status]} / {hearingTimeLabel(record.scheduled_at)}</small>
    {:else if resourceHearing}
      <small>Horario declarado / {hearingTimeLabel(record.scheduled_at)}</small>
    {:else}
      <small>Responsable: {record.responsible.email}</small>
      <small
        >{record.attention_recorded ? 'Atenci\u00f3n declarada' : 'Atenci\u00f3n pendiente'}</small
      >
    {/if}
    {#if context.case_status === 'closed'}<small>Expediente cerrado administrativamente</small>{/if}
  {/if}
  {#if compact}
    <span class="sr-only" id={descriptionId}>
      {title}. {projected}. {context.case_title} / {context.case_reference}. Revisi&#243;n {record.revision}.
      {#if hearing}{hearingStatus[record.status]}.{/if}
    </span>
  {/if}
</button>
