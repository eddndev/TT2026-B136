<script context="module">
  // Gives every grid its own heading id for aria-labelledby.
  let sequence = 0;
</script>

<script>
  import { civilLabel } from '../lib/judicial-calendar-time.mjs';
  import { dayTone, monthCells, monthTitle, weekdayInitials } from '../lib/month-grid.mjs';
  // One month of the shared calendar: a title, the weekday initials and a
  // six-week grid. Only the days of the month are buttons; the neighbouring
  // days are inert so a day name never appears twice on the page.
  export let month;
  // Civil dates ('YYYY-MM-DD') or null.
  export let today = null;
  export let selected = null;
  // describe(date) returns { label, tone?, marker?, attrs? }. label is the
  // whole accessible name of the day; today and selection are only expressed
  // through aria-current and aria-pressed. marker is { text?, dot? } and is
  // hidden from assistive technology, so label must carry its meaning. attrs
  // are spread onto the day button.
  export let describe = (date) => ({ label: civilLabel(date) });
  export let onselect = () => {};
  export let disabled = false;
  export let headingLevel = 3;
  let className = '';
  export { className as class };
  const headingId = `month-grid-${++sequence}`;
  const rounding = {
    tl: 'rounded-tl-lg',
    tr: 'rounded-tr-lg',
    bl: 'rounded-bl-lg',
    br: 'rounded-br-lg',
  };
  $: cells = monthCells(month, today);
  function circle(isToday, pressed) {
    return [
      'mx-auto flex size-7 items-center justify-center rounded-full',
      isToday && 'bg-brand-800 font-medium text-white',
      pressed && !isToday && 'font-medium text-brand-900 ring-2 ring-brand-800',
      pressed && isToday && 'ring-2 ring-brand-800 ring-offset-1',
    ]
      .filter(Boolean)
      .join(' ');
  }
</script>

<div class={className}>
  <svelte:element this={`h${headingLevel}`} id={headingId} class="text-center text-label text-ink"
    >{monthTitle(month)}</svelte:element
  >
  <div class="mt-6 grid grid-cols-7 text-center text-caption leading-6 text-muted">
    {#each weekdayInitials as weekday (weekday.name)}
      <div>
        <span aria-hidden="true">{weekday.initial}</span><span class="sr-only">{weekday.name}</span>
      </div>
    {/each}
  </div>
  <div
    role="group"
    aria-labelledby={headingId}
    class="isolate mt-2 grid grid-cols-7 gap-px rounded-lg bg-line text-body tabular-nums shadow-sm ring-1 ring-line"
  >
    {#each cells as cell, index (index)}
      {#if cell.inMonth}
        {@const info = describe(cell.date)}
        {@const pressed = cell.date === selected}
        <button
          type="button"
          class={`py-1.5 focus:z-10 focus-visible:outline-2 focus-visible:outline-brand-800 enabled:hover:brightness-95 disabled:cursor-not-allowed ${dayTone(info.tone)} ${rounding[cell.corner] ?? ''}`}
          aria-label={info.label}
          aria-pressed={String(pressed)}
          aria-current={cell.isToday ? 'date' : undefined}
          {disabled}
          {...info.attrs ?? {}}
          onclick={() => onselect(cell.date)}
        >
          <time datetime={cell.date} class={circle(cell.isToday, pressed)}>{cell.day}</time>
          <span aria-hidden="true" class="flex h-3 items-center justify-center">
            {#if info.marker?.dot}
              <span class="size-1.5 rounded-full bg-brand-800"></span>
            {:else if info.marker?.text}
              <span class="text-caption leading-3 font-medium text-ink-secondary"
                >{info.marker.text}</span
              >
            {/if}
          </span>
        </button>
      {:else}
        <span
          aria-hidden="true"
          class={`bg-canvas py-1.5 text-muted ${rounding[cell.corner] ?? ''}`}
        >
          <span class="mx-auto flex size-7 items-center justify-center">{cell.day ?? ''}</span>
          <span class="block h-3"></span>
        </span>
      {/if}
    {/each}
  </div>
</div>
