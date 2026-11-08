<script>
  import { onDestroy, onMount } from 'svelte';
  import HeroIcon from './HeroIcon.svelte';
  import { dismissable } from '../lib/dismissable.mjs';
  import { matchCases } from '../lib/shell-search.mjs';
  export let api;
  export let onopencase;
  export let onallcases;
  const PAGE = 100,
    PAGES = 3,
    FRESH_MS = 60000;
  let input,
    query = '',
    open = false,
    active = -1,
    rows = null,
    loadedAt = 0,
    loading = false,
    truncated = false,
    error = '',
    generation = 0,
    alive = true;
  $: results = rows ? matchCases(rows, query) : [];
  $: if (active >= results.length) active = results.length - 1;
  // One listing per search session: the whole accessible case list is matched
  // locally, so typing never sends a query per keystroke.
  async function ensureLoaded() {
    if (loading || (rows && Date.now() - loadedAt < FRESH_MS)) return;
    const current = ++generation;
    loading = true;
    error = '';
    try {
      const collected = [];
      let afterId,
        more = true;
      for (let page = 0; page < PAGES && more; page++) {
        const result = await api.caseAdministrations({ status: 'all', limit: PAGE, afterId });
        collected.push(...result.cases);
        more = result.has_more;
        afterId = result.next_after_id;
      }
      if (!alive || current !== generation) return;
      rows = collected;
      truncated = more;
      loadedAt = Date.now();
    } catch (failure) {
      if (alive && current === generation) error = failure.message;
    } finally {
      if (alive && current === generation) loading = false;
    }
  }
  function show() {
    open = true;
    ensureLoaded();
  }
  function close() {
    open = false;
    active = -1;
  }
  function choose(record) {
    close();
    query = '';
    input.blur();
    onopencase(record);
  }
  function all() {
    close();
    input.blur();
    onallcases();
  }
  function keydown(event) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      show();
      if (!results.length) return;
      const step = event.key === 'ArrowDown' ? 1 : -1;
      active = (active + step + results.length) % results.length;
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (results.length) choose(results[Math.max(active, 0)]);
      else if (query.trim()) all();
    }
  }
  function shortcut(event) {
    const typing = event.target.closest?.('input, textarea, select, [contenteditable="true"]');
    if (((event.ctrlKey || event.metaKey) && event.key === 'k') || (event.key === '/' && !typing)) {
      event.preventDefault();
      input?.focus();
    }
  }
  onMount(() => {
    document.addEventListener('keydown', shortcut);
    return () => document.removeEventListener('keydown', shortcut);
  });
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<form
  class="relative grid flex-1 grid-cols-1"
  role="search"
  onsubmit={(event) => event.preventDefault()}
  use:dismissable={close}
>
  <input
    bind:this={input}
    bind:value={query}
    type="text"
    role="combobox"
    aria-label="Buscar expedientes"
    aria-autocomplete="list"
    aria-expanded={open && query.trim().length > 1}
    aria-controls="shell-search-results"
    aria-activedescendant={active >= 0 ? `shell-search-${results[active]?.id}` : undefined}
    placeholder="Buscar expedientes"
    title="Busca por t&#237;tulo, referencia, NUC o carpeta judicial"
    autocomplete="off"
    spellcheck="false"
    class="col-start-1 row-start-1 block size-full rounded-none border-0 bg-surface py-0 pr-14 pl-8 text-base text-ink outline-hidden placeholder:text-muted focus-visible:outline-hidden sm:text-body sm:leading-6"
    onfocus={show}
    oninput={() => {
      show();
      active = -1;
    }}
    onkeydown={keydown}
  />
  <HeroIcon
    name="search"
    class="pointer-events-none col-start-1 row-start-1 size-5 self-center text-muted"
  />
  <kbd
    class="pointer-events-none col-start-1 row-start-1 hidden self-center justify-self-end rounded border border-line px-1.5 font-sans text-caption text-muted sm:block"
    >Ctrl K</kbd
  >
  {#if open && query.trim().length > 1}
    <div
      class="absolute inset-x-0 top-full z-50 mt-1 max-w-2xl overflow-hidden rounded-lg bg-surface shadow-lg outline-1 outline-ink/5"
    >
      {#if loading && !rows}
        <p class="px-4 py-3 text-body text-muted">Consultando expedientes...</p>
      {:else if error}
        <p class="px-4 py-3 text-body text-danger" role="alert">{error}</p>
      {:else}
        <ul id="shell-search-results" role="listbox" aria-label="Expedientes encontrados">
          {#each results as record, index (record.id)}
            <li
              id={`shell-search-${record.id}`}
              role="option"
              aria-selected={index === active}
              class={`flex cursor-pointer items-center gap-x-3 px-4 py-2.5 ${index === active ? 'bg-brand-50' : 'hover:bg-canvas'}`}
              onpointerdown={(event) => event.preventDefault()}
              onclick={() => choose(record)}
              onmouseenter={() => (active = index)}
            >
              <HeroIcon name="briefcase" class="size-5 text-muted" />
              <span class="min-w-0 flex-1">
                <span class="block truncate text-label text-ink">{record.title}</span>
                <span class="block truncate text-caption text-muted"
                  ><span class="font-mono">{record.reference}</span>{#if record.penal_identifiers}
                    &#183; NUC {record.penal_identifiers.nuc}{/if}</span
                >
              </span>
              {#if record.administrative_status === 'closed'}<span
                  class="rounded-full bg-canvas px-2 py-0.5 text-caption text-ink-secondary ring-1 ring-line ring-inset"
                  >Cerrado</span
                >{/if}
            </li>
          {:else}
            <li class="px-4 py-3 text-body text-muted" role="presentation">
              Ning&#250;n expediente coincide con &#171;{query.trim()}&#187;.
            </li>
          {/each}
        </ul>
        <button
          type="button"
          class="flex w-full items-center justify-between border-t border-line bg-canvas px-4 py-2.5 text-left text-label text-brand-800 hover:bg-brand-50"
          onpointerdown={(event) => event.preventDefault()}
          onclick={all}>Ir al archivo de expedientes<HeroIcon name="arrow" class="size-4" /></button
        >
        {#if truncated}<p class="border-t border-line px-4 py-2 text-caption text-muted">
            La b&#250;squeda r&#225;pida cubre los primeros {PAGE * PAGES} expedientes; usa los filtros
            del archivo para el resto.
          </p>{/if}
      {/if}
    </div>
  {/if}
</form>
