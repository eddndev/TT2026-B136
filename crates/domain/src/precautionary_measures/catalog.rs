use std::str::FromStr;

use crate::DomainError;

/// A declared class, without a determination of legal applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasureKind {
    PeriodicAppearance,
    FinancialGuarantee,
    AssetSeizure,
    AccountFreeze,
    TravelRestriction,
    CustodyOrInstitution,
    PlaceRestriction,
    ContactRestriction,
    HomeSeparation,
    PublicOfficeSuspension,
    ProfessionalSuspension,
    ElectronicMonitoring,
    HomeConfinement,
    PretrialDetention,
}

impl MeasureKind {
    pub const fn tag(self) -> u8 {
        match self {
            Self::PeriodicAppearance => 0,
            Self::FinancialGuarantee => 1,
            Self::AssetSeizure => 2,
            Self::AccountFreeze => 3,
            Self::TravelRestriction => 4,
            Self::CustodyOrInstitution => 5,
            Self::PlaceRestriction => 6,
            Self::ContactRestriction => 7,
            Self::HomeSeparation => 8,
            Self::PublicOfficeSuspension => 9,
            Self::ProfessionalSuspension => 10,
            Self::ElectronicMonitoring => 11,
            Self::HomeConfinement => 12,
            Self::PretrialDetention => 13,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PeriodicAppearance => "periodic_appearance",
            Self::FinancialGuarantee => "financial_guarantee",
            Self::AssetSeizure => "asset_seizure",
            Self::AccountFreeze => "account_freeze",
            Self::TravelRestriction => "travel_restriction",
            Self::CustodyOrInstitution => "custody_or_institution",
            Self::PlaceRestriction => "place_restriction",
            Self::ContactRestriction => "contact_restriction",
            Self::HomeSeparation => "home_separation",
            Self::PublicOfficeSuspension => "public_office_suspension",
            Self::ProfessionalSuspension => "professional_suspension",
            Self::ElectronicMonitoring => "electronic_monitoring",
            Self::HomeConfinement => "home_confinement",
            Self::PretrialDetention => "pretrial_detention",
        }
    }
}

impl FromStr for MeasureKind {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "periodic_appearance" => Ok(Self::PeriodicAppearance),
            "financial_guarantee" => Ok(Self::FinancialGuarantee),
            "asset_seizure" => Ok(Self::AssetSeizure),
            "account_freeze" => Ok(Self::AccountFreeze),
            "travel_restriction" => Ok(Self::TravelRestriction),
            "custody_or_institution" => Ok(Self::CustodyOrInstitution),
            "place_restriction" => Ok(Self::PlaceRestriction),
            "contact_restriction" => Ok(Self::ContactRestriction),
            "home_separation" => Ok(Self::HomeSeparation),
            "public_office_suspension" => Ok(Self::PublicOfficeSuspension),
            "professional_suspension" => Ok(Self::ProfessionalSuspension),
            "electronic_monitoring" => Ok(Self::ElectronicMonitoring),
            "home_confinement" => Ok(Self::HomeConfinement),
            "pretrial_detention" => Ok(Self::PretrialDetention),
            _ => Err(DomainError::InvalidPrecautionaryMeasure("kind")),
        }
    }
}
