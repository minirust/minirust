use crate::*;

impl<'tcx> Ctxt<'tcx> {
    /// Gets the vtable name for the given type and trait object or creates it if it doesn't exist yet.
    /// `trait_obj_ty` must be of kind [`rs::TyKind::Dynamic`].
    pub fn get_vtable(&mut self, ty: rs::Ty<'tcx>, trait_obj_ty: rs::Ty<'tcx>) -> VTableName {
        let rs::TyKind::Dynamic(trait_, _, _) = *trait_obj_ty.kind() else {
            panic!("get_vtable called on non trait object type");
        };
        if let Some(vtable_name) = self.vtable_map.get(&(ty, trait_)) {
            assert!(self.vtables.contains_key(*vtable_name));
            *vtable_name
        } else {
            let fresh_name = VTableName(Name::from_internal(self.vtable_map.len() as _));
            let vtable = self.generate_vtable(ty, trait_obj_ty);
            self.vtables.insert(fresh_name, vtable);
            self.vtable_map.insert((ty, trait_), fresh_name);
            fresh_name
        }
    }

    /// Generates a vtable for the given type and trait object.
    fn generate_vtable(&mut self, ty: rs::Ty<'tcx>, trait_obj_ty: rs::Ty<'tcx>) -> VTable {
        let rs::TyKind::Dynamic(trait_, _, _) = *trait_obj_ty.kind() else {
            panic!("generate_vtable called on non trait object type");
        };
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
        let trait_name = self.get_trait_name(trait_obj_ty);

        VTable { trait_name, size, align, methods }
    }

    /// Generate an method implementation for a vtable entry if it is a method.
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

    /// Returns TraitName for a given trait object. If it does not exist it creates a new one.
    /// `trait_obj_ty` must be of kind [`rs::TyKind::Dynamic`].
    pub fn get_trait_name(&mut self, trait_obj_ty: rs::Ty<'tcx>) -> TraitName {
        let rs::TyKind::Dynamic(trait_, _, _) = *trait_obj_ty.kind() else {
            panic!("get_trait_name called on non trait object type");
        };
        if let Some(trait_name) = self.trait_map.get(&trait_) {
            assert!(self.traits.contains_key(*trait_name));
            *trait_name
        } else {
            let fresh_name = TraitName(Name::from_internal(self.trait_map.len() as _));
            let methods = self.generate_trait(trait_obj_ty);
            self.traits.insert(fresh_name, methods);
            self.trait_map.insert(trait_, fresh_name);
            fresh_name
        }
    }

    /// Generates the set of method names for a given trait object type.
    fn generate_trait(&mut self, trait_obj_ty: rs::Ty<'tcx>) -> Set<TraitMethodName> {
        let rs::TyKind::Dynamic(trait_, _, _) = *trait_obj_ty.kind() else {
            panic!("generate_trait called on non trait object type");
        };

        let Some(princial_def_id) = trait_.principal_def_id() else {
            // no principal trait means no methods
            return Set::new();
        };

        let existential_trait_ref =
            self.tcx.instantiate_bound_regions_with_erased(trait_.principal().unwrap());
        // A trait ref with `dyn Trait` as the self type.
        let trait_ref = existential_trait_ref.with_self_ty(self.tcx, trait_obj_ty);
        let vtable_base = self.tcx.first_method_vtable_slot(trait_ref);

        // The method names are given by an base offset and the index.
        self.tcx
            .own_existential_vtable_entries(princial_def_id)
            .iter()
            .enumerate()
            .map(|(i, _)| TraitMethodName(Name::from_internal((vtable_base + i) as _)))
            .collect()
    }
}
