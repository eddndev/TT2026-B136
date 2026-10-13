<script>
  export let metadata;
  export let compact = false;
  $: tags = metadata?.tags || [];
  $: populated = metadata?.document_type || metadata?.classification || tags.length;
</script>

{#if compact}
  {#if populated}<span class="metadata-compact">
      {#if metadata.document_type}<span class="badge">{metadata.document_type}</span>{/if}
      {#if metadata.classification}<span class="badge">{metadata.classification}</span>{/if}
      {#each tags.slice(0, 2) as tag}<span class="badge">{tag}</span>{/each}
      {#if tags.length > 2}<small>+{tags.length - 2} etiquetas</small>{/if}
    </span>{/if}
{:else}
  {#if !populated}<p class="hint">Sin valores de clasificaci&#243;n</p>{/if}
  <dl class="metadata-values">
    <div>
      <dt>Tipo de documento</dt>
      <dd>{metadata?.document_type || 'Sin tipo'}</dd>
    </div>
    <div>
      <dt>Clasificaci&#243;n</dt>
      <dd>{metadata?.classification || 'Sin clasificaci\u00f3n'}</dd>
    </div>
  </dl>
  {#if tags.length}<ul class="metadata-tags" aria-label="Etiquetas">
      {#each tags as tag}<li class="badge">{tag}</li>{/each}
    </ul>
  {:else}<p class="hint">Sin etiquetas</p>{/if}
{/if}
