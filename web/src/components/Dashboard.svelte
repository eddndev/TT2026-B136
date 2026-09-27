<script>
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import '../styles/dashboard.css';
  export let api, user, onnavigate;
  let mounted = false,
    alive = true,
    seen = null,
    generation = 0;
  let scoped = null,
    value = null,
    busy = true,
    error = '';
  $: allowed = ['owner', 'litigator'].includes(user?.role);
  $: identity = `${user?.id}:${user?.role}`;
  $: if (mounted && identity !== seen) open(identity);
  const metrics = [
    {
      key: 'active_cases',
      label: 'Expedientes activos',
      icon: 'briefcase',
      detail: 'Estado administrativo activo',
    },
    {
      key: 'pending_contracts',
      label: 'Contratos pendientes de sello',
      icon: 'file',
      detail: 'Versiones actuales sin sello interno',
    },
    {
      key: 'deadlines_overdue',
      label: 'Plazos vencidos',
      icon: 'clock',
      detail: 'Sin atenci\u00f3n declarada',
      urgent: true,
    },
    {
      key: 'deadlines_due_48h',
      label: 'Vencen en menos de 48 horas',
      icon: 'clock',
      detail: 'Atenci\u00f3n prioritaria',
      urgent: true,
    },
    {
      key: 'deadlines_due_7d',
      label: 'Vencen en menos de 7 d\u00edas',
      icon: 'calendar',
      detail: 'Incluye los de 48 horas',
    },
    {
      key: 'deadlines_unresolved',
      label: 'Plazos por revisar',
      icon: 'info',
      detail: 'Sin vencimiento operativo vigente',
    },
  ];
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
  const timestamp = (instant) =>
    new Intl.DateTimeFormat('es-MX', {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(instant));
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
  <section class="dashboard-panel" aria-label="Indicadores operativos" aria-busy={busy}>
    <div class="section-heading dashboard-heading">
      <div>
        <h2>Indicadores operativos</h2>
        <p>{user.role === 'owner' ? 'Todo el despacho' : 'Tus expedientes asignados'}</p>
      </div>
      <button class="secondary" disabled={busy} onclick={load}>Actualizar indicadores</button>
    </div>
    {#if busy}
      <div class="card dashboard-status" role="status">Consultando los indicadores actuales...</div>
    {:else if error}
      <p class="notice error" role="alert">{error}</p>
    {:else if value}
      <p class="dashboard-cutoff">
        Consulta del <time datetime={value.checked_at}>{timestamp(value.checked_at)}</time>.
        Actualiza para incorporar cambios posteriores.
      </p>
      <div class="stats-grid dashboard-metrics">
        {#each metrics as item}
          <article
            class="stat-card dashboard-metric"
            class:dashboard-urgent={item.urgent && value[item.key] > 0}
            data-metric={item.key}
            aria-label={item.label}
          >
            <div>
              <span>{item.label}</span><strong>{value[item.key]}</strong><small>{item.detail}</small
              >
            </div>
            <span class="stat-icon"><Icon name={item.icon} size={22} /></span>
          </article>
        {/each}
      </div>
      <p class="hint dashboard-explanation">
        Los plazos incluyen expedientes cerrados administrativamente. Se excluyen los retirados y
        atendidos; los que requieren revisi&oacute;n no se presentan como fechas vigentes.
      </p>
      <section class="card dashboard-workload" aria-label="Carga por litigante">
        <div class="section-heading">
          <div>
            <h3>Carga por litigante</h3>
            <p>Expedientes activos dentro de este alcance.</p>
          </div>
          <span class="tile-icon"><Icon name="users" size={22} /></span>
        </div>
        {#if value.active_cases === 0}<p class="dashboard-empty">
            Sin expedientes activos en este alcance.
          </p>{/if}
        {#if value.workload.length}
          <ul>
            {#each value.workload as row (row.user_id)}
              <li>
                <span>{row.email}</span><strong
                  >{row.active_cases}<small
                    >{row.active_cases === 1 ? 'expediente' : 'expedientes'}</small
                  ></strong
                >
              </li>
            {/each}
          </ul>
          <p class="hint">Un expediente compartido cuenta para cada litigante asignado.</p>
        {:else}<p class="hint">No hay carga asignada a litigantes activos.</p>{/if}
      </section>
      <div class="action-row dashboard-actions">
        <button class="text-button" onclick={() => onnavigate('cases')}
          >Ver expedientes<Icon name="arrow" size={15} /></button
        >
        <button class="text-button" onclick={() => onnavigate('agenda')}
          >Consultar agenda<Icon name="calendar" size={15} /></button
        >
      </div>
    {/if}
  </section>
{/if}
