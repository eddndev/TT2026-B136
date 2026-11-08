<script context="module">
  // Gives every list its own heading id for aria-labelledby.
  let sequence = 0;
</script>

<script>
  // A dated list of records under a title, each row one button that opens
  // its record. A row is { key, value, label, pressed?, attrs?, date,
  // dateText, badge?, title, status?, meta?, time?, timeText?, description? }:
  // label is the accessible name, pressed (when given) becomes aria-pressed,
  // attrs are spread onto the button, time is { datetime, text } and
  // timeText is shown instead when there is no single instant.
  export let title;
  export let level = 2;
  export let rows = [];
  // Muted text for an empty list, or null to show nothing.
  export let empty = null;
  export let onselect = () => {};
  export let disabled = false;
  const headingId = `upcoming-list-${++sequence}`;
</script>

<section aria-labelledby={headingId}>
  <svelte:element this={`h${level}`} id={headingId} class="text-base font-semibold text-ink"
    >{title}</svelte:element
  >
  <slot name="header" />
  {#if rows.length}
    <ol class="mt-2 divide-y divide-line text-sm/6 text-slate-500">
      {#each rows as row (row.key)}
        <li class="py-4">
          <button
            type="button"
            class="w-full rounded-md text-left focus-visible:outline-2 focus-visible:outline-brand-800 enabled:hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-60 sm:flex sm:items-start sm:gap-x-4"
            aria-label={row.label}
            aria-pressed={row.pressed === undefined ? undefined : String(row.pressed)}
            {disabled}
            {...row.attrs ?? {}}
            onclick={() => onselect(row.value)}
          >
            <time datetime={row.date} class="block sm:w-40 sm:flex-none">{row.dateText}</time>
            <span class="mt-1 block min-w-0 flex-auto sm:mt-0">
              <span class="flex flex-wrap items-center gap-x-2">
                {#if row.badge}
                  <span
                    class="rounded-md bg-brand-50 px-1.5 text-xs font-medium text-brand-800 ring-1 ring-brand-200 ring-inset"
                    >{row.badge}</span
                  >
                {/if}
                <span class="font-semibold text-ink">{row.title}</span>
                {#if row.status}
                  <span class="text-xs font-semibold text-amber-800">{row.status}</span>
                {/if}
              </span>
              {#each row.meta ?? [] as line, index (index)}
                <span class="block">{line}</span>
              {/each}
            </span>
            {#if row.time}
              <time datetime={row.time.datetime} class="mt-1 block sm:mt-0 sm:ml-auto sm:flex-none"
                >{row.time.text}</time
              >
            {:else if row.timeText}
              <span class="mt-1 block sm:mt-0 sm:ml-auto sm:flex-none">{row.timeText}</span>
            {/if}
            {#if row.description}
              <span class="sr-only">{row.description}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ol>
  {:else if empty}
    <p class="py-4 text-sm text-muted">{empty}</p>
  {/if}
</section>
