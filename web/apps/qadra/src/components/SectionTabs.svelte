<script>
  // Section tabs: a select on small screens and an underlined row above the
  // breakpoint. Mode "nav" switches app views (links with aria-current);
  // mode "tabs" switches panels inside one view (ARIA tablist, arrow keys).
  import HeroIcon from './HeroIcon.svelte';
  // Each item is { id, label, icon? } with icon a HeroIcon name.
  export let items;
  export let current;
  export let label;
  export let onselect;
  export let mode = 'nav';
  // Tablist mode: tabs get `${idPrefix}-tab-${id}` and control
  // `${idPrefix}-panel-${id}`, which the caller renders.
  export let idPrefix = 'section';
  // Width from which the row replaces the select.
  export let breakpoint = 'sm';
  let row;
  // Full class names so Tailwind finds them in the source.
  const visibility = {
    sm: ['sm:hidden', 'hidden sm:block'],
    lg: ['lg:hidden', 'hidden lg:block'],
  };
  $: [selectShown, rowShown] = visibility[breakpoint];
  const tabClass = (active) =>
    `group inline-flex shrink-0 items-center border-b-2 px-1 py-4 text-label whitespace-nowrap no-underline ${
      active
        ? 'border-brand-800 text-brand-800'
        : 'border-transparent text-muted hover:border-line-strong hover:text-ink-secondary'
    }`;
  const iconClass = (active) =>
    `mr-2 -ml-0.5 size-5 ${active ? 'text-brand-800' : 'text-muted group-hover:text-ink-secondary'}`;
  function move(event, index) {
    const last = items.length - 1;
    const next = {
      ArrowRight: index === last ? 0 : index + 1,
      ArrowLeft: index === 0 ? last : index - 1,
      Home: 0,
      End: last,
    }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    onselect(items[next].id);
    row.querySelectorAll('[role="tab"]')[next]?.focus();
  }
</script>

<div class="mb-6">
  <div class={`grid grid-cols-1 ${selectShown}`}>
    <select
      aria-label={label}
      value={current}
      onchange={(event) => onselect(event.currentTarget.value)}
      class="col-start-1 row-start-1 w-full appearance-none rounded-md border-0 bg-surface py-2 pr-8 pl-3 text-base text-ink outline-1 -outline-offset-1 outline-line-strong focus:outline-2 focus:-outline-offset-2 focus:outline-brand-800"
    >
      {#each items as item (item.id)}<option value={item.id}>{item.label}</option>{/each}
    </select>
    <HeroIcon
      name="chevron"
      class="pointer-events-none col-start-1 row-start-1 mr-2 size-5 self-center justify-self-end text-muted"
    />
  </div>
  <div class={rowShown}>
    <div class="border-b border-line">
      {#if mode === 'tabs'}
        <div
          bind:this={row}
          role="tablist"
          aria-label={label}
          class="-mb-px flex gap-x-8 overflow-x-auto"
        >
          {#each items as item, index (item.id)}
            {@const active = current === item.id}
            <button
              id={`${idPrefix}-tab-${item.id}`}
              role="tab"
              aria-selected={active}
              aria-controls={`${idPrefix}-panel-${item.id}`}
              tabindex={active ? 0 : -1}
              class={tabClass(active)}
              onclick={() => onselect(item.id)}
              onkeydown={(event) => move(event, index)}
              >{#if item.icon}<HeroIcon name={item.icon} class={iconClass(active)} />{/if}<span
                >{item.label}</span
              ></button
            >
          {/each}
        </div>
      {:else}
        <nav aria-label={label} class="-mb-px flex gap-x-8 overflow-x-auto">
          {#each items as item (item.id)}
            {@const active = current === item.id}
            <a
              href={`#${item.id}`}
              aria-current={active ? 'page' : undefined}
              class={tabClass(active)}
              onclick={(event) => {
                event.preventDefault();
                onselect(item.id);
              }}
              >{#if item.icon}<HeroIcon name={item.icon} class={iconClass(active)} />{/if}<span
                >{item.label}</span
              ></a
            >
          {/each}
        </nav>
      {/if}
    </div>
  </div>
</div>
