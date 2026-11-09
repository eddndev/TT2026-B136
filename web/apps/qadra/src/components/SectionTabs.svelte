<script>
  // Section tabs: a select on small screens and an underlined row above the
  // breakpoint. Mode "nav" switches app views (links with aria-current);
  // mode "tabs" switches panels inside one view (ARIA tablist, arrow keys).
  // The row never scrolls sideways: tabs that do not fit move to a menu at
  // its end, and the current tab always stays in the row.
  import { onMount, tick } from 'svelte';
  import HeroIcon from './HeroIcon.svelte';
  import { dismissable } from '../lib/dismissable.mjs';
  import { fitTabs } from '../lib/section-tabs-fit.mjs';
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
  // Space between two tabs (gap-x-8) and a first guess for the menu button.
  const GAP = 32;
  let frame,
    row,
    more,
    trigger,
    open = false,
    widths = null,
    measuredFor = null,
    available = 0;
  // Full class names so Tailwind finds them in the source.
  const visibility = {
    sm: ['sm:hidden', 'hidden sm:block'],
    lg: ['lg:hidden', 'hidden lg:block'],
  };
  $: [selectShown, rowShown] = visibility[breakpoint];
  $: tabs = mode === 'tabs';
  $: active = items.findIndex((item) => item.id === current);
  // Until the widths are known every tab is rendered so it can be measured.
  $: fit =
    widths && measuredFor === items
      ? fitTabs(widths, available, GAP, more?.offsetWidth || 88, active)
      : { visible: items.map((_, index) => index), overflow: [] };
  $: shown = fit.visible.map((index) => items[index]);
  $: hidden = fit.overflow.map((index) => items[index]);
  $: if (!hidden.length) open = false;
  $: if (frame && measuredFor !== items) measure();
  const tabClass = (selected) =>
    `group inline-flex shrink-0 items-center border-b-2 px-1 py-4 text-label whitespace-nowrap no-underline focus-visible:-outline-offset-2 ${
      selected
        ? 'border-brand-800 text-brand-800'
        : 'border-transparent text-muted hover:border-line-strong hover:text-ink-secondary'
    }`;
  const iconClass = (selected) =>
    `mr-2 -ml-0.5 size-5 ${selected ? 'text-brand-800' : 'text-muted group-hover:text-ink-secondary'}`;
  const menuClass =
    'flex w-full items-center gap-x-2 px-3 py-2 text-left text-body text-ink no-underline hover:bg-canvas focus:bg-canvas focus:outline-hidden';
  async function measure() {
    // The row is hidden below the breakpoint; it is measured once it shows.
    if (!frame?.clientWidth) return;
    const subject = items;
    widths = null;
    await tick();
    if (!row || subject !== items) return;
    available = frame.clientWidth;
    widths = [...row.children].map((node) => node.getBoundingClientRect().width);
    measuredFor = subject;
  }
  function resized() {
    if (!frame?.clientWidth) return;
    if (!widths || measuredFor !== items) measure();
    else available = frame.clientWidth;
  }
  onMount(() => {
    const observer = new ResizeObserver(resized);
    observer.observe(frame);
    // Label widths change when the web font replaces the fallback.
    document.fonts?.ready.then(() => frame && measure());
    return () => observer.disconnect();
  });
  function select(event, id) {
    event.preventDefault();
    open = false;
    onselect(id);
  }
  function close(event) {
    open = false;
    if (event?.key === 'Escape') trigger?.focus();
  }
  async function move(event, index) {
    const last = shown.length - 1;
    const next = {
      ArrowRight: index === last ? 0 : index + 1,
      ArrowLeft: index === 0 ? last : index - 1,
      Home: 0,
      End: last,
    }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    const id = shown[next].id;
    onselect(id);
    await tick();
    document.getElementById(`${idPrefix}-tab-${id}`)?.focus();
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
    <div bind:this={frame} class="border-b border-line">
      <svelte:element
        this={tabs ? 'div' : 'nav'}
        aria-label={tabs ? undefined : label}
        class="-mb-px flex gap-x-8"
      >
        <div
          bind:this={row}
          role={tabs ? 'tablist' : undefined}
          aria-label={tabs ? label : undefined}
          class="flex min-w-0 flex-1 gap-x-8 overflow-hidden"
        >
          {#each shown as item, index (item.id)}
            {@const selected = current === item.id}
            {#if tabs}
              <button
                id={`${idPrefix}-tab-${item.id}`}
                type="button"
                role="tab"
                aria-selected={selected}
                aria-controls={`${idPrefix}-panel-${item.id}`}
                tabindex={selected ? 0 : -1}
                class={tabClass(selected)}
                onclick={() => onselect(item.id)}
                onkeydown={(event) => move(event, index)}
                >{#if item.icon}<HeroIcon name={item.icon} class={iconClass(selected)} />{/if}<span
                  >{item.label}</span
                ></button
              >
            {:else}
              <a
                href={`#${item.id}`}
                aria-current={selected ? 'page' : undefined}
                class={tabClass(selected)}
                onclick={(event) => select(event, item.id)}
                >{#if item.icon}<HeroIcon name={item.icon} class={iconClass(selected)} />{/if}<span
                  >{item.label}</span
                ></a
              >
            {/if}
          {/each}
        </div>
        {#if hidden.length}
          <div bind:this={more} class="relative shrink-0" use:dismissable={close}>
            <button
              bind:this={trigger}
              type="button"
              class={tabClass(false)}
              aria-expanded={open}
              aria-controls={`${idPrefix}-more`}
              aria-label={`M\u00e1s ${label.toLowerCase()}`}
              onclick={() => (open = !open)}
              ><span>M&#225;s</span><HeroIcon
                name="chevron"
                class="ml-1 size-4 text-muted group-hover:text-ink-secondary"
              /></button
            >
            {#if open}
              <div
                id={`${idPrefix}-more`}
                class="absolute right-0 z-20 mt-1 w-60 rounded-md bg-surface py-1 shadow-lg outline-1 outline-ink/5"
              >
                {#each hidden as item (item.id)}
                  {#if tabs}
                    <button
                      type="button"
                      class={menuClass}
                      onclick={(event) => select(event, item.id)}
                      >{#if item.icon}<HeroIcon
                          name={item.icon}
                          class="size-5 text-muted"
                        />{/if}{item.label}</button
                    >
                  {:else}
                    <a
                      href={`#${item.id}`}
                      class={menuClass}
                      onclick={(event) => select(event, item.id)}
                      >{#if item.icon}<HeroIcon
                          name={item.icon}
                          class="size-5 text-muted"
                        />{/if}{item.label}</a
                    >
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </svelte:element>
    </div>
  </div>
</div>
