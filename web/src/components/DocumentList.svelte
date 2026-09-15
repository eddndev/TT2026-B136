<script>
  import Icon from './Icon.svelte';
  import { documentStatus } from '../lib/workspace.mjs';
  export let documents;
  export let selectedId;
  export let layout = 'list';
  export let onselect;
</script>

{#if layout === 'grid'}<div class="document-grid">
    {#each documents as item}{@const status = documentStatus(item)}<button
        class="document-card"
        class:selected={selectedId === item.id}
        onclick={() => onselect(item)}
        ><div class="section-heading">
          <span class="file-icon"><Icon name="file" size={25} /></span><span
            class="badge {status.tone}">{status.label}</span
          >
        </div>
        <strong>{item.name}</strong><small
          >{item.version ? `Versi\u00f3n ${item.version}` : 'Referencia por identificador'}</small
        >
        <div class="document-card-footer">
          <code>{item.id.slice(0, 8)}</code><span>Abrir<Icon name="arrow" size={15} /></span>
        </div></button
      >{/each}
  </div>
{:else}<div class="table-scroll">
    <table class="document-table">
      <thead
        ><tr
          ><th>Documento</th><th>Estado</th><th>Versi&#243;n</th><th
            ><span class="sr-only">Acciones</span></th
          ></tr
        ></thead
      ><tbody
        >{#each documents as item}{@const status = documentStatus(item)}<tr
            class:selected={selectedId === item.id}
            ><td
              ><div class="table-document">
                <span class="file-icon"><Icon name="file" /></span><span
                  ><strong>{item.name}</strong><small>{item.id.slice(0, 8)}</small></span
                >
              </div></td
            ><td><span class="badge {status.tone}">{status.label}</span></td><td
              class="version-cell">{item.version ? `v${item.version}` : '--'}</td
            ><td
              ><button
                class="text-button"
                aria-label={`Abrir ${item.name}`}
                onclick={() => onselect(item)}>Abrir<Icon name="arrow" size={15} /></button
              ></td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>{/if}
