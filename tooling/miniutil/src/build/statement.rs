use crate::build::*;

pub fn assign(destination: PlaceExpr, source: ValueExpr) -> Statement {
    Statement::Assign { destination, source }
}

pub fn set_discriminant(destination: PlaceExpr, value: impl Into<Int>) -> Statement {
    Statement::SetDiscriminant { destination, value: value.into() }
}

pub fn validate(place: PlaceExpr, fn_entry: bool) -> Statement {
    Statement::Validate { place, fn_entry }
}

pub fn storage_live(x: u32) -> Statement {
    Statement::StorageLive(LocalName(Name::from_internal(x)))
}

pub fn storage_dead(x: u32) -> Statement {
    Statement::StorageDead(LocalName(Name::from_internal(x)))
}
