<script>
  import AgendaItemCard from './AgendaItemCard.svelte';
  import { agendaDays } from '../lib/agenda-periods.mjs';
  import { agendaGroups, agendaIdentity } from './agenda-presentation.mjs';
  export let rows = [],
    range,
    view = 'week',
    offset = '+00:00',
    complete = false,
    onselect,
    disabled = false;
  const weekdays = [
    'Lunes',
    'Martes',
    'Mi\u00e9rcoles',
    'Jueves',
    'Viernes',
    'S\u00e1bado',
    'Domingo',
  ];
  $: days = agendaDays(range.from, range.until);
  $: leading = (new Date(`${range.from}T00:00:00Z`).getUTCDay() + 6) % 7;
  $: trailing = (7 - ((leading + days.length) % 7)) % 7;
  $: groups = agendaGroups(rows, offset);
</script>

<section class="agenda-calendar" aria-label={view === 'month' ? 'Vista mensual' : 'Vista semanal'}>
  <p class="hint agenda-calendar-scroll-hint">
    En pantallas estrechas, desplaza el calendario horizontalmente.
  </p>
  <div class="agenda-calendar-scroll">
    <div class="agenda-calendar-board">
      <div class="agenda-weekdays" aria-hidden="true">
        {#each weekdays as day}<strong>{day}</strong>{/each}
      </div>
      <div class="agenda-calendar-grid">
        {#each Array(leading) as _}<div class="agenda-day-padding" aria-hidden="true"></div>{/each}
        {#each days as day, index (day)}
          <section
            class="agenda-day-cell"
            data-agenda-day={day}
            aria-label={`Actividades del ${day}`}
          >
            <h3><span class="agenda-day-weekday">{weekdays[(leading + index) % 7]}</span>{day}</h3>
            <div class="agenda-day-items">
              {#each groups.get(day) || [] as item (agendaIdentity(item))}
                <AgendaItemCard {item} {offset} {onselect} {disabled} compact />
              {/each}
            </div>
            {#if !groups.has(day)}<p class="hint">
                {complete ? 'Sin actividades.' : 'Sin actividades cargadas.'}
              </p>{/if}
          </section>
        {/each}
        {#each Array(trailing) as _}<div class="agenda-day-padding" aria-hidden="true"></div>{/each}
      </div>
    </div>
  </div>
</section>
