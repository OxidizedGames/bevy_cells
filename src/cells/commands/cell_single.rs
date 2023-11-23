use aery::edges::CheckedDespawn;
use bevy::ecs::{entity::Entity, system::Command, world::World};

use crate::prelude::CellMapLabel;

use super::{insert_cell, take_cell};

pub struct SpawnCell<L, const N: usize = 2> {
    pub cell_c: [isize; N],
    pub cell_id: Entity,
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for SpawnCell<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        insert_cell::<L, N>(world, self.cell_c, self.cell_id)
    }
}

pub struct DespawnCell<L, const N: usize> {
    pub cell_c: [isize; N],
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for DespawnCell<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        let cell_id = take_cell::<L, N>(world, self.cell_c);
        if let Some(id) = cell_id {
            CheckedDespawn(id).apply(world);
        }
    }
}

pub struct SwapCell<L, const N: usize> {
    pub cell_c_1: [isize; N],
    pub cell_c_2: [isize; N],
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for SwapCell<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        if self.cell_c_1 == self.cell_c_2 {
            return;
        }

        let cell_id_1 = take_cell::<L, N>(world, self.cell_c_1);

        let cell_id_2 = take_cell::<L, N>(world, self.cell_c_2);

        if let Some(cell_id) = cell_id_1 {
            SpawnCell::<L, N> {
                cell_c: self.cell_c_2,
                cell_id,
                label: self.label,
            }
            .apply(world);
        }

        if let Some(cell_id) = cell_id_2 {
            SpawnCell::<L, N> {
                cell_c: self.cell_c_1,
                cell_id,
                label: self.label,
            }
            .apply(world);
        }
    }
}

pub struct MoveCell<L, const N: usize> {
    pub old_c: [isize; N],
    pub new_c: [isize; N],
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for MoveCell<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        if self.old_c == self.new_c {
            return;
        }

        let old_cell_id = take_cell::<L, N>(world, self.old_c);

        if let Some(old_cell_id) = old_cell_id {
            SpawnCell::<L, N> {
                cell_c: self.new_c,
                cell_id: old_cell_id,
                label: self.label,
            }
            .apply(world);
        }
    }
}
