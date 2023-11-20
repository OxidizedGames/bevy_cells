use aery::edges::{CheckedDespawn, Set};
use bevy::ecs::{entity::Entity, system::Command, world::World};

use crate::prelude::{CellMap, CellMapLabel, Chunk, InMap};

use super::spawn_or_remove_map;

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
        let (map_id, mut map) = spawn_or_remove_map::<L, N>(world);

        // Despawn the chunk if it exists
        if let Some(chunk_id) = map.chunks.insert(self.chunk_c.into(), self.chunk_id) {
            CheckedDespawn(chunk_id).apply(world);
        }

        world
            .get_entity_mut(self.chunk_id)
            .unwrap()
            .insert(Chunk::new(L::CHUNK_SIZE.pow(N as u32)));
        Set::<InMap<L>>::new(self.chunk_id, map_id).apply(world);

        map.chunks.insert(self.chunk_c.into(), self.chunk_id);
        world.entity_mut(map_id).insert(map);
    }
}

pub struct DespawnChunk<L, const N: usize> {
    pub chunk_c: [isize; N],
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> DespawnChunk<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn take_entity(self, world: &mut World) -> Option<Entity> {
        // Get the map or return
        let mut map = world
            .query::<&mut CellMap<L, N>>()
            .get_single_mut(world)
            .ok()?;

        // Get the old chunk or return
        map.chunks.remove(&self.chunk_c.into())
    }
}

impl<L, const N: usize> Command for DespawnChunk<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        let cell_id = self.take_entity(world);
        if let Some(id) = cell_id {
            CheckedDespawn(id).apply(world);
        }
    }
}
