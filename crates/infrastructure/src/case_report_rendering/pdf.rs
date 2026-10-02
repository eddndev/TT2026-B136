use super::{
    layout::{Layout, PAGE_HEIGHT, PAGE_WIDTH},
    *,
};
use domain::case_administration::CaseAdministrativeStatus;
use pdf_writer::{Finish, Name, Pdf, Rect, Ref, TextStr};

pub(super) fn render(
    snapshot: &CaseReportSnapshot,
    regular: &[u8],
    bold: &[u8],
) -> Result<Vec<u8>, ApplicationError> {
    let mut layout = Layout::new(regular, bold)?;
    layout.paragraph("Estado actual de expedientes", true, 17.0)?;
    layout.gap(5.0);
    layout.paragraph("Expedientes creados en el intervalo indicado. El estado y las asignaciones corresponden a la captura, no al pasado ni a la actividad realizada durante el periodo.", false, 9.5)?;
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
            "Creaci\u{f3}n desde (incluido): {}",
            bounds::utc(snapshot.filters.created_from)?
        ),
        format!(
            "Creaci\u{f3}n hasta (excluido): {}",
            bounds::utc(snapshot.filters.created_before)?
        ),
        format!(
            "Filtro de estado: {}",
            bounds::status(snapshot.filters.status)
        ),
        format!(
            "Filtro de litigante: {}",
            snapshot
                .filters
                .assigned_litigator
                .map(|id| id.to_string())
                .unwrap_or_else(|| "Todos los permitidos".into())
        ),
    ] {
        layout.paragraph(&text, false, 9.0)?;
    }
    let active = snapshot
        .cases
        .iter()
        .filter(|case| case.status == CaseAdministrativeStatus::Active)
        .count();
    layout.gap(8.0);
    layout.paragraph(
        &format!(
            "Total de expedientes: {}    Activos: {}    Cerrados: {}",
            snapshot.cases.len(),
            active,
            snapshot.cases.len() - active
        ),
        true,
        10.0,
    )?;
    layout.section("Expedientes")?;
    if snapshot.cases.is_empty() {
        layout.paragraph(
            "No hay expedientes que coincidan con estos filtros.",
            false,
            10.0,
        )?;
    }
    for (index, case) in snapshot.cases.iter().enumerate() {
        let state = if case.status == CaseAdministrativeStatus::Active {
            "Activo"
        } else {
            "Cerrado"
        };
        let mut paragraphs = vec![(case.title.clone(), true, 10.0)];
        for text in [
            format!("ID: {}", case.case_id),
            format!("Referencia: {}", case.reference),
            format!("Creado en UTC: {}", bounds::utc(case.created_at)?),
            format!(
                "Revisi\u{f3}n administrativa: {}",
                case.administration_revision
                    .map(|revision| revision.get().to_string())
                    .unwrap_or_else(|| "Sin revisi\u{f3}n capturada".into())
            ),
            format!(
                "Captura administrativa: {}",
                case.administration_digest
                    .map(|digest| digest.to_string())
                    .unwrap_or_else(|| "No disponible".into())
            ),
        ] {
            paragraphs.push((text, false, 9.0));
        }
        if case.assigned_litigators.is_empty() {
            paragraphs.push(("Sin litigantes activos asignados.".into(), false, 9.0));
        }
        for who in &case.assigned_litigators {
            paragraphs.push((
                format!("Litigante: {} | {}", who.email, who.user_id),
                false,
                9.0,
            ));
        }
        layout.row(&format!("Expediente {}  |  {state}", index + 1), paragraphs)?;
        layout.gap(9.0);
    }
    layout.section("Carga por litigante")?;
    layout.paragraph("Conteos sobre los mismos expedientes capturados. Un expediente con varias asignaciones aparece en la carga de cada litigante; estas sumas no son un total de expedientes distintos.", false, 9.5)?;
    layout.gap(7.0);
    if snapshot.workload.is_empty() {
        layout.paragraph("No hay litigantes con carga en esta captura.", false, 10.0)?;
    }
    for (index, row) in snapshot.workload.iter().enumerate() {
        let total = row
            .active_cases
            .checked_add(row.closed_cases)
            .ok_or_else(capacity)?;
        layout.row(
            &format!("Litigante {}", index + 1),
            vec![
                (row.litigator.email.clone(), true, 10.0),
                (format!("ID: {}", row.litigator.user_id), false, 9.0),
                (
                    format!(
                        "Activos: {}    Cerrados: {}    Total: {total}",
                        row.active_cases, row.closed_cases
                    ),
                    false,
                    10.0,
                ),
            ],
        )?;
        layout.gap(9.0);
    }
    let layout = layout.finish()?;
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let tree = Ref::new(2);
    pdf.catalog(catalog).pages(tree);
    pdf.document_info(Ref::new(15))
        .title(TextStr("Estado actual de expedientes"))
        .creator(TextStr("Qadra"));
    super::embed::font(&mut pdf, &layout.fonts[0], 3, false)?;
    super::embed::font(&mut pdf, &layout.fonts[1], 9, true)?;
    let pages: Vec<Ref> = (0..layout.pages.len())
        .map(|index| Ref::new(100 + index as i32 * 2))
        .collect();
    pdf.pages(tree)
        .kids(pages.iter().copied())
        .count(pages.len() as i32);
    for (index, (page_id, content)) in pages.iter().zip(&layout.pages).enumerate() {
        if pdf
            .len()
            .checked_add(content.len() + 65536)
            .ok_or_else(capacity)?
            > MAX_REPORT_ARTIFACT_BYTES
        {
            return Err(capacity());
        }
        let content_id = Ref::new(101 + index as i32 * 2);
        let mut page = pdf.page(*page_id);
        page.parent(tree)
            .media_box(Rect::new(0.0, 0.0, PAGE_WIDTH, PAGE_HEIGHT))
            .contents(content_id);
        page.resources()
            .fonts()
            .pair(Name(b"F1"), Ref::new(3))
            .pair(Name(b"F2"), Ref::new(9));
        page.finish();
        pdf.stream(content_id, content);
    }
    let bytes = pdf.finish();
    if bytes.len() > MAX_REPORT_ARTIFACT_BYTES {
        return Err(capacity());
    }
    Ok(bytes)
}
