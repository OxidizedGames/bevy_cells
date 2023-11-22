use bevy::ecs::{bundle::Bundle, entity::Entity, system::Command, world::World};

use crate::prelude::CellMapLabel;

use super::{insert_chunk_batch, take_chunk_batch_despawn_cells};

pub struct SpawnChunkBatch<L, F, B, IC, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    F: Fn([isize; N]) -> B + Send + 'static,
    B: Bundle + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    pub chunk_cs: IC,
    pub bundle_f: F,
    pub label: std::marker::PhantomData<L>,
}

impl<L, F, B, IC, const N: usize> Command for SpawnChunkBatch<L, F, B, IC, N>
where
    L: CellMapLabel + Send + 'static,
    F: Fn([isize; N]) -> B + Send + 'static,
    B: Bundle + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    fn apply(self, world: &mut World) {
        let (chunk_cs, bundles): (Vec<[isize; N]>, Vec<B>) = self
            .chunk_cs
            .into_iter()
            .map(|coord| (coord, (self.bundle_f)(coord)))
            .unzip();

        let chunks = chunk_cs
            .into_iter()
            .zip(world.spawn_batch(bundles))
            .collect::<Vec<([isize; N], Entity)>>();

        insert_chunk_batch::<L, N>(world, chunks);
    }
}

pub struct DespawnChunkBatch<L, IC, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    pub chunk_cs: IC,
    pub label: std::marker::PhantomData<L>,
}

impl<L, IC, const N: usize> Command for DespawnChunkBatch<L, IC, N>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    fn apply(self, world: &mut World) {
        for (_, cell_id) in take_chunk_batch_despawn_cells::<L, N>(world, self.chunk_cs) {
            world.despawn(cell_id);
        }
    }
}
