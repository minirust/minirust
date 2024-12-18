use crate::*;

impl<'tcx> Ctxt<'tcx> {
    /// Gets the vtable name of for the type and trait or creates it if it doesn't exist yet.
    pub fn get_vtable(&mut self, ty: rs::Ty<'tcx>, trait_: TraitName) -> VTableName {
        if let Some((vtable_name, _)) = self.vtable_map.get(&(ty, trait_)) {
            *vtable_name
        } else {
            let fresh_name = VTableName(Name::from_internal(self.vtable_map.len() as _));
            let vtable = self.generate_vtable(ty, trait_);
            self.vtable_map.insert((ty, trait_), (fresh_name, vtable));
            fresh_name
        }
    }

    /// Generates a vtable for the given type and trait.
    fn generate_vtable(&mut self, ty: rs::Ty<'tcx>, trait_name: TraitName) -> VTable {
        // This is using linear search because we only have a one-way mapping.
        let (trait_, _) = self
            .trait_name_map
            .iter()
            .find(|(_, &n)| n == trait_name)
            .expect("TraitNames are generated when adding to this map");

        // Get the size and align
        let layout = self.rs_layout_of(ty);
        assert!(layout.is_sized(), "There are no unsized trait objects");
        let size = translate_size(layout.size);
        let align = translate_align(layout.align.abi);

        // Get the methods of the principal trait, create a method name wrapping the index in its vtable.
        let methods = if let Some(trait_) = trait_.principal() {
            let trait_ref = trait_.with_self_ty(self.tcx, ty);
            let trait_ref = self.tcx.erase_regions(trait_ref);
            let entries = self.tcx.vtable_entries(trait_ref);

            entries
                .iter()
                .enumerate()
                .filter_map(|(idx, entry)| self.vtable_entry_to_trait_method(idx as _, entry))
                .collect()
        } else {
            Map::new()
        };

        VTable { trait_name, size, align, methods }
    }

    // Generate an method implementation for a vtable entry if it is a method.
    fn vtable_entry_to_trait_method(
        &mut self,
        idx: u32,
        entry: &rs::VtblEntry<'tcx>,
    ) -> Option<(TraitMethodName, FnName)> {
        match entry {
            rs::VtblEntry::Method(func) =>
                Some((TraitMethodName(Name::from_internal(idx as _)), self.get_fn_name(*func))),
            _ => None,
        }
    }
}
