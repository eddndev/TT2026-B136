//! The validation entry point: admission, profile recognition, the checks
//! of the recognized profile and the overall result.
//!
//! The overall result is failed if any mandatory component failed, not
//! supported if any is not supported or no profile matches, partial
//! (indeterminate) if any is indeterminate, and complete (checked)
//! otherwise; the current status never changes it. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use crate::archive;
use crate::certificate;
use crate::checks::{self, Evidence};
use crate::policy::Policy;
use crate::profile;
use crate::report::{
    hex, Component, ComponentKind, Finding, Instant, InstantSource, Outcome, Report,
};

/// Validate the evidence package `zip` under `policy`.
///
/// The function never fails: every rejection is a component finding of the
/// returned report.
pub fn validate(zip: &[u8], policy: &Policy) -> Report {
    let caller = Instant {
        label: "current status".to_owned(),
        unix: policy.current_instant,
        source: InstantSource::Caller,
    };
    let archive = match archive::admit(zip) {
        Ok(archive) => archive,
        Err(finding) => {
            let mut report = refused(finding);
            report.instants.push(caller);
            report.trust = checks::route_notes(policy);
            return report;
        }
    };
    let digests: Vec<(String, String)> = archive
        .entries
        .iter()
        .map(|entry| (entry.name.clone(), hex(&certificate::sha256(&entry.data))))
        .collect();
    let mut trust = checks::route_notes(policy);
    let layout = match profile::recognize(&archive.names()) {
        Ok(layout) => layout,
        Err(finding) => {
            let mut report = refused(finding);
            report.instants.push(caller);
            report.digests = digests;
            report.trust = trust;
            return report;
        }
    };
    let package = Component::new(
        ComponentKind::Package,
        vec![
            Finding::checked("zip.admitted"),
            Finding::new(
                Outcome::Checked,
                "profile.recognized",
                format!("{:?}, document {}", layout.profile, layout.document),
            ),
        ],
    );
    let checked = checks::run(&Evidence {
        archive: &archive,
        layout: &layout,
        policy,
    });
    let mut components = vec![package];
    components.extend(checked.components);
    let mut report = Report::from_components(Some(layout.profile), components);
    report.instants = checked.instants;
    report.instants.push(caller);
    report.digests = digests;
    trust.extend(checked.trust);
    report.trust = trust;
    report.current_status = checked.current;
    report
}

/// The report of a package refused before any profile check.
fn refused(finding: Finding) -> Report {
    Report::from_components(
        None,
        vec![Component::new(ComponentKind::Package, vec![finding])],
    )
}
