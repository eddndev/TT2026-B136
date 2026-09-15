# ADR-0022: Administración auditada del expediente penal

## Estado

Aceptado. Implementado y comprobado mediante pruebas de dominio, aplicación,
HTTP, PostgreSQL y navegador. Los resultados reproducidos se registran en
[el informe de verificación](../verification-report.md).

## Contexto

El expediente de [ADR-0014](0014-case-membership-and-isolation.md) conserva
identidad, título, referencia libre, creador y asignaciones. Esa referencia no
es un NUC ni una carpeta judicial verificados. El catálogo de
`latex/chapters/03-analisis-diseno.tex` exige un registro penal más completo,
edición e historia, mientras documentos y participantes ya preservan sus
propias revisiones y evidencia.

Incorporar esos datos debe conservar los expedientes existentes y la API básica,
sin inventar números, autoridades, autores ni etapas anteriores. El cierre del
trabajo en el despacho requiere una regla operativa independiente del estado
jurídico del proceso. La autorización vigente y la auditoría deben abarcar
también las consultas del expediente.

## Decisión

### Identidad, perfil y administración

Se separan la raíz estable del expediente, las revisiones de administración
y el registro de etapa. Las revisiones contienen título, referencia interna,
estado administrativo y un perfil penal opcional. Un perfil presente es
completo; una vez incorporado no puede volver a ausente.

| Dato | Límite en escalares Unicode | Regla |
| --- | ---: | --- |
| Título | 200 | Obligatorio. |
| Referencia interna | 100 | Obligatoria, libre y no única. |
| NUC | 100 | Obligatorio en el perfil. |
| Autoridad emisora del NUC | 200 | Obligatoria en el perfil. |
| Carpeta judicial | 100 | Obligatoria en el perfil. |
| Órgano emisor de la carpeta | 200 | Obligatorio en el perfil. |
| Delitos registrados | De 1 a 8 entradas, de hasta 120 cada una | Descripciones manuales; orden conservado y duplicados literales rechazados. |
| Información general | 1000 | Opcional, con varias líneas. |
| Identificadores complementarios | 300 | Opcionales; no sustituyen NUC ni carpeta. |

Los valores son declaraciones del despacho. No se deduce identidad oficial,
competencia de una autoridad, naturaleza de un delito ni culpabilidad a partir
de ellos. No se impone un formato nacional de identificadores ni un catálogo
normativo que el prototipo no haya validado.

Se rechazan controles antes de recortar blancos Unicode exteriores. Información
general admite LF y normaliza CRLF a LF; rechaza CR aislado y los demás
controles. Los restantes textos rechazan todos los controles. No se cambian
mayúsculas, acentos, normalización Unicode ni espacios interiores. Los opcionales
vacíos se representan ausentes.

NUC y carpeta son únicos por separado entre los perfiles actuales de la
instancia del despacho, incluidos los cerrados. La comparación es literal
después de normalizar la entrada. Las autoridades aportan contexto y no cambian
esas claves. Corregir un identificador libera el anterior después del commit,
conservando su historia. El conflicto no revela el expediente que lo ocupa.
Esta regla conserva el criterio del catálogo; puede producir colisiones entre
números locales de órganos diferentes y no acredita unicidad nacional.

### Altas y compatibilidad

La interfaz ofrece una sola alta penal completa. Confirma en una transacción
raíz, primera revisión activa, asignación del creador, registro inicial de
Investigación y sus eventos. Ese registro describe la etapa declarada al crear
el expediente; no acredita un acto judicial.

La API básica conserva sus cuatro campos de respuesta y su colección paginada.
Su alta crea un expediente con perfil pendiente, con una revisión
uno real y sin etapa inventada. No es un fallback automático de la interfaz
cuando falle el alta penal. Las consultas básicas devuelven título y referencia
vigentes, revalidando actor y alcance antes de confirmar auditoría.

Las raíces anteriores sin revisiones se proyectan como revisión cero, estado
activo por compatibilidad, perfil pendiente y etapa sin registrar. No se crea
una fila histórica, fecha ni autor retrospectivos. Completar un perfil pendiente
añade una revisión real y no asigna automáticamente una etapa. La adopción
de una etapa existente y las transiciones requieren contratos posteriores.

### Revisiones, consultas y permisos

Owner administra todos los expedientes; Litigator asignado puede consultar,
editar y cambiar estado; Paralegal asignado consulta. Client conserva la
proyección básica de sus expedientes, sin perfiles, historia administrativa ni
correos de autores. Las asignaciones permanecen reservadas a Owner.

La edición reemplaza únicamente título, referencia y perfil con revisión
esperada. El comando exclusivo de estado conserva los textos vigentes dentro
de la transacción. Las revisiones positivas son contiguas y no se desbordan;
la proyección cero solo puede ser la base esperada de una raíz sin revisiones.
Los comandos devuelven su propia instantánea confirmada, sin una consulta
posterior para reconstruir el resultado.

Un índice autorizado para el personal del despacho selecciona la cabeza antes
de filtrar y paginar por UUID ascendente. La historia usa revisiones
descendentes y cursores exclusivos. Las consultas confirman su auditoría antes
de entregar datos y no prometen una instantánea compartida entre páginas.

El canon de valores usa `CADM1`, estado, título, referencia, presencia del perfil,
sus cuatro identificadores y autoridades, lista ordenada de delitos y los dos
opcionales. Los textos usan longitud UTF-8 de cuatro bytes en orden de mayor
a menor peso; los opcionales tienen una etiqueta de presencia de un byte y
la lista un contador de cuatro bytes. El máximo es 12 717 bytes. SHA-256
cubre los valores; identidad, revisión y autoría se vinculan por claves y
recurso auditado. Los cuerpos JSON de alta, edición y estado tienen un límite completo
de 64 KiB para admitir también los textos máximos escapados.

### Cierre administrativo

El cierre bloquea nuevas cargas, versiones, clasificación y sellado documental;
mutaciones del directorio; edición del perfil y futuras transiciones. Permite
consultar, verificar, exportar y revisar historia con los permisos vigentes.
Owner puede asignar y revocar acceso sin reabrir. Reactivar el expediente no
reactiva fichas archivadas ni cambia su etapa.

Cada mutación comprueba el estado después de resolver autorización y recurso,
dentro de la transacción que confirma el cambio. La comprobación anterior a
cifrar o solicitar una TSA no sustituye esa revalidación. Si el cierre gana
la carrera, una operación preparada previamente se rechaza sin nuevo estado
ni evento de éxito. Cerrar administrativamente no termina ni reabre un proceso
judicial, ni modifica sus plazos.

### Persistencia y recuperación

La migración es aditiva. Las altas normales exigen R1 mediante una referencia
diferida desde la raíz; solo raíces anteriores o baselines preparados con cuenta
administrativa pueden carecer de esa obligación. El rol operativo no puede
insertar el marcador de baseline ni alterar raíces, revisiones o historia.

La unicidad sobre cabezas se comprueba bajo el bloqueo exclusivo común de
auditoría y un aislamiento explícito READ COMMITTED. No se utiliza un índice
único sobre toda la historia ni una consulta previa sin serialización. El
arranque comprueba inventario, continuidad, valores canónicos, huellas,
restricciones y privilegios. Estas comprobaciones no autentican cualquier DDL
de un administrador confiable ni detectan una reversión coherente de toda la base.

El primer import sigue exigiendo auditoría vacía. Sus destinos se preparan
administrativamente como baselines sin historia inventada; crear expedientes
por HTTP y borrar luego sus revisiones o eventos no es una preparación válida.
La restauración íntegra conserva todas las tablas e historias. Conciliar un
recibo existente permite actividad administrativa posterior válida y preserva
el prefijo importado y la evidencia documental exacta.

## Consecuencias

- El registro penal puede completarse sin reinterpretar referencias antiguas ni
  ampliar el acceso de Client a información sensible.
- El cierre requiere cambios coordinados en los adaptadores de documentos y
  participantes; ocultar controles de interfaz no constituye su implementación.
- Se mantienen historias independientes para administración, participantes,
  documentos y etapa, vinculadas por expediente y auditoría.
- La comparación literal y los campos manuales tienen límites explícitos; no
  sustituyen validación institucional o jurídica.
- La entrega incluye pruebas de permisos, fallos, concurrencia, migración,
  importación y restauración, además de recorridos Qadra reales y revisión
  académica. Su resultado se registra en `docs/verification-report.md`.
