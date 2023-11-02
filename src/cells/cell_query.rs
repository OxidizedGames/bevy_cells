use std::ops::{Deref, DerefMut};

use aery::prelude::*;
use bevy::{
    ecs::{
        query::{ReadOnlyWorldQuery, WorldQuery},
        system::SystemParam,
    },
    prelude::Query,
};

use super::{CellMap, CellMapLabel, Chunk, InChunk, InMap};
use crate::cells::coords::*;

/// Used to query individual cells from a cell map.
/// This query also implicitly queries chunks and maps
/// in order to properly resolve cells.
#[derive(SystemParam)]
pub struct CellQuery<'w, 's, L, Q, F = (), const N: usize = 2>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    cell_q: Query<'w, 's, Q, (F, Relations<InChunk<L>>)>,
    chunk_q: Query<'w, 's, &'static Chunk, Relations<InMap<L>>>,
    map_q: Query<'w, 's, &'static CellMap<L, N>>,
}

impl<'w, 's, L, Q, F, const N: usize> Deref for CellQuery<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Target = Query<'w, 's, Q, (F, Relations<InChunk<L>>)>;

    fn deref(&self) -> &Self::Target {
        &self.cell_q
    }
}

impl<'w, 's, L, Q, F, const N: usize> DerefMut for CellQuery<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell_q
    }
}

impl<'w, 's, L, Q, F, const N: usize> CellQuery<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    /// Get's the readonly query item for the given cell.
    pub fn get_at(
        &self,
        cell_c: [isize; N],
    ) -> Option<<<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get(*cell_e).ok()
    }

    /// Get's the query item for the given cell.
    pub fn get_at_mut(&mut self, cell_c: [isize; N]) -> Option<<Q as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get_mut(*cell_e).ok()
    }

    /// Get's the query item for the given cell.
    /// # Safety
    /// This function makes it possible to violate Rust's aliasing guarantees: please use responsibly.
    pub unsafe fn get_at_unchecked(
        &self,
        cell_c: [isize; N],
    ) -> Option<<Q as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get_unchecked(*cell_e).ok()
    }

    /// Iterate over all the cells in a given space, starting at `corner_1`
    /// inclusive over `corner_2`
    pub fn iter_in(
        &self,
        corner_1: [isize; N],
        corner_2: [isize; N],
    ) -> CellQueryIter<'_, 's, L, Q, F, N> {
        unsafe { CellQueryIter::new(self, corner_1, corner_2) }
    }

    /// Iterate over all the cells in a given space, starting at `corner_1`
    /// inclusive over `corner_2`
    pub fn iter_in_mut(
        &mut self,
        corner_1: [isize; N],
        corner_2: [isize; N],
    ) -> CellQueryIterMut<'_, 's, L, Q, F, N> {
        unsafe { CellQueryIterMut::new(self, corner_1, corner_2) }
    }

    pub fn to_readonly(
        &self,
    ) -> CellQuery<'_, 's, L, <Q as WorldQuery>::ReadOnly, <F as WorldQuery>::ReadOnly, N> {
        CellQuery::<L, <Q as WorldQuery>::ReadOnly, <F as WorldQuery>::ReadOnly, N> {
            cell_q: self.cell_q.to_readonly(),
            chunk_q: self.chunk_q.to_readonly(),
            map_q: self.map_q.to_readonly(),
        }
    }

    /// Iter all cells in a given chunk.
    /// # Note
    /// The coordinates for this function are givne in chunk coordinates.
    pub fn iter_in_chunk(&self, chunk_c: [isize; N]) -> CellQueryIter<'_, 's, L, Q, F, N> {
        // Get corners of chunk
        let corner_1 = calculate_cell_coordinate(chunk_c, 0, L::CHUNK_SIZE);
        let corner_2 =
            calculate_cell_coordinate(chunk_c, max_cell_index::<N>(L::CHUNK_SIZE), L::CHUNK_SIZE);
        // Create cell iter
        unsafe { CellQueryIter::new(self, corner_1, corner_2) }
    }

    /// Iter all cells in a given chunk.
    /// # Note
    /// The coordinates for this function are givne in chunk coordinates.
    pub fn iter_in_chunk_mut(&self, chunk_c: [isize; N]) -> CellQueryIterMut<'_, 's, L, Q, F, N> {
        // Get corners of chunk
        let corner_1 = calculate_cell_coordinate(chunk_c, 0, L::CHUNK_SIZE);
        let corner_2 =
            calculate_cell_coordinate(chunk_c, max_cell_index::<N>(L::CHUNK_SIZE), L::CHUNK_SIZE);
        // Create cell iter
        unsafe { CellQueryIterMut::new(self, corner_1, corner_2) }
    }

    /// Iter all cells in the chunks in the given range.
    /// # Note
    /// The coordinates for this function are givne in chunk coordinates.
    pub fn iter_in_chunks(
        &mut self,
        chunk_c_1: [isize; N],
        chunk_c_2: [isize; N],
    ) -> CellQueryIter<'_, 's, L, Q, F, N> {
        // Get corners of chunk
        let corner_1 = calculate_cell_coordinate(chunk_c_1, 0, L::CHUNK_SIZE);
        let corner_2 =
            calculate_cell_coordinate(chunk_c_2, max_cell_index::<N>(L::CHUNK_SIZE), L::CHUNK_SIZE);
        // Create cell iter
        unsafe { CellQueryIter::new(self, corner_1, corner_2) }
    }

    /// Iter all cells in the chunks in the given range.
    /// # Note
    /// The coordinates for this function are givne in chunk coordinates.
    pub fn iter_in_chunks_mut(
        &mut self,
        chunk_c_1: [isize; N],
        chunk_c_2: [isize; N],
    ) -> CellQueryIterMut<'_, 's, L, Q, F, N> {
        // Get corners of chunk
        let corner_1 = calculate_cell_coordinate(chunk_c_1, 0, L::CHUNK_SIZE);
        let corner_2 =
            calculate_cell_coordinate(chunk_c_2, max_cell_index::<N>(L::CHUNK_SIZE), L::CHUNK_SIZE);
        // Create cell iter
        unsafe { CellQueryIterMut::new(self, corner_1, corner_2) }
    }
}

pub struct CellQueryIter<'w, 's, L, Q, F, const N: usize>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    coord_iter: CoordIterator<N>,
    cell_q: &'w CellQuery<'w, 's, L, Q, F, N>,
}

impl<'w, 's, L, Q, F, const N: usize> CellQueryIter<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    /// # Safety
    /// This iterator uses unchecked get's to get around some lifetime issue I don't understand yet.
    /// Due to this, you should only call this constructor from a context where the query is actually
    /// borrowed mutabley.
    unsafe fn new(
        cell_q: &'w CellQuery<'w, 's, L, Q, F, N>,
        corner_1: [isize; N],
        corner_2: [isize; N],
    ) -> Self {
        Self {
            cell_q,
            coord_iter: CoordIterator::new(corner_1, corner_2),
        }
    }
}

impl<'w, 's, L, Q, F, const N: usize> Iterator for CellQueryIter<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Item = <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'w>;

    #[allow(clippy::while_let_on_iterator)]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(target) = self.coord_iter.next() {
            // This fixes some lifetime issue that I'm not sure I understand quite yet, will do testing
            let cell = self.cell_q.get_at(target);
            if cell.is_some() {
                return cell;
            }
        }

        None
    }
}

pub struct CellQueryIterMut<'w, 's, L, Q, F, const N: usize>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    coord_iter: CoordIterator<N>,
    cell_q: &'w CellQuery<'w, 's, L, Q, F, N>,
}

impl<'w, 's, L, Q, F, const N: usize> CellQueryIterMut<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    /// # Safety
    /// This iterator uses unchecked get's to get around some lifetime issue I don't understand yet.
    /// Due to this, you should only call this constructor from a context where the query is actually
    /// borrowed mutabley.
    unsafe fn new(
        cell_q: &'w CellQuery<'w, 's, L, Q, F, N>,
        corner_1: [isize; N],
        corner_2: [isize; N],
    ) -> Self {
        Self {
            cell_q,
            coord_iter: CoordIterator::new(corner_1, corner_2),
        }
    }
}

impl<'w, 's, L, Q, F, const N: usize> Iterator for CellQueryIterMut<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Item = <Q as WorldQuery>::Item<'w>;

    #[allow(clippy::while_let_on_iterator)]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(target) = self.coord_iter.next() {
            // This fixes some lifetime issue that I'm not sure I understand quite yet, will do testing
            let cell = unsafe { self.cell_q.get_at_unchecked(target) };
            if cell.is_some() {
                return cell;
            }
        }

        None
    }
}
