<script>
  export let record;
</script>

<section class="case-comparison" aria-label="Creaci&#243;n conjunta confirmada">
  <h3>Resultado y plazo guardados juntos</h3>
  <p>{record.result.values.summary} / Resultado revisi&#243;n {record.result.revision}</p>
  <p>{record.deadline.definition.title} / Plazo revisi&#243;n {record.deadline.revision}</p>
  <p>Captura original verificada. Las revisiones posteriores se consultan por separado.</p>
  <details>
    <summary>Origen de la creaci&#243;n conjunta</summary>
    <p>Resultado: <code>{record.origin.result_id}</code></p>
    <p>Plazo: <code>{record.origin.deadline_id}</code></p>
    <p>Autor original: {record.origin.recorded_by.email}</p>
    <p>Evento: {record.origin.source_event.sequence}</p>
    <p>Revisi&#243;n aprobada: <code>{record.review_digest}</code></p>
    <p>Captura conservada: <code>{record.capture_digest}</code></p>
  </details>
</section>
