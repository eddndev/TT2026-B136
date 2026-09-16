# ADR-0031: Hechos declarados de resolucion y notificacion

## Status

Accepted. Pure domain values and canonical encodings are implemented in
[the fact model](../procedural-facts.md). The [application contract](../procedural-facts-application.md)
now includes commands, exact-source validation, coordinating service and reads.
[Source and operation encodings PFSRC1/PFTXN1](../procedural-facts-receipts.md)
are implemented. The [PostgreSQL backend](../procedural-facts-persistence.md)
now includes migrations, audited transactions, effective case membership checks,
exact historical reads and startup validation. Backend verification is approved locally.
The [HTTP API](../procedural-facts-api.md) and composition are implemented, with
focused DTO, projection and route tests passing. The workspace suite and real
HTTP workflow with exact restoration passed locally. The [Qadra client](../../web/README.md#resoluciones-y-notificaciones-declaradas)
is implemented, with integrated browser verification approved locally. These declarations do
not establish operational legal deadlines.

## Context

Los resultados de audiencia conservan relatos, comparecencias y acuerdos con
historia exacta. Su registro no determina que exista una resolucion especifica,
que una persona haya sido notificada o que empiece un plazo. El
[contrato de resultados](../hearing-results-api.md) mantiene esa separacion.

El alcance de [recursos](../procedural-resources-scope.md) necesita resoluciones,
actos y soportes relacionados, sin convertirlos en una etapa lineal adicional.
El [conteo civil](../deadline-day-counting.md) recibe una primera fecha y una
cantidad; no interpreta hechos, selecciona una regla o produce por si solo un
vencimiento operativo. Es necesario conservar insumos estructurados sin atribuir
al texto libre efectos que el sistema no ha establecido.

Una resolucion puede relacionarse con varias practicas de notificacion. Sus
fechas, destinatarios y receptores pueden diferir. Combinar todo en una captura
unica dificulta corregir una practica sin reemplazar la historia de las demas.
Inmovilizar toda referencia personal tambien impediria rectificar una seleccion
erronea; consultar siempre su cabeza actual alteraria silenciosamente el pasado.

## Decision

### Familias, identidad y revisiones

Introducir dos familias de hechos declarados: `resolution` y `notification`.
Cada raiz tiene UUID, expediente y familia fijos. La raiz de notificacion fija
ademas el UUID de una resolucion del mismo expediente, existente previamente.
La relacion es uno a muchos; una resolucion puede existir sin notificaciones.
Esta separacion logica no obliga a duplicar infraestructura o pares de tablas.

Cada revision de notificacion conserva la revision exacta de la resolucion que
se selecciono, sus digests y las referencias personales declaradas. Estas
referencias pueden cambiar mediante correccion explicita y motivada dentro de
la misma raiz. Cambiar a otra raiz de resolucion exige una nueva notificacion.
La lectura de una revision nunca sustituye fuentes por sus cabezas actuales.

La raiz identifica una declaracion registrada, no acredita identidad juridica
universal del acto. Una nueva practica de notificacion crea otra raiz; corregir
un error de captura crea una revision. No deduplicar practicas por nombre,
fecha, texto o soporte, ni elegir automaticamente la notificacion aplicable.
El retiro es administrativo: conserva contenido e historia y no declara nulidad,
ineficacia o inexistencia del acto, ni extincion de un plazo.

### Datos declarados y procedencia

Separar clase de resolucion, organo emisor y tiempo declarado de la modalidad,
subtipo y tiempo de notificacion. No inferirlos desde nombre de archivo, etapa,
tipo de audiencia, resumen, perfil del directorio o clasificacion documental.
Tampoco inferir tipo de recurso, efectos, aplicabilidad o fecha de inicio.

Destinatario pretendido, receptor material y representante son funciones
separadas. Una relacion de representacion requiere declaracion expresa de sus
extremos, alcance y procedencia; no se obtiene del rol de una cuenta o de una
comparecencia. Reutilizar fichas y sujetos historicos exactos del mismo expediente
con sus digests, conforme a [identidades y perfiles](../typed-participants-api.md).
Una revision posterior del directorio no reescribe esa seleccion historica.

Los soportes identifican documento, version, digest y localizador, con una funcion
explicita en la declaracion. El servidor resuelve pertenencia, integridad y
proyecciones; un digest aportado por el cliente solo expresa una expectativa.
La admision tecnica no acredita autenticidad juridica ni efectos del documento.
Una version puede respaldar varias afirmaciones sin duplicar su procesamiento.

La procedencia desde una audiencia conserva resultado y revision exactos; si
senala un acuerdo, incluye su UUID y verifica que pertenece a esa revision.
Un UUID de acuerdo aislado no identifica el antecedente. No copiar el relato
HRES1 ni modificar sus canones. Una resolucion externa a audiencia no necesita
una programacion ficticia. Las referencias historicas preservan sus estados;
que una fuente pueda leerse no demuestra elegibilidad para un calculo futuro.

### Fuentes verificables y administracion

Separar material interno exacto de referencias para recibos y vistas legibles.
Un digest aislado no permite recomputar un canon ni verificar pertenencia de un
acuerdo. Resolver valores de ficha y sujeto, resultados y resolucion padre; este
ultimo solo incorpora su resultado y soporte historicos, sin un grafo recursivo.
Derivar vistas desde valores comprobados, sin exponer por defecto el sujeto
completo. La seleccion y las cotas quedan en el [contrato de aplicacion](../procedural-facts-application.md).
PFSRC1 vincula referencias, estados y vistas; PFTXN1 vincula actor, comando,
identidad fija y digests. Una correccion puede reseleccionar fuentes, pero no
reescribir las proyecciones de referencias exactas que conserva. Dos acuerdos
del mismo resultado comparten sus datos de revision.

Una declaracion externa no requiere inventar una audiencia, etapa o perfil penal.
Conservar una base administrativa sin revision como `Unrevised`; el contenedor
la liga al expediente y el adaptador comprueba sus metadatos originales. La
administracion actual cerrada bloquea cambios. Las observaciones de preparacion
no son revisiones esperadas administrativas ni sustituyen la captura bajo bloqueo.

### Precision temporal

Adoptar el valor separado de [precision temporal](../procedural-time.md):
`Unknown`, `Date`, `Minute` y `Second`, con desfase opcional cuando hay datos.
Conservar fecha u hora local sin desfase y minuto sin segundos. Solo `Second`
con desfase declarado permite consultar un instante; UTC expreso y ausencia
son distintos. No completar medianoche, segundos, zona IANA ni desfase.

Tiempo del acto, recepcion, efectos expresamente asentados y captura del servidor
son conceptos distintos y se conservan conforme al contrato de hechos.
El valor temporal no consulta Clock ni impone la politica no-futuro de otro
recurso. Sus extremos civiles y UTC siguen su contrato, sin conversion implicita
ni cambios de formato para tiempos historicos de etapas o resultados.

### Frontera de implementacion y verificacion

Implementar mediante TDD: primero pruebas que fallen para identidad, precision,
referencias exactas, correccion frente a nueva practica y denegacion de acceso.
El servicio implementado autentica por rol antes de consultar sus puertos,
prepara y verifica fuentes sin reservar una revision, y reautentica al mismo
actor antes de devolver el borrador. El envio vuelve a preparar, compara el
digest esperado y reautentica antes de solicitar la confirmacion. Comprueba el
recibo devuelto contra comando, actor, identidad fija y fuentes.

Alta y correccion admiten el lote directo de cero a dos versiones en una llamada
cuando no esta vacio, por cada preparacion, incluida la del envio, con limites
compartidos. Los antecedentes historicos no
agregan archivos al lote. Retiro conserva valores y fuentes y rechaza material
nuevo. Las lecturas comprueban expediente, revision exacta, recibos y paginacion;
no aplican el rechazo de escritura a expedientes actualmente cerrados o declaraciones retiradas.

El adaptador PostgreSQL confirma raiz, revision, recibo y auditoria en una
transaccion. Revalida permiso, pertenencia y expediente activo bajo el bloqueo
comun, con lectura vigente, revision esperada y operacion unica. Captura autor,
administracion y Clock en esa frontera.
Mantener la politica de [administracion](../case-administration-api.md): cierre
bloquea cambios y conserva consultas autorizadas. No ampliar acceso de Client.

Seguir la [frontera auditada](0016-case-document-transactions.md), con pruebas
reales de carreras, revocacion entre preparacion y confirmacion, fuentes alteradas,
fallos y rollback. El inventario y las pruebas de restauracion conservan y validan revisiones,
referencias y recibos; el cierre ejecutado se registra en el informe de verificacion. Una respuesta incierta requiere conciliacion exacta antes
de reenviar; una coincidencia visual no demuestra confirmacion de la operacion.

No crear outbox ni intenciones sin consumidor. Los hechos persisten su auditoria,
sin simular evaluaciones, trabajo pendiente o alertas enviadas.
La futura integracion del evaluador definira su frontera durable, dependencias,
recuperacion y reevaluacion atomica antes de habilitar resultados operativos.

## Consequences

- Las revisiones permiten rectificar selecciones manteniendo identidad estable
  e historia reproducible. Los consumidores deberan fijar revisiones y digests;
  no bastara conservar un UUID o consultar una cabeza mutable.
- El [modelo puro](../procedural-facts.md) fija catalogos descriptivos, campos,
  desconocimiento, personas y representacion. Los canones PFRES1/PFNOT1 preservan
  esas declaraciones. La aplicacion coordina preparacion, envio y lecturas con
  PFSRC1/PFTXN1, autenticacion y reautenticacion. El adaptador persistente agrega
  transaccion auditada, membresia real, fuentes historicas e inventario. Las
  pruebas de puertos y los constructores puros siguen sin acreditar por si solos
  esas propiedades; requieren comprobacion PostgreSQL.
- El modelo admite hasta un soporte directo por resolucion y dos por notificacion,
  conservando funciones/localizadores y rechazando digests contradictorios. Estan
  implementados la admision en el servicio, los recibos y el presupuesto compartido
  de documentos directos, separados de antecedentes historicos resueltos. No
  eludir limites dividiendo el trabajo en lotes. El esquema y la transaccion estan
  implementados. La API agrega cuerpos estrictos de 512 KiB, rutas por familia
  y padre, y proyecciones historicas. Su comprobacion integrada con servicios
  reales y restauracion esta aprobada localmente. Qadra implementa captura, fuentes
  historicas, consulta y conciliacion explicita; su verificacion integral esta aprobada localmente.
- Queda pendiente el evaluador: perfiles revisados, aplicabilidad, calendario exacto,
  responsable, canal y corte temporal, discrepancias, consumidores y reevaluacion.
  Este ADR no fija reglas juridicas, un catalogo universal ni una formula mensual.
- Registrar estos hechos no habilita calculo automatico, alertas o recursos ni
  completa su alcance aprobado. La implementacion y sus evidencias deben cerrar
  esos contratos posteriores antes de atribuir tales capacidades al producto.
