# Qadra: kit de evaluación de usabilidad

**Tu despacho, en orden.** Material para observar cómo personas del despacho
usan los flujos disponibles del prototipo con datos exclusivamente ficticios.
El kit está preparado para ejecución; **no contiene participantes ni resultados**.
No acredita por sí mismo usabilidad, seguridad, validez jurídica o puesta en servicio.

## Uso inmediato

1. La persona responsable lee el [protocolo](protocol.md), registra la versión
   exacta del producto y elige las tareas habilitadas en esa instalación.
2. La persona que modera prepara el entorno con la
   [lista de comprobación](facilitator.md) y el [catálogo de tareas](tasks.md).
3. Completa los datos de responsable, contacto y conservación de información en
   el [consentimiento](templates/consent.md). Obtén consentimiento antes de tomar notas.
4. Entrega sólo la [hoja del participante](participant.md) y las tarjetas de
   tarea elegidas. Las respuestas esperadas de `tasks.md` son para moderación.
5. Duplica las [plantillas vacías](templates/) en una ubicación privada fuera del
   repositorio. Sigue el [diccionario de datos](data-dictionary.md) para registrar
   cada sesión, intento y observación. No completes las plantillas versionadas.
6. Resume los resultados realmente obtenidos, informa exclusiones y distingue
   problemas de interfaz, fallos del producto y fallos del entorno. Conserva la
   relación entre cada conclusión y sus observaciones.

## Materiales

| Archivo | Uso |
| --- | --- |
| [protocol.md](protocol.md) | Alcance, selección, tiempos y reglas de análisis. |
| [facilitator.md](facilitator.md) | Preparación y guion neutral de moderación. |
| [tasks.md](tasks.md) | Precondiciones, resultado observable y límites por tarea. |
| [participant.md](participant.md) | Instrucciones y hoja de respuesta de la persona participante. |
| [consent.md](templates/consent.md) | Consentimiento voluntario y separado para capturas. |
| [sessions.csv](templates/sessions.csv) | Una fila por sesión y condiciones observadas. |
| [attempts.csv](templates/attempts.csv) | Una fila por tarea planificada e intento. |
| [observations.csv](templates/observations.csv) | Hechos, errores, ayudas y evidencia. |
| [summary.csv](templates/summary.csv) | Agregados vacíos, derivados de los registros. |
| [data-dictionary.md](data-dictionary.md) | Campos, estados y reglas de consistencia. |
| [fixtures/README.md](fixtures/README.md) | Caso ficticio y archivos de práctica. |

## Qué se puede evaluar

Los recorridos base usan acceso con MFA, expedientes asignados, carga documental,
versiones, sellado técnico, verificación, evidencia, agenda, asignaciones y cierre
de sesión. Deben estar comprobados en la versión elegida antes de convocar personas.

Informes y consulta filtrada de auditoría son **módulos condicionales**: se incluyen
sólo después de comprobar su aceptación integrada y disponibilidad en esa versión.
La verificación de la cadena de auditoría es una función separada. El estado de
integración o despliegue se registra, no se presume por existir una pantalla.

Invitaciones, recuperación autónoma de contraseña o enrolamiento, expiración por
inactividad con borradores recuperables, firma personal, autenticación con
certificado y activación jurídica automática no se presentan como implementados.
No se piden datos reales de clientes ni decisiones sobre un caso jurídico real.

Referencias: [manual de uso](../user-guide.md),
[estado del producto](../product-completion.md),
[informe de verificación](../verification-report.md),
[objetivos aprobados](../../latex/chapters/01-introduccion.tex) y
[análisis y diseño](../../latex/chapters/03-analisis-diseno.tex).
