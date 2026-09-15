<script>
  import Icon from './Icon.svelte';
  export let onnavigate;
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">AYUDA Y ORIENTACI&Oacute;N</span>
    <h1>Un recorrido claro, de principio a fin</h1>
    <p>Lo que necesitas para trabajar con tus documentos.</p>
  </div>
</div>
<section class="guide-grid">
  {#each [{ icon: 'upload', title: '1. Carga tu documento', text: 'Abre primero el expediente. Selecciona o arrastra un archivo de hasta 16 MiB. Te sugerimos un nombre compatible; puedes revisarlo antes de cargarlo.' }, { icon: 'lock', title: '2. Genera su evidencia', text: 'Un administrador o litigante puede firmar y sellar. La confirmaci\u00f3n registra esta acci\u00f3n con la identidad de quien la realiza.' }, { icon: 'shield', title: '3. Comprueba el resultado', text: 'Revisa la integridad, la firma, el certificado y el sello. Si hay alg\u00fan fallo, el resultado te indicar\u00e1 qu\u00e9 componente necesita revisi\u00f3n.' }, { icon: 'download', title: '4. Conserva el respaldo', text: 'Descarga el paquete ZIP para conservar la evidencia. El documento queda disponible en el expediente para futuras consultas.' }] as item}<article
      class="card"
    >
      <span class="tile-icon"><Icon name={item.icon} size={24} /></span>
      <h2>{item.title}</h2>
      <p>{item.text}</p>
    </article>{/each}
</section>
<section class="card guide-questions">
  <h2>Antes de continuar</h2>
  <details>
    <summary>&iquest;C&oacute;mo registro o completo un expediente penal?</summary>
    <p>
      En Expedientes, elige Nuevo expediente penal. Registra t&iacute;tulo, referencia interna, NUC,
      carpeta judicial, sus autoridades y de una a ocho descripciones manuales de delito. El Resumen
      muestra la ficha y el registro inicial de Investigaci&oacute;n, sin acreditar un acto
      judicial.
    </p>
    <p>
      Una ficha pendiente permite editar los datos b&aacute;sicos o completar todos los datos
      penales. Completar un expediente anterior no inventa una etapa ni fechas pasadas. El historial
      administrativo conserva las revisiones y sus autores.
    </p>
  </details>
  <details>
    <summary>&iquest;Qu&eacute; ocurre al cerrar administrativamente?</summary>
    <p>
      El administrador o el litigante asignado pueden cerrar y reactivar el expediente desde
      Resumen. Cerrado conserva consulta, historial, verificaci&oacute;n y descarga de evidencia;
      suspende cambios de ficha, documentos, participantes y etapas. No cambia la etapa registrada
      ni reactiva participantes archivados.
    </p>
    <p>
      Si el expediente se cerr&oacute; mientras guardabas, se consulta su estado y se conserva el
      formulario. Reactivarlo no reenv&iacute;a autom&aacute;ticamente tus cambios.
    </p>
  </details>
  <details>
    <summary>&iquest;C&oacute;mo registro una etapa procesal?</summary>
    <p>
      Abre Etapas dentro del expediente. Una ficha completa sin etapa permite registrar la etapa
      conocida, su fecha, motivo y soporte, sin reconstruir transiciones anteriores. Desde
      Investigaci&oacute;n puedes registrar el paso a Intermedia; desde Intermedia, el paso a
      Juicio. El Resumen conserva el registro inicial hist&oacute;rico.
    </p>
    <p>
      Declara fechas y desfases UTC; si desconoces la hora, elige fecha sin hora. Selecciona y
      confirma una versi&oacute;n exacta del soporte PDF o DOCX. Para Juicio, emisi&oacute;n del
      auto y recepci&oacute;n por el tribunal son datos separados. El registro conserva lo
      declarado; no acredita autom&aacute;ticamente un acto judicial.
    </p>
    <p>
      Si hay conflicto, el borrador permanece disponible para comparar con etapa e historial. Una
      carga confirmada conserva su archivo aunque se rechace registrar la etapa. Si no se conoce el
      resultado del env&iacute;o, consulta antes de intentar de nuevo.
    </p>
  </details>
  <details>
    <summary>&iquest;C&oacute;mo organizo los participantes?</summary>
    <p>
      Dentro del expediente, abre Participantes. El administrador y el litigante asignado pueden
      registrar y editar personas; el asistente legal asignado puede consultarlas. El rol cliente no
      accede al directorio. Registrar una persona no crea una cuenta ni concede acceso.
    </p>
    <p>
      El historial conserva los cambios. Archivar y reactivar solo organizan el directorio; no
      cambian la situaci&oacute;n jur&iacute;dica. Ante una edici&oacute;n concurrente, consulta los
      datos actuales y confirma tus cambios sin perder el formulario.
    </p>
  </details>
  <details>
    <summary>&iquest;D&oacute;nde est&aacute;n mis documentos anteriores?</summary>
    <p>
      Abre el expediente y consulta su archivo documental. Puedes buscar por nombre, filtrar por
      estado de sellado y recorrer las p&aacute;ginas. La b&uacute;squeda abarca los documentos
      autorizados del expediente, incluso despu&eacute;s de cerrar sesi&oacute;n.
    </p>
  </details>
  <details>
    <summary>&iquest;C&oacute;mo agrego o consulto otra versi&oacute;n?</summary>
    <p>
      Abre el documento y usa Agregar versi&oacute;n para cargar otro archivo sin reemplazar los
      anteriores. El historial muestra primero las versiones m&aacute;s nuevas. Selecciona una fila
      para consultar, sellar o descargar esa versi&oacute;n exacta. Si otra persona agrega una
      versi&oacute;n mientras preparas tu archivo, este permanece en el di&aacute;logo: consulta la
      versi&oacute;n actual, revisa los datos y confirma de nuevo el env&iacute;o.
    </p>
  </details>
  <details>
    <summary>&iquest;C&oacute;mo organizo y encuentro mis documentos?</summary>
    <p>
      Al cargar un archivo puedes indicar tipo, clasificaci&oacute;n y etiquetas. Cada etiqueta se
      agrega por separado y puede contener comas y acentos. Despu&eacute;s puedes editar estos
      valores en Clasificaci&oacute;n actual del documento y consultar su historial de cambios. Los
      archivos, firmas y sellos conservan sus propias versiones. Los filtros de clasificaci&oacute;n
      buscan valores completos y distinguen may&uacute;sculas y acentos.
    </p>
  </details>
  <details>
    <summary>&iquest;Qui&eacute;n puede hacer cada cosa?</summary>
    <p>
      El administrador tiene acceso a todos los expedientes. El litigante y el asistente legal
      necesitan una asignaci&oacute;n vigente. El administrador y el litigante pueden cargar,
      sellar, verificar y descargar. El asistente legal puede cargar, verificar y descargar; el
      sellado requiere un administrador o litigante. La auditor&iacute;a completa y el alta de
      usuarios corresponden al administrador.
    </p>
  </details>
  <details>
    <summary>&iquest;Qu&eacute; significa sellado local?</summary>
    <p>
      Es una demostraci&oacute;n t&eacute;cnica de firma y sello de tiempo. No equivale a una
      constancia NOM-151 emitida por un PSC autorizado.
    </p>
  </details>
  <details>
    <summary>&iquest;Puedo organizar expedientes y audiencias?</summary>
    <p>
      Puedes crear y consultar expedientes. Las asignaciones se gestionan mediante la API; su
      selector de usuarios todav&iacute;a no est&aacute; disponible en esta interfaz. Las audiencias
      y los plazos a&uacute;n no est&aacute;n disponibles. El directorio de participantes registra
      datos manuales, sin validar identidad legal ni etapas procesales.
    </p>
  </details>
  <details>
    <summary>&iquest;Qu&eacute; pasa si pierdo mi segundo factor?</summary>
    <p>
      Usa uno de los c&oacute;digos de recuperaci&oacute;n guardados durante el alta. Cada
      c&oacute;digo funciona una sola vez. Si no conservas ninguno, contacta al administrador.
    </p>
  </details>
  <button class="primary" onclick={() => onnavigate('documents')}
    >Ir a documentos<Icon name="arrow" size={16} /></button
  >
</section>
