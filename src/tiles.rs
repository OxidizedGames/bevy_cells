use aery::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;
use std::ops::Deref;

pub mod cellquery;
pub mod commands;

// ===============
// Cell Components
// ===============

#[derive(Component)]
pub(crate) struct CellIndex(usize);

impl From<usize> for CellIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl Deref for CellIndex {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component)]
pub(crate) struct CellCoord<const N: usize = 2>([isize; N]);

impl<const N: usize> From<[isize; N]> for CellCoord<N> {
    fn from(value: [isize; N]) -> Self {
        Self(value)
    }
}

impl<const N: usize> Deref for CellCoord<N> {
    type Target = [isize; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Relation)]
#[aery(Recursive)]
pub struct InChunk<L>(std::marker::PhantomData<L>);

// ================
// Chunk Components
// ================

#[derive(Component, PartialEq, Eq, Hash)]
pub(crate) struct ChunkCoord<const N: usize = 2>([isize; N]);

impl<const N: usize> From<[isize; N]> for ChunkCoord<N> {
    fn from(value: [isize; N]) -> Self {
        Self(value)
    }
}

impl<const N: usize> Deref for ChunkCoord<N> {
    type Target = [isize; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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

// ==============
// Map Components
// ==============

/// Adds type level info on how a Cell Map should be treated.
pub trait CellMapLabel: Send + Sync {
    /// How many cells per dimension a chunk in this map extends.
    const CHUNK_SIZE: usize;
}

#[derive(Component)]
pub struct CellMap<L, const N: usize = 2>
where
    L: CellMapLabel + 'static,
{
    pub(crate) chunks: HashMap<ChunkCoord<N>, Entity>,
    label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Default for CellMap<L, N>
where
    L: CellMapLabel + 'static,
{
    fn default() -> Self {
        Self {
            chunks: Default::default(),
            label: Default::default(),
        }
    }
}

// ================
// Helper Functions
// ================

pub fn calculate_chunk_coordinate<const N: usize>(
    cell_c: [isize; N],
    chunk_size: usize,
) -> [isize; N] {
    cell_c.map(|c| c / (chunk_size as isize) - if c < 0 { 1 } else { 0 })
}

pub fn calculate_chunk_relative_cell_coordinate<const N: usize>(
    mut cell_c: [isize; N],
    chunk_size: usize,
) -> [isize; N] {
    let chunk_c = calculate_chunk_coordinate(cell_c, chunk_size);
    for i in 0..N {
        cell_c[i] -= chunk_c[i] * chunk_size as isize;
    }
    cell_c
}

pub fn calculate_cell_index<const N: usize>(cell_c: [isize; N], chunk_size: usize) -> usize {
    let mut index = 0;
    let relative_cell_c = calculate_chunk_relative_cell_coordinate(cell_c, chunk_size);
    for (i, c) in relative_cell_c.iter().enumerate() {
        index += (*c as usize) * chunk_size.pow(i as u32);
    }
    index
}

pub fn calculate_cell_coordinate<const N: usize>(
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
