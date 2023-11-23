use aery::edges::CheckedDespawn;
use bevy::ecs::{entity::Entity, system::Command, world::World};

use crate::prelude::CellMapLabel;

use super::{insert_chunk, take_chunk};

pub struct SpawnChunk<L, const N: usize = 2> {
    pub chunk_c: [isize; N],
    pub chunk_id: Entity,
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for SpawnChunk<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        insert_chunk::<L, N>(world, self.chunk_c, self.chunk_id)
    }
}

pub struct DespawnChunk<L, const N: usize> {
    pub chunk_c: [isize; N],
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for DespawnChunk<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        let cell_id = take_chunk::<L, N>(world, self.chunk_c);
        if let Some(id) = cell_id {
            CheckedDespawn(id).apply(world);
        }
    }
}
