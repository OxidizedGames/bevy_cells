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
    prelude::{Bundle, Commands, Entity, With, World},
};

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

        // Get the chunk or insert it
        let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE).into();
        let mut chunk_e = if let Some(chunk_id) = map_e
            .get::<CellMap<L, N>>()
            .unwrap()
            .chunks
            .get(&chunk_c)
            .copied()
            .and_then(|chunk_e| {
                map_e
                    .world()
                    .get_entity(chunk_e)
                    .map(|chunk_e| chunk_e.id())
            }) {
            world.get_entity_mut(chunk_id).unwrap()
        } else {
            let mut chunk_id = None;
            let map_id = map_e.id();

            map_e.world_scope(|world| {
                chunk_id = Some(world.spawn(Chunk::new(L::CHUNK_SIZE.pow(N as u32))).id());
                Set::<InMap<L>>::new(chunk_id.unwrap(), map_id).apply(world);
            });

            let chunk_id = chunk_id.unwrap();
            let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE).into();

            map_e
                .get_mut::<CellMap<L, N>>()
                .unwrap()
                .chunks
                .insert(chunk_c, chunk_id);

            world.get_entity_mut(chunk_id).unwrap()
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
