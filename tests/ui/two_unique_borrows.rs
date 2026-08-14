use themis::{sync_region_placement_scope, ConstNumaPinnedCellRef};

fn main() {
    sync_region_placement_scope(|region| {
        let mut cell = region.cell(0u32);
        let (mut p0, mut p1) = region.split_static::<0, 1>();
        let r0 = ConstNumaPinnedCellRef::<0, _>::from_unique(&mut cell);
        let r1 = ConstNumaPinnedCellRef::<1, _>::from_unique(&mut cell);
        *p0.write(&r0) = 1;
        *p1.write(&r1) = 2;
    });
}
