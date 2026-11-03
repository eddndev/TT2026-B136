//! Maps the stable refusals of the contract triggers to contract errors, so
//! a race lost after the adapter's own checks answers the same code.

use application::{
    contracts::{ContractBlockers, ContractWorkflowError as Error},
    document_signing::SigningWorkflowError as Signing,
    ApplicationError,
};

pub(crate) fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("contract database operation failed".into())
}

pub(super) fn inconsistent() -> ApplicationError {
    Error::Inconsistent.into()
}

fn blocked(client: bool, firm: bool, document: bool) -> ApplicationError {
    Error::PartyIneligible(ContractBlockers {
        client_not_ready: client,
        firm_signer_not_ready: firm,
        document_not_ready: document,
    })
    .into()
}

/// The contract error of a refused statement, or a port error.
pub(super) fn refusal(error: postgres::Error) -> ApplicationError {
    let Some(db) = error.as_db_error() else {
        return port(error);
    };
    let message = db.message();
    match (db.code().code(), message) {
        ("40001", "contract revision conflict") => Error::RevisionConflict.into(),
        ("23505", _) if db.constraint() == Some("contracts_document_key") => Error::Exists.into(),
        ("23505", _) => Signing::Conflict.into(),
        ("23503", _) => ApplicationError::PermissionDenied,
        ("42501", "contract client is not eligible") => blocked(true, false, false),
        ("42501", "contract firm signer is not eligible") => blocked(false, true, false),
        ("42501", "contract case is closed") => ApplicationError::CaseClosed,
        ("42501", "contract signer is not currently authorized") => Signing::AccountChanged.into(),
        ("42501", _) => ApplicationError::PermissionDenied,
        ("23514", "contract request is open" | "contract request must expire first") => {
            Error::RequestOpen.into()
        }
        ("23514", "contract is closed") => Error::RequestClosed.into(),
        ("23514", "contract event is out of turn" | "contract signature is out of turn") => {
            Error::OutOfTurn.into()
        }
        ("23514", "contract request has expired") => Error::Expired.into(),
        ("23514", "contract version is frozen") => Error::VersionFrozen.into(),
        ("23514", "contract version is superseded") => Error::VersionSuperseded.into(),
        ("23514", "contract document version is not available") => blocked(false, false, true),
        (
            "23514",
            "contract party binding is inactive"
            | "contract signing binding is inactive or inconsistent",
        ) => Signing::InactiveBinding.into(),
        ("23514", "contract signing current trust differs") => Signing::TrustChanged.into(),
        _ => port(error),
    }
}
