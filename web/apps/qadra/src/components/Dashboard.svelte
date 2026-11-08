<script>
  // Overview of Owner and Litigator: the heading with the authorized counts,
  // the coming agenda, the load per litigator and the shortcuts. Counts and
  // agenda are read again for each account; a response that arrives after the
  // account or role changed, or after the view closed, is discarded.
  import { onMount, onDestroy } from 'svelte';
  import OverviewHeading from './OverviewHeading.svelte';
  import DashboardStats from './DashboardStats.svelte';
  import DashboardAgenda from './DashboardAgenda.svelte';
  import DashboardWorkload from './DashboardWorkload.svelte';
  import OverviewLinks from './OverviewLinks.svelte';
  export let api, user, onnavigate, ondocument;
  let mounted = false,
    alive = true,
    seen = null,
    generation = 0;
  let scoped = null,
    value = null,
    busy = true,
    error = '',
    agenda = null;
  $: allowed = ['owner', 'litigator'].includes(user?.role);
  $: identity = `${user?.id}:${user?.role}`;
  $: if (mounted && identity !== seen) open(identity);
  $: scope = user?.role === 'owner' ? 'Todo el despacho' : 'Tus expedientes asignados';
  function open(current) {
    seen = current;
    generation++;
    scoped?.dispose();
    scoped = allowed ? api.dashboard() : null;
    value = null;
    error = '';
    busy = false;
    if (scoped) load();
  }
  async function load() {
    if (!alive || !scoped || busy) return;
    const request = ++generation,
      context = identity;
    busy = true;
    value = null;
    error = '';
    try {
      const response = await scoped.get();
      if (!alive || request !== generation || context !== identity) return;
      if (response.scope !== (user.role === 'owner' ? 'office' : 'assigned_cases'))
        throw new Error('El alcance de los indicadores no coincide con tu acceso.');
      value = response;
    } catch (failure) {
      if (alive && request === generation && context === identity) error = failure.message;
    } finally {
      if (alive && request === generation && context === identity) busy = false;
    }
  }
  function refresh() {
    load();
    agenda?.refresh();
  }
  onMount(() => {
    mounted = true;
  });
  onDestroy(() => {
    alive = false;
    generation++;
    value = null;
    scoped?.dispose();
  });
</script>

{#if allowed}
  <section class="grid gap-8" aria-label="Indicadores operativos" aria-busy={busy}>
    <OverviewHeading {user} {scope} {onnavigate}>
      <DashboardStats {value} {busy} {error} onrefresh={refresh} />
    </OverviewHeading>
    <div class="grid gap-8 xl:grid-cols-3">
      {#key identity}
        <DashboardAgenda bind:this={agenda} class="xl:col-span-2" {api} {onnavigate} />
      {/key}
      <div class="grid grid-cols-1 content-start gap-8">
        {#if value}<DashboardWorkload {value} />{/if}
        <OverviewLinks {user} {onnavigate} {ondocument} />
      </div>
    </div>
  </section>
{/if}
