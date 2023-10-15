use std::{any::TypeId, marker::PhantomData};

use aery::prelude::*;
use bevy::{
    ecs::{
        query::{ReadOnlyWorldQuery, WorldQuery},
        system::{ReadOnlySystemParam, SystemParam},
    },
    prelude::*,
};
use std::collections::HashMap;

#[derive(Relation)]
#[aery(Recursive)]
pub(crate) struct InChunk<L>(std::marker::PhantomData<L>);

#[derive(Component)]
pub struct Chunk {
    pub(crate) cells: Vec<Option<Entity>>,
}

impl Chunk {
    pub(crate) fn new(chunk_size: usize) -> Self {
        Self {
            cells: vec![None; chunk_size],
        }
    }
}

#[derive(Relation)]
#[aery(Recursive)]
pub(crate) struct InMap<L>(std::marker::PhantomData<L>);

#[derive(Component, Default)]
pub struct CellMap {
    pub(crate) chunks: HashMap<CellCoord<2>, Entity>,
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
    cell_q: Query<'w, 's, Q, (F, Relations<InChunk<L>>)>,
    chunk_q: Query<'w, 's, &'static Chunk, Relations<InMap<L>>>,
    map_q: Query<'w, 's, &'static CellMap, With<MapLabel<L>>>,
}

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct CellCoord<const N: usize>([isize; N]);

impl<const N: usize> From<[isize; N]> for CellCoord<N> {
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
    pub fn get(
        &self,
        cell_c: [isize; 2],
    ) -> Option<<<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get(*cell_e).ok()
    }

    pub fn get_mut(&mut self, cell_c: [isize; 2]) -> Option<<Q as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get_mut(*cell_e).ok()
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
