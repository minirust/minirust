use crate::build::*;

impl FunctionBuilder {
    pub fn assign(&mut self, destination: PlaceExpr, source: ValueExpr) {
        self.cur_block().statements.push(Statement::Assign { destination, source });
    }

    pub fn place_mention(&mut self, place: PlaceExpr) {
        self.cur_block().statements.push(Statement::PlaceMention(place));
    }

    pub fn set_discriminant(&mut self, destination: PlaceExpr, value: impl Into<Int>) {
        self.cur_block()
            .statements
            .push(Statement::SetDiscriminant { destination, value: value.into() });
    }

    pub fn validate(&mut self, place: PlaceExpr, fn_entry: bool) {
        self.cur_block().statements.push(Statement::Validate { place, fn_entry });
    }

    pub fn storage_live(&mut self, local: PlaceExpr) {
        let PlaceExpr::Local(name) = local else { panic!("PlaceExpr is not a local") };
        self.cur_block().statements.push(Statement::StorageLive(name));
    }

    pub fn storage_dead(&mut self, local: PlaceExpr) {
        let PlaceExpr::Local(name) = local else { panic!("PlaceExpr is not a local") };
        self.cur_block().statements.push(Statement::StorageDead(name));
    }
}

pub fn assign(destination: PlaceExpr, source: ValueExpr) -> Statement {
    Statement::Assign { destination, source }
}

pub fn place_mention(place: PlaceExpr) -> Statement {
    Statement::PlaceMention(place)
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
