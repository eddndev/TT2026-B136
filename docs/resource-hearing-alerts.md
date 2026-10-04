# Alertas de audiencias propias de recursos

## Alcance de la entrega local

Esta ampliación forma parte de las [audiencias propias de recursos](resource-hearings.md).
Su entrega e integración están en curso. Los resultados ejecutados y los controles
pendientes se registran en [verificación](verification-report.md); una prueba de
cliente con HTTP controlado no acredita el consumidor real ni la restauración.
No se activa configuración de correo ni se despliega este trabajo al escribirlo.

## Identidad y evidencia

El sujeto de una notificación propia es
`{kind:"resource_hearing",case_id,resource_id,id}`. La identidad de audiencia no
se interpreta como audiencia ordinaria ni plazo. El padre es el recurso de la
creación original; no se reconstruye desde la asociación que esté activa al leer.
Dos familias pueden compartir UUID sin compartir ocurrencia ni ruta de consulta.

Esta familia admite exclusivamente `upcoming`, con origen de revisión uno y la
huella `capture_digest` de la audiencia. No utiliza el digest de envío, del
recurso o de la asociación. La carga del origen verifica la creación completa:
captura, material histórico, vínculo inicial y marcador auditado. La lectura
conserva la metadata del expediente observada al registrar, sin sustituirla por
un título actual. La forma hexadecimal de la huella no constituye una
verificación criptográfica hecha por el navegador.

`activity_at` corresponde exactamente al instante UTC de la programación
capturada. Las anticipaciones pertenecen a `hearing_upcoming`; no se crea una
preferencia nueva ni se hereda `deadline_upcoming`. El umbral resta las horas
seleccionadas, conservando los componentes temporales exactos. La ventana de
generación es `[trigger_at,activity_at)`. Los avisos que dejan de ser elegibles
conservan su evidencia y se resuelven; no se convierten en plazos vencidos.

## Destinatarios y continuidad

Se conserva la política existente de audiencias: destinatarios staff activos
que sean miembros del expediente. Owner sólo recibe avisos nuevos de esta
familia si es miembro. La participación procesal no equivale a una cuenta ni
concede acceso. Client no accede a alertas.

La bandeja exige destinatario propio y autorización vigente: Owner tiene el
acceso global ya existente sobre sus avisos; Litigator y Paralegal requieren
membresía. Retirar la membresía impide a estos últimos listar, abrir o marcar
avisos del expediente. Leer un aviso no cambia la audiencia ni su vínculo.

Desvincular la asociación, archivar el recurso o cerrar administrativamente el
expediente no cancela el señalamiento original. La familia no dispone de un
comando de cancelación o reprogramación. Esos cambios tampoco eliminan su origen
histórico. Reiniciar el generador no crea otra ocurrencia para el mismo sujeto,
instante y anticipación, ni pierde una lectura ya confirmada.

## Consulta desde Qadra

La tarjeta distingue **Audiencia de recurso próxima** de las otras familias.
Antes de abrir la captura vuelve a consultar la alerta y la administración
autorizada del expediente. Luego solicita únicamente la revisión propia uno:
`GET /api/v1/cases/{case}/procedural-resources/{resource}/activities/resource-hearings/{id}/revisions/1`.
Contrasta expediente, recurso, audiencia, revisión, huella e instante UTC antes
de mostrar el panel histórico reutilizado de Agenda.

El panel conserva la fecha con su desfase original, soporte, participantes,
fuentes del recurso y acto, autor y vínculo inicial. No afirma que esa asociación
continúe activa. Cerrar el panel conserva los filtros de la bandeja; cambiar la
consulta o abandonar su contexto invalida respuestas pendientes. Un rechazo de
autorización retira la información del expediente. Abrir el detalle sólo consulta:
no crea otra audiencia, no envía correo y no marca automáticamente el aviso.

## Persistencia y restauración

La representación histórica de audiencias ordinarias y plazos conserva sus tags
y bytes. La familia propia usa tag 2 y el arreglo `[2,case,id,resource]`.
Las proyecciones de planificación y notificación deben conservar el mismo padre
que la raíz verificada, con restricciones, permisos e inventario de apertura.
El recurso padre no es un campo editable de una alerta.

El respaldo debe conservar conjuntamente audiencias, asociaciones, recursos,
auditoría y tablas de alertas. Tras restaurar, se reponen los permisos runtime y
se valida el inventario antes de admitir tráfico. Las pruebas de desconexión,
reapertura y alteración de inventario no sustituyen una campaña completa de
`pg_dump`/`pg_restore` y aceptación HTTP real.

Los canales siguen el contrato general de [alertas](alerts-api.md). Una
preferencia de correo activa no equivale a un transporte habilitado ni a una
entrega. La aceptación del proveedor y la lectura interna son estados separados.
