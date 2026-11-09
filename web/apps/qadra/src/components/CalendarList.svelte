<script>
  import Button from './Button.svelte';
  import FilterBar from './FilterBar.svelte';
  import FilterField from './FilterField.svelte';
  import { calendarEntities, calendarStatus } from '../lib/judicial-calendar-labels.mjs';
  export let rows,
    filters,
    onapply,
    onselect,
    busy = false,
    disabled = false,
    more = false,
    index = 0,
    onprevious,
    onnext,
    onrefresh,
    canManage = false,
    onnew;
  $: locked = busy || disabled;
</script>

<section class="card calendar-catalog" aria-label="Cat&#225;logo de calendarios" aria-busy={busy}>
  <FilterBar
    as="form"
    onsubmit={(event) => {
      event.preventDefault();
      onapply();
    }}
  >
    <FilterField
      >Estado del calendario<select bind:value={filters.status} disabled={locked}
        ><option value="published">Publicados</option><option value="retired">Retirados</option
        ><option value="all">Todos</option></select
      ></FilterField
    >
    <FilterField
      >Filtrar fuero<select bind:value={filters.jurisdiction} disabled={locked}
        ><option value="">Cualquier fuero</option><option value="federal">Federal</option><option
          value="local">Local</option
        ></select
      ></FilterField
    >
    <FilterField class="calendar-entity-filter"
      >Filtrar entidad<select bind:value={filters.entityCode} disabled={locked}
        ><option value="">Cualquier entidad</option>{#each calendarEntities as [code, name]}<option
            value={code}>{code} {name}</option
          >{/each}</select
      ></FilterField
    >
    <Button variant="secondary" disabled={locked}>Aplicar filtros de calendarios</Button>
    <div class="filter-actions calendar-actions">
      <Button variant="secondary" type="button" disabled={locked} onclick={onrefresh}
        >Actualizar calendarios</Button
      >{#if canManage}<Button variant="primary" type="button" disabled={locked} onclick={onnew}
          >Publicar calendario</Button
        >{/if}
    </div>
  </FilterBar>
  {#if busy}<p role="status">Consultando calendarios...</p>{/if}
  {#each rows as row (row.id)}<article class="calendar-list-row">
      <div>
        <h3>{row.scope.title}</h3>
        <p>{row.scope.authority} / {row.scope.organ}</p>
        <p>
          {row.scope.jurisdiction === 'federal' ? 'Federal' : 'Local'} / Cobertura {row.coverage
            .from} a {row.coverage.through}
        </p>
        <p class="hint">
          {calendarStatus[row.status]} / Revisi&#243;n {row.revision}{row.has_unresolved
            ? ' / Incluye reglas sin resolver'
            : ''}
        </p>
        <details><summary>Identidad del calendario</summary><code>{row.id}</code></details>
      </div>
      <Button
        variant="secondary"
        disabled={locked}
        onclick={() => onselect(row.id)}
        aria-label={`Consultar calendario ${row.id}`}>Consultar calendario</Button
      >
    </article>{/each}
  {#if !busy && !rows.length}<p>No hay calendarios en esta consulta.</p>{/if}
  <div class="pagination">
    <span class="hint">{rows.length} calendarios en esta p&#225;gina</span>
    <div class="action-row">
      <Button variant="secondary" disabled={locked || !index} onclick={onprevious}
        >P&#225;gina anterior de calendarios</Button
      ><Button variant="secondary" disabled={locked || !more} onclick={onnext}
        >P&#225;gina siguiente de calendarios</Button
      >
    </div>
  </div>
  <p class="filter-note">
    Estas reglas declaradas no calculan vencimientos ni determinan notificaciones. Su aplicabilidad
    requiere el &#225;mbito y las fuentes expresos.
  </p>
</section>
