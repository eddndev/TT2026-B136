# ADR 0032: Aritmetica temporal explicita para plazos

## Status

Accepted. La decision cubre aritmetica pura; perfiles normativos y gestion
operativa de plazos permanecen pendientes.

## Context

El conteo existente recorre fechas computables de un calendario exacto. No cubre
horas transcurridas ni meses civiles. Reutilizarlo mediante conversiones de meses
en dias, o completar componentes de tiempos parciales, produciria resultados
que no se desprenden de las entradas. Los hechos declarados conservan precision
y fuentes, pero sus textos y clases generales no califican un supuesto juridico.

## Decision

Introducir un calculo puro con unidad, inclusion y ajuste final expresos. Dias
computables reutilizan el conteo existente; naturales usan desplazamiento civil;
meses desplazan ano/mes directamente y bloquean si falta el dia homologo; horas
requieren segundo y desfase para obtener un instante UTC. Ninguna politica se
elige por defecto. Conservar operandos, candidata intermedia, fuentes consultadas
y causa de bloqueo. El contrato completo vive en
[deadline-arithmetic.md](../deadline-arithmetic.md).

Separar este resultado matematico de calificacion, perfil normativo, identidad de
insumos, estado operativo y entrega de alertas. No modificar los canones de
hechos o calendarios ni importar snapshots de aplicacion al dominio.

## Consequences

Las unidades se prueban con vectores matematicos independientes y limites
explícitos. Un mes sin homologo queda pendiente hasta que un perfil respaldado
establezca la politica aplicable. Una fecha calculada carece de corte horario y
no permite programar alertas de horas por si sola. La evaluacion operativa aun
requiere datos estructurados, fuentes exactas, autorizacion, transacciones,
historia, reevaluacion durable e interfaz. El alcance aprobado no se reduce a
este componente.
