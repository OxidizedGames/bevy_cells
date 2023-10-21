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

impl<'w, 's, L, Q, F, const N: usize> CellQuery<'w, 's, L, Q, F, N>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
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

    pub fn get_at_mut(&mut self, cell_c: [isize; N]) -> Option<<Q as WorldQuery>::Item<'_>> {
        let map = self.map_q.get_single().ok()?;
        let chunk_c = calculate_chunk_coordinate(cell_c, L::CHUNK_SIZE);
        let chunk_e = map.chunks.get(&chunk_c.into())?;

        let chunk = self.chunk_q.get(*chunk_e).ok()?;
        let cell_index = calculate_cell_index(cell_c, L::CHUNK_SIZE);
        let cell_e = chunk.cells.get(cell_index)?.as_ref()?;

        self.cell_q.get_mut(*cell_e).ok()
    }
}

impl<'w, 's, L, Q, F> Deref for CellQuery<'w, 's, L, Q, F>
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

impl<'w, 's, L, Q, F> DerefMut for CellQuery<'w, 's, L, Q, F>
where
    L: CellMapLabel + 'static,
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell_q
    }
}
