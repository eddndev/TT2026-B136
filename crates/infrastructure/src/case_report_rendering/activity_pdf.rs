use super::{layout::Layout, *};

pub(super) fn render(
    snapshot: &CaseReportSnapshot,
    regular: &[u8],
    bold: &[u8],
) -> Result<Vec<u8>, ApplicationError> {
    let value = activity::payload(snapshot)?;
    let totals = activity::totals(value)?;
    let mut layout = Layout::new(regular, bold)?;
    layout.paragraph("Actividad registrada", true, 17.0)?;
    layout.gap(5.0);
    layout.paragraph("Escrituras confirmadas durante el periodo, atribuidas a la cuenta autora. Las asignaciones posteriores no cambian esa atribuci\u{f3}n. Los conteos no califican resultados jur\u{ed}dicos.", false, 9.5)?;
    layout.gap(8.0);
    for text in [
        format!("Informe: {}", snapshot.report_id),
        format!("Captura SHA-256: {}", snapshot.digest),
        format!("Observado en UTC: {}", bounds::utc(snapshot.checked_at)?),
        format!("Solicitante: {}", snapshot.requester.principal.id),
        format!(
            "Alcance: {}",
            if snapshot.scope == CaseReportScope::Office {
                "Oficina"
            } else {
                "Expedientes asignados"
            }
        ),
        format!(
            "Periodo desde (incluido): {}",
            bounds::utc(snapshot.filters.period_from)?
        ),
        format!(
            "Periodo hasta (excluido): {}",
            bounds::utc(snapshot.filters.period_before)?
        ),
        format!(
            "Filtro de estado: {}",
            bounds::status(snapshot.filters.status)
        ),
        format!(
            "Filtro de autor: {}",
            snapshot
                .filters
                .litigator
                .map(|id| id.to_string())
                .unwrap_or_else(|| "Todos los permitidos".into())
        ),
    ] {
        layout.paragraph(&text, false, 9.0)?;
    }
    layout.gap(8.0);
    if value.documents_complete {
        layout.paragraph(
            "Cobertura documental completa para el periodo capturado.",
            false,
            9.5,
        )?;
    } else {
        layout.paragraph("Cobertura documental incompleta", true, 10.0)?;
        layout.paragraph("Se cuentan documentos con autores identificados. La falta de atribuci\u{f3}n hist\u{f3}rica no equivale a cero documentos registrados.", false, 9.5)?;
    }
    layout.section("Totales de actividad")?;
    layout.paragraph(&activity::text(totals.capture), true, 10.0)?;
    layout.section("Actividad por litigante")?;
    if value.actors.is_empty() {
        layout.paragraph("No hay autores con actividad en esta captura.", false, 10.0)?;
    }
    for (index, (who, total)) in value.actors.iter().zip(totals.actors).enumerate() {
        layout.row(
            &format!("Litigante {}", index + 1),
            vec![
                (who.email.clone(), true, 10.0),
                (format!("ID: {}", who.user_id), false, 9.0),
                (activity::text(total), false, 10.0),
            ],
        )?;
        layout.gap(9.0);
    }
    layout.section("Actividad por expediente y autor")?;
    if value.rows.is_empty() {
        layout.paragraph(
            "No hay actividad registrada que coincida con estos filtros.",
            false,
            10.0,
        )?;
    }
    for (index, entry) in value.rows.iter().enumerate() {
        let case = activity::case(snapshot, entry.case_id)?;
        let who = activity::actor(value, entry.litigator_id)?;
        layout.row(
            &format!("Registro {}", index + 1),
            vec![
                (case.title.clone(), true, 10.0),
                (format!("Expediente: {}", case.case_id), false, 9.0),
                (format!("Referencia: {}", case.reference), false, 9.0),
                (
                    format!("Autor: {} | {}", who.email, who.user_id),
                    false,
                    9.0,
                ),
                (activity::text(activity::counts(entry)), false, 10.0),
            ],
        )?;
        layout.gap(9.0);
    }
    pdf::finish(layout, "Actividad registrada")
}
