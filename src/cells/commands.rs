use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use super::{CellCoord, CellIndex, CellMap, CellMapLabel, Chunk, InChunk, InMap};
use crate::cells::coords::*;
use aery::{
    edges::CheckedDespawn,
    prelude::{Set, Unset},
};
use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::{Bundle, Commands, Entity, EntityMut, With, World},
    utils::{HashMap, HashSet},
};
use helpers::*;

// REUSE THINGS BY MAKING HELPERS WHEN YOU CAN
// BUT PLEASE UNDERSTAND THESE COMMANDS FLY IN THE FACE
// OF NLL CONDITION #3 AND ARE A PITA TO MAKE HELPERS FOR
mod helpers;

/// Applies commands to a specific cell map.
pub struct CellCommands<'a, 'w, 's, L, const N: usize> {
    commands: &'a mut Commands<'w, 's>,
    phantom: PhantomData<L>,
}

impl<'a, 'w, 's, L, const N: usize> Deref for CellCommands<'a, 'w, 's, L, N>
where
    L: CellMapLabel + 'static,
{
    type Target = Commands<'w, 's>;

    fn deref(&self) -> &Self::Target {
        self.commands
    }
}

impl<'a, 'w, 's, L, const N: usize> DerefMut for CellCommands<'a, 'w, 's, L, N>
where
    L: CellMapLabel + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.commands
    }
}

pub trait CellCommandExt<'w, 's> {
    /// Gets the [CellCommands] to apply commands at the cell map level.
    fn cells<'a, L, const N: usize>(&'a mut self) -> CellCommands<'a, 'w, 's, L, N>
    where
        L: CellMapLabel + 'static;
}

impl<'w, 's> CellCommandExt<'w, 's> for Commands<'w, 's> {
    fn cells<L, const N: usize>(&mut self) -> CellCommands<'_, 'w, 's, L, N>
    where
        L: CellMapLabel + 'static,
    {
        CellCommands {
            commands: self,
            phantom: PhantomData,
        }
    }
}

impl<'a, 'w, 's, L, const N: usize> CellCommands<'a, 'w, 's, L, N>
where
    L: CellMapLabel + 'static,
{
    /// Spawns a cell and returns a handle to the underlying entity.
    /// This will despawn any cell that already exists in this coordinate
    pub fn spawn_cell<T>(&mut self, cell_c: [isize; N], bundle: T) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
    {
        let cell_id = self.spawn(bundle).id();
        self.add(SpawnCell::<L, N> {
            cell_c,
            cell_id,
            label: std::marker::PhantomData,
        });
        self.entity(cell_id)
    }

    /// Spawns a cell and returns a handle to the underlying entity.
    /// This will despawn any cell that already exists in this coordinate
    pub fn spawn_cell_batch_with<F, B, IC>(&mut self, cell_c: IC, bundle_f: F)
    where
        F: Fn([isize; N]) -> B + Send + 'static,
        B: Bundle + Send + 'static,
        IC: IntoIterator<Item = [isize; N]>,
    {
        let cell_cs = cell_c.into_iter().collect();
        self.add(SpawnCellBatch::<L, F, B, N> {
            cell_cs,
            bundle_f,
            label: std::marker::PhantomData,
        });
    }

    /// Recursively despawns a map and all it's chunks and cells.
    pub fn despawn_map(&mut self) -> &mut Self {
        self.add(DespawnMap::<L, N> { label: PhantomData });
        self
    }

    /// Despawns a cell.
    pub fn despawn_cell(&mut self, cell_c: [isize; N]) -> &mut Self {
        self.add(DespawnCell::<L, N> {
            cell_c,
            label: PhantomData,
        });
        self
    }

    /// Moves a cell from one coordinate to another, overwriting and despawning any cell in the new coordinate.
    pub fn move_cell(&mut self, old_c: [isize; N], new_c: [isize; N]) -> &mut Self {
        self.add(MoveCell::<L, N> {
            old_c,
            new_c,
            label: PhantomData,
        });
        self
    }

    /// Manually spawn a chunk entity, note that this will overwrite and despawn existing chunks at this location.
    pub fn spawn_chunk<T>(&mut self, chunk_c: [isize; N], bundle: T) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
    {
        let chunk_id = self.spawn(bundle).id();
        self.add(SpawnChunk::<L, N> {
            chunk_c,
            chunk_id,
            label: std::marker::PhantomData,
        });
        self.entity(chunk_id)
    }

    /// Recursively despawn a chunk and all it's cells.
    pub fn despawn_chunk(&mut self, chunk_c: [isize; N]) -> &mut Self {
        self.add(DespawnChunk::<L, N> {
            chunk_c,
            label: std::marker::PhantomData,
        });
        self
    }

    /// Swaps two cells if both exist, or just moves one cell if the other doesn't exist.
    pub fn swap_cells(&mut self, cell_c_1: [isize; N], cell_c_2: [isize; N]) -> &mut Self {
        self.add(SwapCell::<L, N> {
            cell_c_1,
            cell_c_2,
            label: PhantomData,
        });
        self
    }
}

pub struct DespawnMap<L, const N: usize = 2> {
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for DespawnMap<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        if let Ok(map_id) = world
            .query_filtered::<Entity, With<CellMap<L, N>>>()
            .get_single(world)
        {
            CheckedDespawn(map_id).apply(world);
        }
    }
}

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
        // Get the map or insert it
        let map_e = if let Some(map) = world
            .query_filtered::<Entity, With<CellMap<L, N>>>()
            .get_single_mut(world)
            .ok()
            .and_then(|map_id| world.get_entity(map_id))
        {
            map
        } else {
            let map_id = world.spawn(CellMap::<L, N>::default()).id();
            world.get_entity(map_id).unwrap()
        };

        // Get the chunk or insert it
        let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE).into();
        let map_id = map_e.id();
        let mut chunk_e = if let Some([chunk_e]) = map_e
            .get::<CellMap<L, N>>()
            .unwrap()
            .chunks
            .get(&chunk_c)
            .cloned()
            .and_then(|chunk_id| world.get_many_entities_mut([chunk_id]).ok())
        {
            chunk_e
        } else {
            spawn_and_insert_chunk::<L, N>(chunk_c.0, map_id, world)
        };

        // Insert the tile
        let cell_i = calculate_cell_index(self.cell_c, L::CHUNK_SIZE);
        let chunk_id = chunk_e.id();
        let mut chunk = chunk_e.get_mut::<Chunk>().unwrap();

        if let Some(cell) = chunk.cells.get_mut(cell_i) {
            if let Some(old_cell_id) = cell.replace(self.cell_id) {
                world.despawn(old_cell_id);
            }
        }

        Set::<InChunk<L>>::new(self.cell_id, chunk_id).apply(world);

        world
            .get_entity_mut(self.cell_id)
            .unwrap()
            .insert((CellIndex::from(cell_i), CellCoord::<N>::from(self.cell_c)));
    }
}

pub struct SpawnCellBatch<L, F, B, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    F: Fn([isize; N]) -> B + Send + 'static,
    B: Bundle + Send + 'static,
{
    pub cell_cs: Vec<[isize; N]>,
    pub bundle_f: F,
    pub label: std::marker::PhantomData<L>,
}

impl<L, F, B, const N: usize> Command for SpawnCellBatch<L, F, B, N>
where
    L: CellMapLabel + Send + 'static,
    F: Fn([isize; N]) -> B + Send + 'static,
    B: Bundle + Send + 'static,
{
    fn apply(mut self, world: &mut World) {
        // Group cells by chunk
        let mut cells = HashMap::new();
        for (cell_id, cell_c) in world
            .spawn_batch(self.cell_cs.iter().map(|coord| (self.bundle_f)(*coord)))
            .zip(self.cell_cs.iter())
        {
            cells.insert(cell_id, *cell_c);
        }

        let mut chunked_cells = HashMap::new();
        for (chunk_coord, coord, ent) in cells
            .drain()
            .map(|(ent, coord)| (calculate_chunk_coordinate(coord, L::CHUNK_SIZE), coord, ent))
        {
            match chunked_cells.entry(chunk_coord) {
                bevy::utils::hashbrown::hash_map::Entry::Vacant(v) => {
                    v.insert(vec![(coord, ent)]);
                }
                bevy::utils::hashbrown::hash_map::Entry::Occupied(mut o) => {
                    o.get_mut().push((coord, ent));
                }
            };
        }

        // Get the map or insert it
        let map_e = if let Ok(map_id) = world
            .query_filtered::<Entity, With<CellMap<L, N>>>()
            .get_single_mut(world)
        {
            match world.get_entity(map_id) {
                Some(map) => map,
                None => {
                    let map_id = world.spawn(CellMap::<L, N>::default()).id();
                    world.get_entity(map_id).unwrap()
                }
            }
        } else {
            let map_id = world.spawn(CellMap::<L, N>::default()).id();
            world.get_entity(map_id).unwrap()
        };

        let map_id = map_e.id();

        // Get the chunks and entities from the map
        let mut missing_chunks = Vec::new();
        let mut map_slice = HashMap::new();
        for (chunk_c, _) in chunked_cells.iter() {
            if let Some(chunk_id) = map_e
                .get::<CellMap<L, N>>()
                .unwrap()
                .chunks
                .get(&(*chunk_c).into())
            {
                match world.get_entity(*chunk_id) {
                    Some(entity) => {
                        map_slice.insert(*chunk_c, entity.id());
                    }
                    None => missing_chunks.push(*chunk_c),
                }
            } else {
                missing_chunks.push(*chunk_c);
            };
        }

        // Insert missing chunks
        for chunk_c in missing_chunks.drain(..) {
            map_slice.insert(
                chunk_c,
                spawn_and_insert_chunk::<L, N>(chunk_c, map_id, world).id(),
            );
        }

        for (chunk_c, cells) in chunked_cells.drain() {
            let chunk_id = *map_slice.get(&chunk_c).unwrap();

            let mut chunk_e = world.get_entity_mut(chunk_id).unwrap();

            let mut chunk = chunk_e.get_mut::<Chunk>().unwrap();

            let mut despawn_ids = Vec::new();
            let mut cells_with_index = Vec::with_capacity(cells.len());
            for (cell_c, cell_id) in cells {
                // Insert the tile
                let cell_i = calculate_cell_index(cell_c, L::CHUNK_SIZE);

                if let Some(cell) = chunk.cells.get_mut(cell_i) {
                    if let Some(old_cell_id) = cell.replace(cell_id) {
                        despawn_ids.push(old_cell_id);
                    }
                }

                cells_with_index.push((cell_c, cell_id, cell_i));
            }

            for (cell_c, cell_id, cell_i) in cells_with_index {
                Set::<InChunk<L>>::new(cell_id, chunk_id).apply(world);

                world
                    .get_entity_mut(cell_id)
                    .unwrap()
                    .insert((CellIndex::from(cell_i), CellCoord::<N>::from(cell_c)));
            }
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
        let map_e = world
            .query_filtered::<Entity, With<CellMap<L, N>>>()
            .get_single_mut(world)
            .ok()
            .and_then(|map_id| world.get_entity_mut(map_id))?;

        // Get the old chunk or return
        let old_chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE).into();
        let mut old_chunk_e = if let Some(chunk_id) = map_e
            .get::<CellMap<L, N>>()
            .unwrap()
            .chunks
            .get(&old_chunk_c)
            .copied()
            .and_then(|chunk_e| {
                map_e
                    .world()
                    .get_entity(chunk_e)
                    .map(|chunk_e| chunk_e.id())
            }) {
            world.get_entity_mut(chunk_id).unwrap()
        } else {
            return None;
        };

        // Remove the old entity or return if the old entity is already deleted
        let mut old_chunk = old_chunk_e.get_mut::<Chunk>().unwrap();
        let old_cell_i = calculate_cell_index(self.cell_c, L::CHUNK_SIZE);
        let old_cell_id = if let Some(Some(cell_id)) =
            old_chunk.cells.get_mut(old_cell_i).map(|cell| cell.take())
        {
            cell_id
        } else {
            return None;
        };

        let old_chunk_id = old_chunk_e.id();

        world.get_entity(old_cell_id)?;

        // Remove the old relation
        Unset::<InChunk<L>>::new(old_cell_id, old_chunk_id).apply(world);

        Some(old_cell_id)
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
        // Get the map or insert it
        let mut map_e = if let Some(map) = world
            .query_filtered::<Entity, With<CellMap<L, N>>>()
            .get_single_mut(world)
            .ok()
            .and_then(|map_id| world.get_entity_mut(map_id))
        {
            map
        } else {
            world.spawn(CellMap::<L, N>::default())
        };

        // Despawn the chunk if it exists
        if let Some(chunk_id) = map_e
            .get::<CellMap<L, N>>()
            .unwrap()
            .chunks
            .get(&self.chunk_c.into())
            .copied()
            .and_then(|chunk_e| {
                map_e
                    .world()
                    .get_entity(chunk_e)
                    .map(|chunk_e| chunk_e.id())
            })
        {
            map_e.world_scope(|world| CheckedDespawn(chunk_id).apply(world));
        }

        let map_id = map_e.id();

        map_e.world_scope(|world| {
            world
                .get_entity_mut(self.chunk_id)
                .unwrap()
                .insert(Chunk::new(L::CHUNK_SIZE.pow(N as u32)));
            Set::<InMap<L>>::new(self.chunk_id, map_id).apply(world);
        });

        let mut map = map_e.get_mut::<CellMap<L, N>>().unwrap();
        map.chunks.insert(self.chunk_c.into(), self.chunk_id);
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
        let map_e = world
            .query_filtered::<Entity, With<CellMap<L, N>>>()
            .get_single_mut(world)
            .ok()
            .and_then(|map_id| world.get_entity_mut(map_id))?;

        // Get the old chunk or return
        map_e
            .get::<CellMap<L, N>>()
            .unwrap()
            .chunks
            .get(&self.chunk_c.into())
            .cloned()
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
