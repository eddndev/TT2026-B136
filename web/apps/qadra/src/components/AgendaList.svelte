<script>
  import AgendaItemCard from './AgendaItemCard.svelte';
  import { agendaDays } from '../lib/agenda-periods.mjs';
  import { agendaGroups, agendaIdentity } from './agenda-presentation.mjs';
  export let rows = [],
    range,
    view = 'day',
    offset = '+00:00',
    complete = false,
    onselect,
    disabled = false;
  $: groups = agendaGroups(rows, offset);
  $: days = view === 'day' ? agendaDays(range.from, range.until) : [...groups.keys()].sort();
</script>

<section class="agenda-list" aria-label={view === 'day' ? 'Vista diaria' : 'Vista de rango'}>
  {#each days as day (day)}
    <section class="agenda-day-list" data-agenda-day={day} aria-label={`Actividades del ${day}`}>
      <h3>{day} / UTC{offset}</h3>
      <div class="agenda-day-items">
        {#each groups.get(day) || [] as item (agendaIdentity(item))}
          <AgendaItemCard {item} {offset} {onselect} {disabled} />
        {/each}
      </div>
      {#if !groups.has(day)}<p class="hint">
          {complete ? 'Sin actividades.' : 'Sin actividades cargadas.'}
        </p>{/if}
    </section>
  {/each}
</section>
