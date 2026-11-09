<script>
  // Active cases assigned to each active litigator within the scope of the
  // counts. A row shows the account display name, or its email when the row
  // has none, and a bar proportional to the largest load.
  import { accountInitials } from '../lib/account-name.mjs';
  export let value;
  $: largest = Math.max(1, ...value.workload.map((row) => row.active_cases));
</script>

<section class="surface-card p-6" aria-labelledby="dashboard-workload-title">
  <h2 id="dashboard-workload-title" class="text-card-title text-ink">Carga por litigante</h2>
  <p class="mt-1 text-body text-muted">Expedientes activos dentro de este alcance</p>
  {#if value.active_cases === 0}
    <p class="mt-4 text-body text-muted">Sin expedientes activos en este alcance</p>
  {/if}
  {#if value.workload.length}
    <ul role="list" class="mt-2 divide-y divide-line">
      {#each value.workload as row (row.user_id)}
        {@const name = row.name ?? row.email}
        <li class="flex items-center gap-x-3 py-3">
          <span
            class="flex size-8 shrink-0 items-center justify-center rounded-full bg-brand-50 text-caption font-medium text-brand-800"
            aria-hidden="true">{accountInitials(row)}</span
          >
          <div class="min-w-0 flex-auto">
            <div class="flex flex-wrap items-baseline justify-between gap-x-3">
              <p class="max-w-full truncate text-label text-ink" title={row.email}>{name}</p>
              <p class="shrink-0 text-label text-ink tabular-nums">
                {row.active_cases}
                <span class="font-normal text-muted"
                  >{row.active_cases === 1 ? 'expediente' : 'expedientes'}</span
                >
              </p>
            </div>
            <div class="mt-2 h-1 rounded-full bg-line" aria-hidden="true">
              <div
                class="h-1 rounded-full bg-muted"
                style:width={`${(row.active_cases / largest) * 100}%`}
              ></div>
            </div>
          </div>
        </li>
      {/each}
    </ul>
    <p class="mt-2 text-caption text-muted">
      Un expediente compartido cuenta para cada litigante asignado
    </p>
  {:else}
    <p class="mt-4 text-body text-muted">No hay carga asignada a litigantes activos</p>
  {/if}
</section>
