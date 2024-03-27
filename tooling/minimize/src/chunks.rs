use crate::*;

/// Calculates the Chunks for a union type.
/// This works roughly as described here:
/// https://github.com/rust-lang/unsafe-code-guidelines/issues/354#issuecomment-1297545313
pub fn calc_chunks(fields: Fields, size: Size) -> List<(Offset, Size)> {
    let s = size.bytes().try_to_usize().unwrap();
    let mut markers = vec![false; s];
    for (offset, ty) in fields {
        let offset = offset.bytes().try_to_usize().unwrap();
        mark_used_bytes(ty, &mut markers[offset..]);
    }

    let mut chunks = Vec::new();
    let mut current_chunk_start: Option<usize> = None;

    // this garantees that `markers` ends with false,
    // hence the last chunk will be added.
    markers.push(false);

    for (i, b) in markers.iter().enumerate() {
        match (b, &current_chunk_start) {
            (true, None) => {
                current_chunk_start = Some(i);
            }
            (false, Some(s)) => {
                let start = Offset::from_bytes(*s).unwrap();
                let length = Size::from_bytes(i - *s).unwrap();
                chunks.push((start, length));
                current_chunk_start = None;
            }
            _ => {}
        }
    }

    chunks.into_iter().collect()
}

// The `markers` object stores a bool for each byte within the size of a union.
// Such a bool is `true` if the corresponding byte should be part of a chunk (i.e. it contains actual data),
// and it's false if this byte is just padding.
//
// marks any non-padding bytes used by `ty` as `true`.
fn mark_used_bytes(ty: Type, markers: &mut [bool]) {
    match ty {
        Type::Int(int_ty) => mark_size(int_ty.size, markers),
        Type::Bool => mark_size(Size::from_bytes_const(1), markers),
        Type::Ptr(_) => mark_size(DefaultTarget::PTR_SIZE, markers),
        Type::Tuple { fields, .. } =>
            for (offset, ty) in fields {
                let offset = offset.bytes().try_to_usize().unwrap();
                mark_used_bytes(ty, &mut markers[offset..]);
            },
        Type::Union { chunks, .. } =>
            for (offset, len) in chunks {
                let offset = offset.bytes().try_to_usize().unwrap();
                mark_size(len, &mut markers[offset..]);
            },
        Type::Array { elem, count } => {
            let elem = elem.extract();
            for i in Int::ZERO..count {
                let offset = i * elem.size::<DefaultTarget>();
                let offset = offset.bytes().try_to_usize().unwrap();
                mark_used_bytes(elem, &mut markers[offset..]);
            }
        }
        Type::Enum { variants, .. } =>
            for Variant { ty, tagger } in variants.values() {
                mark_used_bytes(ty, markers);
                for (offset, (ity, _)) in tagger {
                    let offset = offset.bytes().try_to_usize().unwrap();
                    mark_size(ity.size, &mut markers[offset..]);
                }
            },
    }
}

// marks all bytes from 0..size as true.
fn mark_size(size: Size, markers: &mut [bool]) {
    for i in Int::ZERO..size.bytes() {
        let i = i.try_to_usize().unwrap();
        markers[i] = true;
    }
}
