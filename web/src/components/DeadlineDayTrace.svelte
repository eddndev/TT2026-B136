<script>
  export let count;
  let visible = 25;
  const classifications = {
    countable: 'Computable',
    excluded: 'Excluido',
    unresolved: 'Sin resolver',
  };
  const outcomes = {
    candidate: 'Fecha candidata',
    unresolved: 'Fecha sin resolver',
    outside_coverage: 'Fuera de cobertura',
    date_range_exhausted: 'Intervalo agotado despu\u00e9s de',
  };
</script>

<p>
  Primer d&#237;a incluido: {count.first_included}. Requeridos: {count.quantity}. Acumulados: {count.accumulated}.
</p>
<p>{outcomes[count.outcome.kind]}: {count.outcome.date ?? count.outcome.after}</p>
<div class="deadline-days" role="region" aria-label="Detalle de d&#237;as computados" tabindex="0">
  <table>
    <thead
      ><tr
        ><th>Fecha</th><th>Clasificaci&#243;n</th><th>Acumulado</th><th>Fundamento registrado</th
        ></tr
      ></thead
    >
    <tbody
      >{#each count.trace.slice(0, visible) as step}<tr>
          <td>{step.day.date}</td><td
            >{classifications[step.day.classification] ?? 'Fuera de cobertura'}</td
          ><td>{step.accumulated}</td>
          <td
            ><p>{step.day.explanation}</p>
            {#if step.day.origin}<p>
                {step.day.origin.kind === 'weekly_pattern'
                  ? 'Patr\u00f3n semanal'
                  : 'Excepci\u00f3n declarada'}
              </p>{/if}
            {#if step.day.source_ids.length}<details>
                <summary>Referencias de este d&#237;a</summary>
                <ul>
                  {#each step.day.source_ids as id}<li><code>{id}</code></li>{/each}
                </ul>
              </details>{/if}
          </td></tr
        >{/each}</tbody
    >
  </table>
</div>
{#if visible < count.trace.length}<button class="secondary" onclick={() => (visible += 25)}
    >Mostrar m&#225;s d&#237;as ({Math.min(visible, count.trace.length)} de {count.trace
      .length})</button
  >{/if}
