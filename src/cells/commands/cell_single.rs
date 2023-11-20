use aery::edges::{CheckedDespawn, Set, Unset};
use bevy::ecs::{entity::Entity, system::Command, world::World};

use crate::prelude::{
    calculate_cell_index, calculate_chunk_coordinate, CellCoord, CellIndex, CellMap, CellMapLabel,
    Chunk, InChunk,
};

use super::{spawn_or_remove_chunk, spawn_or_remove_map};

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
        // Take the map out and get the id to reinsert it
        let (map_id, mut map) = spawn_or_remove_map::<L, N>(world);

        // Take the chunk out and get the id to reinsert it
        let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE);
        let (chunk_id, mut chunk) = spawn_or_remove_chunk::<L, N>(world, &mut map, map_id, chunk_c);

        // Insert the tile
        let cell_i = calculate_cell_index(self.cell_c, L::CHUNK_SIZE);

        if let Some(cell) = chunk.cells.get_mut(cell_i) {
            if let Some(old_cell_id) = cell.replace(self.cell_id) {
                world.despawn(old_cell_id);
            }
        }

        Set::<InChunk<L>>::new(self.cell_id, chunk_id).apply(world);

        world
            .get_entity_mut(self.cell_id)
            .unwrap()
            .insert((CellIndex::from(cell_i), CellCoord::<N>::new(self.cell_c)));

        world.get_entity_mut(chunk_id).unwrap().insert(chunk);
        world.get_entity_mut(map_id).unwrap().insert(map);
    }
}

pub struct DespawnCell<L, const N: usize> {
    pub cell_c: [isize; N],
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> DespawnCell<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn take_entity(self, world: &mut World) -> Option<Entity> {
        // Get the map or return
        let map = world.query::<&CellMap<L, N>>().get_single(world).ok()?;

        // Get the old chunk or return
        let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE);
        let chunk_id = *map.chunks.get(&chunk_c.into())?;
        let mut chunk = world.query::<&mut Chunk>().get_mut(world, chunk_id).ok()?;

        // Remove the old entity or return if the old entity is already deleted
        let cell_i = calculate_cell_index(self.cell_c, L::CHUNK_SIZE);
        if let Some(cell_id) = chunk.cells.get_mut(cell_i).and_then(|cell| cell.take()) {
            Unset::<InChunk<L>>::new(cell_id, chunk_id).apply(world);
            Some(cell_id)
        } else {
            None
        }
    }
}

impl<L, const N: usize> Command for DespawnCell<L, N>
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

        let cell_id_1 = DespawnCell::<L, N> {
            cell_c: self.cell_c_1,
            label: self.label,
        }
        .take_entity(world);

        let cell_id_2 = DespawnCell::<L, N> {
            cell_c: self.cell_c_2,
            label: self.label,
        }
        .take_entity(world);

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

        let old_cell_id = DespawnCell::<L, N> {
            cell_c: self.old_c,
            label: self.label,
        }
        .take_entity(world);

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
