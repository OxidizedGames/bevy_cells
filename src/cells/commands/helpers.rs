use std::cell::Cell;

use aery::prelude::Set;
use bevy::{
    ecs::system::Command,
    prelude::{Entity, EntityMut, With, World},
};

use crate::{
    cells::{coords::calculate_chunk_coordinate, CellMap, Chunk, InMap},
    prelude::CellMapLabel,
};

/// Spawns a chunk, sets it's relations, inserts it into a map, then returns it.
pub(crate) fn spawn_and_insert_chunk_map_removed<'w, L, const N: usize>(
    chunk_c: [isize; N],
    map_id: Entity,
    map: &mut CellMap<L, N>,
    world: &'w mut World,
) -> EntityMut<'w>
where
    L: CellMapLabel + Send + 'static,
{
    let chunk_id = world.spawn(Chunk::new(L::CHUNK_SIZE.pow(N as u32))).id();
    Set::<InMap<L>>::new(chunk_id, map_id).apply(world);

    let [chunk_e] = world.get_many_entities_mut([chunk_id]).unwrap();
    map.chunks.insert(chunk_c.into(), chunk_id);

    chunk_e
}

/// Spawns a chunk in the world if needed, inserts the info into the map, and returns
/// and id for reinsertion
pub(crate) fn spawn_or_remove_chunk<L, const N: usize>(
    world: &mut World,
    map: &mut CellMap<L, N>,
    map_id: Entity,
    chunk_c: [isize; N],
) -> (Entity, Chunk)
where
    L: CellMapLabel + Send + 'static,
{
    if let Some(mut chunk_e) = map
        .chunks
        .get(&chunk_c.into())
        .cloned()
        .and_then(|chunk_id| world.get_entity_mut(chunk_id))
    {
        (chunk_e.id(), chunk_e.take::<Chunk>().unwrap())
    } else {
        let chunk_id = world.spawn_empty().id();
        map.chunks.insert(chunk_c.into(), chunk_id);
        Set::<InMap<L>>::new(chunk_id, map_id).apply(world);
        (chunk_id, Chunk::new(L::CHUNK_SIZE.pow(N as u32)))
    }
}

pub(crate) fn spawn_or_remove_map<L, const N: usize>(world: &mut World) -> (Entity, CellMap<L, N>)
where
    L: CellMapLabel + Send + 'static,
{
    // Get the map or insert it
    if let Ok(map_id) = world
        .query_filtered::<Entity, With<CellMap<L, N>>>()
        .get_single_mut(world)
    {
        (
            map_id,
            world
                .get_entity_mut(map_id)
                .unwrap()
                .take::<CellMap<L, N>>()
                .unwrap(),
        )
    } else {
        (world.spawn_empty().id(), CellMap::<L, N>::default())
    }
}
