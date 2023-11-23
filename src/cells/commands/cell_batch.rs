use bevy::{
    ecs::{bundle::Bundle, entity::Entity, system::Command, world::World},
    utils::HashMap,
};
use bimap::BiMap;

use crate::prelude::{commands::insert_cell_batch, CellMapLabel};

use super::take_cell_batch;

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

        let cells = cell_cs
            .into_iter()
            .zip(world.spawn_batch(bundles))
            .collect::<Vec<([isize; N], Entity)>>();

        insert_cell_batch::<L, N>(world, cells);
    }
}

pub struct DespawnCellBatch<L, IC, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    pub cell_cs: IC,
    pub label: std::marker::PhantomData<L>,
}

impl<L, IC, const N: usize> Command for DespawnCellBatch<L, IC, N>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    fn apply(self, world: &mut World) {
        for (_, cell_id) in take_cell_batch::<L, N>(world, self.cell_cs) {
            world.despawn(cell_id);
        }
    }
}

pub struct MoveCellBatch<L, IC, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = ([isize; N], [isize; N])> + Send + 'static,
{
    pub cell_cs: IC,
    pub label: std::marker::PhantomData<L>,
}

impl<L, IC, const N: usize> Command for MoveCellBatch<L, IC, N>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = ([isize; N], [isize; N])> + Send + 'static,
{
    fn apply(self, world: &mut World) {
        const ERR_MESSAGE: &str =
            "Couldn't find cell coord in batch move.  Maybe repeated cell coord in command.";

        let mut cell_cs = self
            .cell_cs
            .into_iter()
            .collect::<HashMap<[isize; N], [isize; N]>>();

        let removed =
            take_cell_batch::<L, N>(world, cell_cs.keys().cloned().collect::<Vec<[isize; N]>>())
                .into_iter()
                .map(|(cell_c, cell_id)| (cell_cs.remove(&cell_c).expect(ERR_MESSAGE), cell_id));

        insert_cell_batch::<L, N>(world, removed);
    }
}

pub struct SwapCellBatch<L, IC, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = ([isize; N], [isize; N])> + Send + 'static,
{
    pub cell_cs: IC,
    pub label: std::marker::PhantomData<L>,
}

impl<L, IC, const N: usize> Command for SwapCellBatch<L, IC, N>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = ([isize; N], [isize; N])> + Send + 'static,
{
    fn apply(self, world: &mut World) {
        const ERR_MESSAGE: &str =
            "Couldn't find cell coord in batch move.  Maybe repeated cell coord in command.";

        let cell_cs = self
            .cell_cs
            .into_iter()
            .collect::<BiMap<[isize; N], [isize; N]>>();

        let removed_left = take_cell_batch::<L, N>(
            world,
            cell_cs.left_values().cloned().collect::<Vec<[isize; N]>>(),
        )
        .into_iter()
        .map(|(cell_c, cell_id)| (*cell_cs.get_by_left(&cell_c).expect(ERR_MESSAGE), cell_id));

        let removed_right = take_cell_batch::<L, N>(
            world,
            cell_cs.right_values().cloned().collect::<Vec<[isize; N]>>(),
        )
        .into_iter()
        .map(|(cell_c, cell_id)| (*cell_cs.get_by_right(&cell_c).expect(ERR_MESSAGE), cell_id));

        insert_cell_batch::<L, N>(world, removed_left.chain(removed_right));
    }
}
