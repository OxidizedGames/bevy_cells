use std::{any::TypeId, marker::PhantomData};

use aery::tuple_traits::RelationEntries;
use aery::{prelude::*, relation::RelationId};
use bevy::ecs::query::QueryIter;
use bevy::{
    ecs::{
        query::{ReadOnlyWorldQuery, WorldQuery},
        system::{ReadOnlySystemParam, SystemParam},
    },
    prelude::*,
};
use bimap::BiHashMap;
use std::collections::HashMap;

#[derive(Component)]
pub(crate) struct CellIndex(isize);

#[derive(Relation)]
#[aery(Recursive)]
pub(crate) struct InChunk<L>(std::marker::PhantomData<L>);

#[derive(Component)]
pub struct Chunk {
    pub(crate) cells: BiHashMap<usize, Entity>,
}

impl Chunk {
    pub(crate) fn new(chunk_size: usize) -> Self {
        Self {
            cells: BiHashMap::with_capacity(chunk_size),
        }
    }
}

#[derive(Relation)]
#[aery(Recursive)]
pub(crate) struct InMap<L>(std::marker::PhantomData<L>);

#[derive(Component, Default)]
pub struct CellMap {
    pub(crate) chunks: BiHashMap<ChunkCoord<2>, Entity>,
}

/// Used to query individual cells from a cell map.
/// This query also implicitly queries chunks and maps
/// in order to properly resolve cells.
#[derive(SystemParam)]
pub struct CellQuery<'w, 's, L, Q, F = ()>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    cell_q: Query<'w, 's, (Q, Relations<InChunk<L>>, Entity), F>,
    chunk_q: Query<'w, 's, (&'static Chunk, Relations<InMap<L>>)>,
    map_q: Query<'w, 's, &'static CellMap, With<MapLabel<L>>>,
}

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct ChunkCoord<const N: usize>([isize; N]);

impl<const N: usize> From<[isize; N]> for ChunkCoord<N> {
    fn from(value: [isize; N]) -> Self {
        Self(value)
    }
}

impl<'w, 's, L, Q, F> CellQuery<'w, 's, L, Q, F>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    pub fn get_single_with_coord(
        &self,
    ) -> Option<(
        [isize; 2],
        <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'_>,
    )> {
        let (val, edge, cell_id) = self.cell_q.get_single().ok()?;
        let chunk_id = edge.targets(RelationId::of::<InChunk<L>>())[0];
        let cell_i = self
            .chunk_q
            .get(chunk_id)
            .unwrap()
            .0
            .cells
            .get_by_right(&cell_id)
            .cloned()
            .unwrap();

        let map = self.map_q.get_single().ok()?;
        let chunk_coord = map.chunks.get_by_right(&chunk_id).unwrap();

        Some((
            calculate_cell_coordinate(chunk_coord.0, cell_i, L::CHUNK_SIZE),
            val,
        ))
    }

    pub fn iter_mut_with_coord(&mut self) -> CellQueryIter<'_, 's, L, Q, F> {
        CellQueryIter {
            cell_iter: self.cell_q.iter_mut(),
            chunk_q: &self.chunk_q,
            map_q: &self.map_q,
        }
    }

    pub fn get(
        &self,
        cell_c: [isize; 2],
    ) -> Option<<<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get_by_left(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.0.cells.get_by_left(&cell_index)?;

        self.cell_q.get(*cell_e).ok().map(|res| res.0)
    }

    pub fn get_mut(&mut self, cell_c: [isize; 2]) -> Option<<Q as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get_by_left(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.0.cells.get_by_left(&cell_index)?;

        self.cell_q.get_mut(*cell_e).ok().map(|res| res.0)
    }
}

pub(crate) fn calculate_chunk_coordinate<const N: usize>(
    cell_c: [isize; N],
    chunk_size: usize,
) -> [isize; N] {
    cell_c.map(|c| c / (chunk_size as isize) - if c < 0 { 1 } else { 0 })
}

pub(crate) fn calculate_chunk_relative_cell_coordinate<const N: usize>(
    mut cell_c: [isize; N],
    chunk_size: usize,
) -> [isize; N] {
    let chunk_c = calculate_chunk_coordinate(cell_c, chunk_size);
    for i in 0..N {
        cell_c[i] -= chunk_c[i] * chunk_size as isize;
    }
    cell_c
}

pub(crate) fn calculate_cell_index<const N: usize>(cell_c: [isize; N], chunk_size: usize) -> usize {
    let mut index = 0;
    let relative_cell_c = calculate_chunk_relative_cell_coordinate(cell_c, chunk_size);
    for (i, c) in relative_cell_c.iter().enumerate() {
        index += (*c as usize) * chunk_size.pow(i as u32);
    }
    index
}

pub(crate) fn calculate_cell_coordinate<const N: usize>(
    chunk_c: [isize; N],
    cell_i: usize,
    chunk_size: usize,
) -> [isize; N] {
    let mut chunk_world_c = chunk_c.map(|c| c * chunk_size as isize);
    for (i, c) in chunk_world_c.iter_mut().enumerate() {
        if i == 0 {
            *c += (cell_i % chunk_size) as isize;
        } else {
            *c += (cell_i / chunk_size.pow(i as u32)) as isize;
        }
    }
    chunk_world_c
}

pub trait CellMapLabel: Send + Sync {
    const CHUNK_SIZE: usize;
}

#[derive(Component)]
pub struct MapLabel<L>
where
    L: CellMapLabel + 'static,
{
    label: std::marker::PhantomData<L>,
}

impl<L> MapLabel<L>
where
    L: CellMapLabel + 'static,
{
    pub(crate) fn new() -> Self {
        Self { label: PhantomData }
    }
}

pub struct CellQueryIter<'w, 's, L, Q, F>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    cell_iter: QueryIter<'w, 's, (Q, Relations<InChunk<L>>, Entity), F>,
    chunk_q: &'w Query<'w, 's, (&'static Chunk, Relations<InMap<L>>)>,
    map_q: &'w Query<'w, 's, &'static CellMap, With<MapLabel<L>>>,
}

impl<'w, 's, L, Q, F> Iterator for CellQueryIter<'w, 's, L, Q, F>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Item = ([isize; 2], <Q as WorldQuery>::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        let (val, edge, cell_id) = self.cell_iter.next()?;
        let chunk_id = edge.targets(RelationId::of::<InChunk<L>>())[0];
        let cell_i = self
            .chunk_q
            .get(chunk_id)
            .unwrap()
            .0
            .cells
            .get_by_right(&cell_id)
            .cloned()
            .unwrap();

        let map = self.map_q.get_single().ok()?;
        let chunk_coord = map.chunks.get_by_right(&chunk_id).unwrap();

        Some((
            calculate_cell_coordinate(chunk_coord.0, cell_i, L::CHUNK_SIZE),
            val,
        ))
    }
}
