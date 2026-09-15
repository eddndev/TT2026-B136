# ADR-0023: Adopción y transiciones auditadas de etapa

## Estado

Aceptado como decisión de diseño. La integración y sus resultados ejecutados
se registran por separado en [el informe de verificación](../verification-report.md).

## Contexto

[ADR-0022](0022-audited-penal-case-administration.md) conserva el registro
inicial de Investigación al dar de alta un expediente penal. Las altas básicas
y los expedientes anteriores pueden carecer de etapa. Una edición de la ficha
administrativa no debe inventar una etapa inicial ni cambiar su historia.

El catálogo de `latex/chapters/03-analisis-diseno.tex` requiere registrar la
acusación y el paso a Juicio con soportes documentales. La existencia de un
archivo, su nombre o su sellado no prueban la procedencia jurídica del acto.
Las fechas conocidas pueden tener precisión de día; añadirles una hora
ficticia destruiría esa distinción.

## Decisión

### Registro y secuencia

Se agrega un flujo independiente de administración, con revisión de etapa
propia. Un expediente sin etapa admite una adopción explícita de Investigación,
Intermedia o Juicio, con motivo, fecha conocida y soporte. El registro inicial
original y la adopción son alternativas excluyentes para la revisión 1.

Los avances ordinarios son Investigación a Intermedia e Intermedia a Juicio.
El primero captura fecha declarada de acusación y su soporte. El segundo
captura emisión del auto, soporte, recepción, tribunal receptor y, si existen,
referencia y constancia de recepción. No hay retroceso, sobrescritura histórica
ni inferencia automática al cargar, clasificar o sellar un archivo.

El origen inicial conserva su revisión y digest administrativos CADM1, fecha
y autor originales. Las nuevas entradas contienen valores CSTG1, SHA-256,
etapa anterior, revisión administrativa vigente y su digest, fecha de captura
y UUID/correo del actor. Las revisiones administrativas y procesales no se
incrementan mutuamente.

### Fechas y textos

Una fecha declara día local y desfase conocido; un instante declara fecha,
hora y desfase. Se conserva esa precisión y el desfase original, con minutos
enteros entre −14 y +14 horas. Se rechaza el desfase desconocido `-00:00`.
Los años locales y UTC deben permanecer entre 1 y 9999.

El día representa un intervalo para comprobar límites, sin persistir un acto
ficticio a medianoche. Una declaración es futura solo cuando el comienzo de
todo su intervalo supera la captura. Emisión y recepción se rechazan por orden
cuando el comienzo de la emisión supera el final de la recepción. Intervalos
que se solapan no establecen un orden que el usuario no proporcionó.

El motivo de adopción es obligatorio y admite hasta 1000 escalares Unicode;
la nota opcional tiene el mismo límite. Ambos normalizan CRLF a LF y admiten
varias líneas. Tribunal y referencia de recepción admiten 200 escalares, en
una línea. Se rechazan controles incompatibles antes de recortar blancos
exteriores. No se modifica el texto interior ni su normalización Unicode.

### Soportes y confirmación

Cada papel documental fija UUID, versión positiva y SHA-256 esperado.
Seleccionar una versión posterior no sustituye la anterior. Una misma
referencia puede cumplir ambos papeles de Juicio si su digest coincide;
los valores conservan ambos papeles y la validación se realiza una sola vez.

La aplicación autentica, prepara las referencias exactas dentro del ámbito
autorizado, verifica cifrado y evidencia capturada, y valida un único lote de
formatos según [ADR-0024](0024-isolated-document-format-admission.md). Después
autentica de nuevo al mismo actor, obtiene la hora de captura y confirma.

La transacción final revisa permisos, asignación, perfil penal completo,
estado administrativo activo, cabeza de etapa y registros documentales
completos, incluida evidencia. Confirma etapa y evento en la misma frontera
auditada. Un sellado concurrente entre preparación y commit exige repetir la
validación; un sellado posterior no cambia la entrada ya registrada. No se
consulta de nuevo después del commit para construir una respuesta de éxito.

Owner gestiona y consulta cualquier expediente; Litigator requiere asignación
vigente; Paralegal asignado consulta. Client no tiene acceso a estas rutas de
personal. Lecturas e historia se auditan. Ausencia y expediente ajeno comparten
respuesta opaca. Una revisión obsoleta produce conflicto, sin reintento ciego.

### Interfaz y resultado incierto

Qadra distingue etapa actual, origen e historia; permite elegir una versión
exacta o cargar un soporte. Cargar y registrar etapa son confirmaciones
separadas. El rechazo de etapa no deshace una carga guardada.

El formulario conserva el borrador ante conflicto. Ante pérdida de respuesta,
consulta etapa e historia antes de habilitar un nuevo envío. Una coincidencia
de valores no acredita por sí sola que aquel envío haya sido confirmado.
No se ofrece una falsa garantía de idempotencia.

## Consecuencias

La secuencia conserva los registros anteriores y sus evidencias, a costa de
consultas y comprobaciones adicionales. Las declaraciones procesales y nombres
de soportes son metadatos autorizados; requieren protección en respaldos y
operación igual que la ficha penal.

Este recurso implementa dos avances ordinarios. El objetivo aprobado también
menciona recursos; su conciliación e implementación son trabajo adicional y
no se declaran resueltas mediante una cuarta arista ficticia. No se modifican
los objetivos académicos por esta decisión.

El contrato HTTP y los ejemplos están en [etapas procesales](../case-stages-api.md).
Las pruebas reproducidas, integración, restauración y revisión visual deben
figurar en el informe antes de declarar cerrada la entrega.
