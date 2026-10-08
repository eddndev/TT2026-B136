use domain::{
    hearings::HearingNote,
    precautionary_measures::{MeasureCorrectionValues, MeasureTime, MeasureValidity},
    procedural_time::DeclaredProceduralTime,
};

pub fn note(text: &str) -> HearingNote {
    HearingNote::new(text).unwrap()
}

pub fn unknown(reason: &str) -> MeasureTime {
    MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note(reason))).unwrap()
}

pub fn known(declared: DeclaredProceduralTime) -> MeasureTime {
    MeasureTime::new(declared, None).unwrap()
}

pub fn with_times(start: MeasureTime, end: Option<MeasureTime>) -> MeasureCorrectionValues {
    MeasureCorrectionValues::new(
        note("C"),
        MeasureValidity::new(start, note("V"), end).unwrap(),
        note("S"),
    )
}

pub fn correction() -> MeasureCorrectionValues {
    with_times(unknown("U"), None)
}

pub fn unhex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
