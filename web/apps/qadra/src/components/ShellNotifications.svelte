<script>
  import { onDestroy, onMount } from 'svelte';
  import HeroIcon from './HeroIcon.svelte';
  import { dismissable } from '../lib/dismissable.mjs';
  import { alertKindLabel, alertTimeLabel, alertFailure } from '../lib/alerts-presentation.mjs';
  export let api;
  export let user;
  export let view;
  export let onnavigate;
  const SHOWN = 5,
    REFRESH_MS = 120000;
  let open = false,
    unread = [],
    more = false,
    loading = false,
    error = '',
    alive = true,
    generation = 0,
    timer,
    lastView = view;
  $: count = unread.length;
  $: countLabel = more ? `${count}+` : String(count);
  $: label = count ? `Avisos: ${countLabel} sin leer` : 'Avisos: sin pendientes';
  // The personal inbox already holds deadline and hearing alerts; the bell only
  // counts unread active ones and links to the full Alertas view.
  async function refresh() {
    const current = ++generation;
    let inbox;
    loading = true;
    try {
      inbox = api.alerts(user.id);
      const page = await inbox.list({ limit: 20, read: 'unread', state: 'active' });
      if (!alive || current !== generation) return;
      unread = page.alerts;
      more = page.has_more;
      error = '';
    } catch (failure) {
      if (alive && current === generation) error = alertFailure(failure);
    } finally {
      inbox?.dispose();
      if (alive && current === generation) loading = false;
    }
  }
  $: if (view !== lastView) {
    if (lastView === 'alerts') refresh();
    lastView = view;
  }
  function toggle() {
    open = !open;
    if (open) refresh();
  }
  function go() {
    open = false;
    onnavigate('alerts');
  }
  onMount(() => {
    refresh();
    timer = setInterval(() => {
      if (document.visibilityState === 'visible') refresh();
    }, REFRESH_MS);
  });
  onDestroy(() => {
    alive = false;
    generation++;
    clearInterval(timer);
  });
</script>

<div class="relative" use:dismissable={() => (open = false)}>
  <button
    type="button"
    class="relative -m-2.5 p-2.5 text-muted hover:text-ink-secondary"
    aria-label={label}
    aria-expanded={open}
    aria-controls="shell-notifications"
    onclick={toggle}
  >
    <HeroIcon name={count ? 'bellAlert' : 'bell'} class="size-6" />
    {#if count}<span
        class="absolute top-1.5 right-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-danger px-1 text-caption font-medium text-white tabular-nums"
        aria-hidden="true">{countLabel}</span
      >{/if}
  </button>
  {#if open}
    <div
      id="shell-notifications"
      class="absolute right-0 z-50 mt-2.5 w-80 origin-top-right overflow-hidden rounded-lg bg-surface shadow-lg outline-1 outline-ink/5 sm:w-96"
      role="region"
      aria-label="Avisos sin leer"
    >
      <div class="flex items-center justify-between border-b border-line px-4 py-3">
        <p class="text-label text-ink">Avisos sin leer</p>
        {#if count}<span
            class="rounded-full bg-brand-50 px-2 py-0.5 text-caption font-medium text-brand-800 tabular-nums"
            >{countLabel}</span
          >{/if}
      </div>
      {#if error}
        <p class="px-4 py-3 text-body text-danger" role="alert">{error}</p>
      {:else if loading && !count}
        <p class="px-4 py-3 text-body text-muted">Consultando avisos...</p>
      {:else if !count}
        <div class="px-4 py-6 text-center">
          <HeroIcon name="bell" class="mx-auto size-8 text-muted" />
          <p class="mt-2 text-body text-muted">No tienes avisos pendientes de leer.</p>
        </div>
      {:else}
        <ul role="list" class="max-h-96 divide-y divide-line overflow-y-auto">
          {#each unread.slice(0, SHOWN) as row (row.id)}
            {@const overdue = row.kind.kind === 'overdue_unattended'}
            <li class="flex gap-x-3 px-4 py-3">
              <HeroIcon
                name={overdue ? 'warning' : 'clock'}
                class={`mt-0.5 size-5 shrink-0 ${overdue ? 'text-danger' : 'text-muted'}`}
              />
              <div class="min-w-0">
                <p class="text-caption font-medium text-muted">{alertKindLabel(row)}</p>
                <p class="truncate text-label text-ink">{row.subject_title}</p>
                <p class="truncate text-caption text-muted">
                  {row.case_title} / <span class="font-mono">{row.case_reference}</span>
                </p>
                <p class="mt-0.5 text-caption text-muted">{alertTimeLabel(row.created_at)}</p>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
      <button
        type="button"
        class="flex w-full items-center justify-between border-t border-line bg-canvas px-4 py-2.5 text-left text-label text-brand-800 hover:bg-brand-50"
        onclick={go}
        >{count > SHOWN ? `Ver los ${countLabel} avisos` : 'Abrir la bandeja de alertas'}<HeroIcon
          name="arrow"
          class="size-4"
        /></button
      >
    </div>
  {/if}
</div>
