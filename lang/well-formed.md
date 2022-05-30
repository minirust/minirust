# MiniRust well-formedness requirements

The various syntactic constructs of MiniRust (types, functions, ...) come with well-formedness requirements: certain invariants need to be satisfied for this to be considered a well-formed program.
The idea is that for well-formed programs, the `step` function will never panic.
Those requirements are defined in this file.

Note that `check` functions for testing well-formedness return `Option<()>` rather than `bool` so that we can use `?`.

## Well-formed layouts and types

```rust
impl IntType {
    fn check(self) -> Option<()> {
        if !self.size.bytes().is_power_of_two() { return None; }
    }
}

impl Layout(self) {
    fn check(self) -> Option<()> {
        // Size must be a multiple of alignment.
        if self.size.bytes() % self.align.bytes() != 0 { return None; }
    }
}

impl Type {
    fn check_fields(fields: Fields, total_size: Size) -> Option<()> {
        // The fields must not overlap.
        fields.sort_by_key(|(offset, type)| offset);
        let mut last_end = Size::ZERO;
        for (offset, type) in fields {
            // Recursively check the field type.
            type.check()?;
            // And ensure it fits after the one we previously checked.
            if offset < last_end { return None; }
            last_end = offset.checked_add(type.size())?;
        }
        // And they must all fit into the size.
        if size < last_end { return None; }
    }

    fn check(self) -> Option<()> {
        use Type::*;
        match self {
            Int(int_type) => {
                int_type.check()?;
            }
            Bool => {
            }
            Ref { pointee, .. } | Box { pointee } | RawPtr { pointee, .. } => {
                pointee.check()?;
            }
            Tuple { fields, size, align } => {
                Type::check_fields(fields, size)?;
            }
            Enum { variants, size, .. } => {
                for variant in variants {
                    Type::check_fields(variant, size)?;
                }
            }
            Union { fields, size } => {
                // These may overlap, but they must all fit the size.
                for (offset, type) in fields {
                    type.check()?;
                    if size < offset.checked_add(type.size())? { return None; }
                }
            }
        }
    }
}
```

## Well-formed expressions, functions, and programs

- TODO: define this
