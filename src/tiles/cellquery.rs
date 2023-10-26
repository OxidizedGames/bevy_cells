use std::ops::{Deref, DerefMut};

use aery::prelude::*;
use bevy::{
    ecs::{
        query::{ReadOnlyWorldQuery, WorldQuery},
        system::SystemParam,
    },
    prelude::Query,
};

use super::{
    calculate_cell_index, calculate_chunk_coordinate, CellMap, CellMapLabel, Chunk, InChunk, InMap,
};

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
    /// Get's the entity at the cell coordinate in the cell map, if it still exists.
    pub fn get_at(
        &'w self,
        cell_c: [isize; N],
    ) -> Option<<<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'w>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get(*cell_e).ok()
    }

    /// Get's the entity mutably at the cell coordinate in the cell map, if it still exists.
    pub fn get_at_mut(&'w mut self, cell_c: [isize; N]) -> Option<<Q as WorldQuery>::Item<'w>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get_mut(*cell_e).ok()
    }

    /// Iterate over all the cells in a given space, starting at `corner_1`
    /// inclusive over `corner_2`
    pub fn iter_in(
        &'w self,
        corner_1: [isize; N],
        corner_2: [isize; N],
    ) -> CellQueryIter<'w, 's, L, Q, F, N> {
        CellQueryIter::new(self, corner_1, corner_2)
    }

    /*
    /// Iterate over all the cells in a given space, starting at `corner_1`
    /// inclusive over `corner_2`
    pub fn iter_in_mut(
        &'w mut self,
        corner_1: [isize; N],
        corner_2: [isize; N],
    ) -> CellQueryIterMut<'w, 's, L, Q, F, N> {
        CellQueryIterMut::new(self, corner_1, corner_2)
    }
    */

    pub fn to_readonly(
        &self,
    ) -> CellQuery<'_, 's, L, <Q as WorldQuery>::ReadOnly, <F as WorldQuery>::ReadOnly, N> {
        CellQuery::<L, <Q as WorldQuery>::ReadOnly, <F as WorldQuery>::ReadOnly, N> {
            cell_q: self.cell_q.to_readonly(),
            chunk_q: self.chunk_q.to_readonly(),
            map_q: self.map_q.to_readonly(),
        }
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
    fn new(
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
    for<'i> Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Item = <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'w>;

    #[allow(clippy::while_let_on_iterator)]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(target) = self.coord_iter.next() {
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
    cell_q: &'w mut CellQuery<'w, 's, L, Q, F, N>,
}

impl<'w, 's, L, Q, F, const N: usize> CellQueryIterMut<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    fn new(
        cell_q: &'w mut CellQuery<'w, 's, L, Q, F, N>,
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
    type Item = Q::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(target) = self.coord_iter.next() {
            let cell = self.cell_q.get_at_mut(target);
            if cell.is_some() {
                return cell;
            }
        }

        None
    }
}

pub struct CoordIterator<const N: usize> {
    corner_1: [isize; N],
    corner_2: [isize; N],
    current: [isize; N],
    complete: bool,
}

impl<const N: usize> CoordIterator<N> {
    pub fn new(mut corner_1: [isize; N], mut corner_2: [isize; N]) -> Self {
        for i in 0..N {
            if corner_1[i] > corner_2[i] {
                std::mem::swap(&mut corner_1[i], &mut corner_2[i]);
            };
        }

        Self {
            corner_1,
            corner_2,
            current: corner_1,
            complete: false,
        }
    }
}

impl<const N: usize> Iterator for CoordIterator<N> {
    type Item = [isize; N];

    fn next(&mut self) -> Option<Self::Item> {
        if self.complete {
            return None;
        }

        let ret = self.current;

        if self.current == self.corner_2 {
            self.complete = true;
        } else {
            for i in 0..N {
                if self.current[i] == self.corner_2[i] {
                    self.current[i] = self.corner_1[i];
                    continue;
                }
                self.current[i] += 1;
                break;
            }
        }

        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::ops::RangeInclusive;

    use super::CoordIterator;

    fn make_range_iter(val_1: isize, val_2: isize) -> RangeInclusive<isize> {
        if val_1 < val_2 {
            val_1..=val_2
        } else {
            val_2..=val_1
        }
    }

    #[rstest]
    #[case([0, 0, 0], [3, 3, 3])]
    #[case([3, 3, 3], [0, 0, 0])]
    #[case([0, 3, 0], [3, 0, 3])]
    #[case([0, 3, 0], [3, 3, 3])]
    #[case([0, 3, 0], [0, 0, 3])]
    #[case([3, 3, 3], [3, 3, 3])]
    fn coord_iter(#[case] corner_1: [isize; 3], #[case] corner_2: [isize; 3]) {
        let mut iter = CoordIterator::new(corner_1, corner_2);

        for z in make_range_iter(corner_1[2], corner_2[2]) {
            for y in make_range_iter(corner_1[1], corner_2[1]) {
                for x in make_range_iter(corner_1[0], corner_2[0]) {
                    let next = iter.next();
                    println!("Iter: {:?}", next);
                    assert_eq!(Some([x, y, z]), next);
                }
            }
        }

        let next = iter.next();
        println!("Fin: {:?}", next);
        assert_eq!(None, next);
    }
}
