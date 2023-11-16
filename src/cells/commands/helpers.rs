use aery::prelude::Set;
use bevy::{
    ecs::system::Command,
    prelude::{Entity, EntityMut, World},
};

use crate::{
    cells::{CellMap, Chunk, InMap},
    prelude::CellMapLabel,
};

/// Spawns a chunk, sets it's relations, inserts it into a map, then returns it.
pub fn spawn_and_insert_chunk<L, const N: usize>(
    chunk_c: [isize; N],
    map_id: Entity,
    world: &mut World,
) -> EntityMut<'_>
where
    L: CellMapLabel + Send + 'static,
{
    let chunk_id = world.spawn(Chunk::new(L::CHUNK_SIZE.pow(N as u32))).id();
    Set::<InMap<L>>::new(chunk_id, map_id).apply(world);

    let [mut map_e, chunk_e] = world.get_many_entities_mut([map_id, chunk_id]).unwrap();
    map_e
        .get_mut::<CellMap<L, N>>()
        .unwrap()
        .chunks
        .insert(chunk_c.into(), chunk_id);

    chunk_e
}
