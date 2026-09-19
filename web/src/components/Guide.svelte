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
      suspende cambios de ficha, documentos, participantes, etapas y audiencias. No cambia la etapa
      registrada ni reactiva participantes archivados.
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
      Agregar participante permite elegir o registrar una identidad, tipificar el rol y revisar
      coincidencias con soporte documental exacto. Registrar ficha pendiente conserva el alta
      manual; Completar perfil agrega una revisi&oacute;n sin borrar su historia. Los
      hom&oacute;nimos se revisan expresamente y nunca se fusionan por el nombre.
    </p>
    <p>
      Defensor y juez de control requieren certificado p&uacute;blico y firma personal. Descarga la
      declaraci&oacute;n preparada, f&iacute;rmala con tu herramienta y carga la firma separada. Tu
      clave privada permanece fuera de Qadra. La comprobaci&oacute;n usa la CA interna de
      demostraci&oacute;n y no acredita FIREL oficial. Cambiar valores o certificado exige revisar y
      preparar de nuevo; un resultado incierto se consulta antes de reenviarlo.
    </p>
    <p>
      La ficha conserva su identidad vinculada. Consultar o editar la identidad actual no modifica
      las fichas ni firmas anteriores. Contacto y protecci&oacute;n se describen mediante soportes:
      Qadra no env&iacute;a mensajes ni crea un canal de contacto.
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
    <summary>&iquest;C&oacute;mo programo y consulto audiencias?</summary>
    <p>
      Abre Audiencias dentro de un expediente con ficha penal y etapa registradas. El administrador
      y el litigante asignado pueden programar, corregir, reprogramar y cancelar; el asistente legal
      asignado puede consultar. Declara fecha, hora, desfase UTC, modalidad y sede o
      conexi&oacute;n. Cada cambio conserva el motivo y la historia anterior.
    </p>
    <p>
      Elige las revisiones exactas de las fichas previstas. Sus cambios posteriores no actualizan la
      audiencia. Individualizaci&oacute;n requiere un antecedente de condena declarado y soporte PDF
      o DOCX exacto; la etapa Juicio no demuestra por s&iacute; sola una condena. Programar no
      acredita asistencia, celebraci&oacute;n ni resultados.
    </p>
    <p>
      Agenda consulta las citas de todos tus expedientes autorizados dentro de un rango y desfase
      expl&iacute;citos. Abre una revisi&oacute;n exacta; consulta el registro actual antes de
      modificar. Si se pierde una respuesta, conserva el formulario y consulta el resultado del
      env&iacute;o sin repetir la escritura. Un resultado todav&iacute;a no disponible sigue
      incierto.
    </p>
    <p>
      La programaci&oacute;n y los resultados declarados conservan historias separadas. Registrar
      una audiencia no crea autom&aacute;ticamente un plazo ni env&iacute;a avisos. Las asignaciones
      de cuentas se gestionan mediante la API.
    </p>
  </details>
  <details>
    <summary>&iquest;C&oacute;mo registro y consulto un plazo?</summary>
    <p>
      Abre Plazos dentro del expediente. El administrador y el litigante asignado pueden registrar,
      corregir, declarar atenci&oacute;n y retirar; el asistente legal asignado puede consultar.
      Selecciona perfil, fuente, calendario y responsable. Para cada dependencia, declara si sigue
      cambios o conserva la revisi&oacute;n seleccionada. Declara aplicabilidad, condiciones y
      cantidad concedida cuando corresponda; no uses el m&aacute;ximo como una cantidad otorgada.
    </p>
    <p>
      Prepara el plazo y revisa el resultado, los pasos del c&oacute;mputo y sus bloqueos antes de
      confirmar. Un registro bloqueado conserva lo que falta sin inventar un vencimiento. La
      historia permite consultar el perfil y las fuentes exactas de cada revisi&oacute;n. Declarar
      atenci&oacute;n o retirar conserva el c&aacute;lculo anterior y no acredita por s&iacute; solo
      una presentaci&oacute;n v&aacute;lida.
    </p>
    <p>
      Un conflicto conserva tu borrador: consulta y compara la base actual antes de preparar otra
      correcci&oacute;n. Si se pierde la respuesta al confirmar, consulta el env&iacute;o exacto sin
      repetirlo. Un expediente cerrado conserva lecturas autorizadas.
    </p>
    <p>
      El seguimiento procesa cambios de las dependencias. Si sigues un calendario, una nueva
      revisi&oacute;n puede actualizar el c&aacute;lculo. Si sigues una fuente o perfil que cambia,
      revisa los insumos y confirma expl&iacute;citamente una correcci&oacute;n. Mientras se
      requiere revisi&oacute;n, el c&aacute;lculo anterior permanece en la historia y no se presenta
      como vencimiento vigente. Consulta la causa y el autor de cada revisi&oacute;n. La agenda
      conjunta y los avisos siguen pendientes.
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
