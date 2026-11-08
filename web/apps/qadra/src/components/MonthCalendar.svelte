<script>
  import Button from './Button.svelte';
  import HeroIcon from './HeroIcon.svelte';
  import MonthGrid from './MonthGrid.svelte';
  import { civilLabel } from '../lib/judicial-calendar-time.mjs';
  import { monthNavigation, visibleMonths } from '../lib/month-grid.mjs';
  // One or two months side by side (the second only from md up) with
  // previous/next controls at the top corners. The parent owns month and
  // receives the new first month through onmonth; min and max ('YYYY-MM')
  // optionally bound the navigation.
  export let month;
  export let months = 2;
  export let min = null,
    max = null;
  export let today = null,
    selected = null;
  // Passed to every MonthGrid; see MonthGrid.svelte for its shape.
  export let describe = (date) => ({ label: civilLabel(date) });
  export let onselect = () => {},
    onmonth = () => {};
  export let disabled = false;
  // aria-label of the calendar region.
  export let label;
  export let headingLevel = 3;
  export let previousLabel = 'Mes anterior',
    nextLabel = 'Mes siguiente';
  $: shown = visibleMonths(month, months);
  $: navigation = monthNavigation(month, { count: months, min, max });
</script>

<section
  aria-label={label}
  class={`relative grid grid-cols-1 gap-x-14 ${months === 2 ? 'md:grid-cols-2' : ''}`}
>
  <Button
    variant="icon"
    type="button"
    class="absolute -top-1 left-0 z-10"
    disabled={disabled || !navigation.previous}
    onclick={() => onmonth(navigation.previous)}
    ><HeroIcon name="chevronLeft" class="size-5" /><span class="sr-only">{previousLabel}</span
    ></Button
  >
  <Button
    variant="icon"
    type="button"
    class="absolute -top-1 right-0 z-10"
    disabled={disabled || !navigation.next}
    onclick={() => onmonth(navigation.next)}
    ><HeroIcon name="chevronRight" class="size-5" /><span class="sr-only">{nextLabel}</span></Button
  >
  {#each shown as value, index (value)}
    <MonthGrid
      month={value}
      class={`pt-1 ${index ? 'hidden md:block' : ''}`}
      {today}
      {selected}
      {describe}
      {onselect}
      {disabled}
      {headingLevel}
    />
  {/each}
</section>
