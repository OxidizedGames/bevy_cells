use std::{
    cmp::Eq,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use super::{CellMap, CellMapLabel, Chunk, InMap};
use aery::prelude::Set;
use bevy::{
    ecs::system::{Command, EntityCommands},
    log::info,
    prelude::{Bundle, Commands, Entity, With, World},
    utils::{hashbrown::hash_map::Entry, HashMap},
};

mod cell_batch;
mod cell_single;
mod chunk_batch;
mod chunk_single;
mod map;

use cell_batch::*;
use cell_single::*;
use chunk_batch::*;
use chunk_single::*;
use map::*;

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

    /// Despawns a cell.
    pub fn despawn_cell(&mut self, cell_c: [isize; N]) -> &mut Self {
        self.add(DespawnCell::<L, N> {
            cell_c,
            label: PhantomData,
        });
        self
    }

    /// Despawns cells from the given iterator.
    pub fn despawn_cell_batch<IC>(&mut self, cell_cs: IC)
    where
        IC: IntoIterator<Item = [isize; N]> + Send + 'static,
    {
        self.add(DespawnCellBatch::<L, IC, N> {
            cell_cs,
            label: std::marker::PhantomData,
        });
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

    /// Move cells from the first coordinate to the second coordinate, despawning
    /// any cell found in the second coordinate.
    pub fn move_cell_batch<IC>(&mut self, cell_cs: IC)
    where
        IC: IntoIterator<Item = ([isize; N], [isize; N])> + Send + 'static,
    {
        self.add(MoveCellBatch::<L, IC, N> {
            cell_cs,
            label: std::marker::PhantomData,
        });
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

    /// Swap cells from the first coordinate and the second coordinate
    pub fn swap_cell_batch<IC>(&mut self, cell_cs: IC)
    where
        IC: IntoIterator<Item = ([isize; N], [isize; N])> + Send + 'static,
    {
        self.add(SwapCellBatch::<L, IC, N> {
            cell_cs,
            label: std::marker::PhantomData,
        });
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

    /// Recursively despawns a map and all it's chunks and cells.
    pub fn despawn_map(&mut self) -> &mut Self {
        self.add(DespawnMap::<L, N> { label: PhantomData });
        self
    }
}

/// Spawns a chunk in the world if needed, inserts the info into the map, and returns
/// and id for reinsertion
#[inline]
fn spawn_or_remove_chunk<L, const N: usize>(
    world: &mut World,
    map: &mut CellMap<L, N>,
    map_id: Entity,
    chunk_c: [isize; N],
) -> (Entity, Chunk)
where
    L: CellMapLabel + Send + 'static,
{
    if let Some(chunk_info) = remove_chunk::<L, N>(world, map, chunk_c) {
        info!("Chunk found!");
        chunk_info
    } else {
        info!("Chunk spawned!");
        let chunk_id = world.spawn_empty().id();
        map.chunks.insert(chunk_c.into(), chunk_id);
        Set::<InMap<L>>::new(chunk_id, map_id).apply(world);
        (chunk_id, Chunk::new(L::CHUNK_SIZE.pow(N as u32)))
    }
}

/// Removes a chunk from the world if it exists, and returns the info to reinsert it.
#[inline]
fn remove_chunk<L, const N: usize>(
    world: &mut World,
    map: &mut CellMap<L, N>,
    chunk_c: [isize; N],
) -> Option<(Entity, Chunk)>
where
    L: CellMapLabel + Send + 'static,
{
    info!("Removing Chunk: {:?}!", chunk_c);
    map.chunks
        .get(&chunk_c.into())
        .cloned()
        .and_then(|chunk_id| world.get_entity_mut(chunk_id))
        .map(|mut chunk_e| (chunk_e.id(), chunk_e.take::<Chunk>().unwrap()))
}

/// Takes the map out of the world or spawns a new one and returns the entity id to return the map to.
#[inline]
fn spawn_or_remove_map<L, const N: usize>(world: &mut World) -> (Entity, CellMap<L, N>)
where
    L: CellMapLabel + Send + 'static,
{
    let map_info = remove_map::<L, N>(world);
    if let Some(map_info) = map_info {
        map_info
    } else {
        (world.spawn_empty().id(), CellMap::<L, N>::default())
    }
}

/// Takes the map out of the world if it exists.
#[inline]
fn remove_map<L, const N: usize>(world: &mut World) -> Option<(Entity, CellMap<L, N>)>
where
    L: CellMapLabel + Send + 'static,
{
    world
        .query_filtered::<Entity, With<CellMap<L, N>>>()
        .get_single_mut(world)
        .ok()
        .map(|map_id| {
            (
                map_id,
                world
                    .get_entity_mut(map_id)
                    .unwrap()
                    .take::<CellMap<L, N>>()
                    .unwrap(),
            )
        })
}

trait GroupBy: Iterator {
    fn group_by<F, K>(
        self,
        f: F,
    ) -> bevy::utils::hashbrown::hash_map::IntoIter<
        K,
        std::vec::Vec<<Self as std::iter::Iterator>::Item>,
    >
    where
        F: Fn(&Self::Item) -> K,
        K: Eq + Hash;
}

impl<T> GroupBy for T
where
    T: Iterator,
{
    fn group_by<F, K>(
        self,
        f: F,
    ) -> bevy::utils::hashbrown::hash_map::IntoIter<
        K,
        std::vec::Vec<<T as std::iter::Iterator>::Item>,
    >
    where
        F: Fn(&Self::Item) -> K,
        K: Eq + Hash,
    {
        let mut map = HashMap::new();
        for item in self {
            let key = f(&item);
            match map.entry(key) {
                Entry::Vacant(v) => {
                    v.insert(vec![item]);
                }
                Entry::Occupied(mut o) => o.get_mut().push(item),
            }
        }
        map.into_iter()
    }
}
