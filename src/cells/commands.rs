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
    prelude::{info, Bundle, Commands, Entity, With, World},
    utils::HashMap,
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

    /// Spawns cells from the given iterator using the given function.
    /// This will despawn any cell that already exists in this coordinate
    pub fn spawn_cell_batch_with<F, B, IC>(&mut self, cell_cs: IC, bundle_f: F)
    where
        F: Fn([isize; N]) -> B + Send + 'static,
        B: Bundle + Send + 'static,
        IC: IntoIterator<Item = [isize; N]> + Send + 'static,
    {
        self.add(SpawnCellBatch::<L, F, B, IC, N> {
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

pub struct SpawnCellBatch<L, F, B, IC, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    F: Fn([isize; N]) -> B + Send + 'static,
    B: Bundle + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    pub cell_cs: IC,
    pub bundle_f: F,
    pub label: std::marker::PhantomData<L>,
}

impl<L, F, B, IC, const N: usize> Command for SpawnCellBatch<L, F, B, IC, N>
where
    L: CellMapLabel + Send + 'static,
    F: Fn([isize; N]) -> B + Send + 'static,
    B: Bundle + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    fn apply(self, world: &mut World) {
        let (cell_cs, bundles): (Vec<[isize; N]>, Vec<B>) = self
            .cell_cs
            .into_iter()
            .map(|coord| (coord, (self.bundle_f)(coord)))
            .unzip();

        // Group cells by chunk
        let mut cells = HashMap::new();
        for (cell_id, cell_c) in world.spawn_batch(bundles).zip(cell_cs) {
            cells.insert(cell_id, cell_c);
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

        // Remove the map, or spawn an entity to hold the map, then create an empty map
        let (map_id, mut map) = spawn_or_remove_map::<L, N>(world);

        // Get the chunks and entities from the map
        let cells_with_chunk = Vec::from_iter(chunked_cells.drain().map(|(chunk_c, cells)| {
            let (chunk_id, chunk) = spawn_or_remove_chunk(world, &mut map, map_id, chunk_c);
            (chunk_id, chunk, cells)
        }));

        for (chunk_id, mut chunk, cells) in cells_with_chunk {
            for (cell_c, cell_id) in cells {
                let cell_i = calculate_cell_index(cell_c, L::CHUNK_SIZE);

                if let Some(cell) = chunk.cells.get_mut(cell_i) {
                    if let Some(old_cell_id) = cell.replace(cell_id) {
                        world.despawn(old_cell_id);
                    }
                }

                Set::<InChunk<L>>::new(cell_id, chunk_id).apply(world);

                world
                    .get_entity_mut(cell_id)
                    .unwrap()
                    .insert((CellIndex::from(cell_i), CellCoord::<N>::new(cell_c)));
            }

            world.get_entity_mut(chunk_id).unwrap().insert(chunk);
        }

        world.get_entity_mut(map_id).unwrap().insert(map);
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
        let map = world.query::<&CellMap<L, N>>().get_single(world).ok()?;

        // Get the old chunk or return
        let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE);
        let chunk_id = *map.chunks.get(&chunk_c.into())?;
        let mut chunk = world.query::<&mut Chunk>().get_mut(world, chunk_id).ok()?;

        // Remove the old entity or return if the old entity is already deleted
        let cell_i = calculate_cell_index(self.cell_c, L::CHUNK_SIZE);
        chunk.cells.get_mut(cell_i).and_then(|cell| cell.take())
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
