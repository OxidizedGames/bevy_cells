use aery::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;
use std::ops::Deref;

pub mod cell_query;
pub mod chunk_query;
pub mod commands;
pub mod coords;

// ===============
// Cell Components
// ===============

#[derive(Component, Debug)]
pub struct CellIndex(usize);

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

#[derive(Component, Debug)]
pub struct CellCoord<const N: usize = 2>([isize; N]);

impl<const N: usize> CellCoord<N> {
    pub(crate) fn new(value: [isize; N]) -> Self {
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

#[derive(Component, Debug, PartialEq, Eq, Hash)]
pub struct ChunkCoord<const N: usize = 2>([isize; N]);

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
pub struct InMap<L>(std::marker::PhantomData<L>);

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
