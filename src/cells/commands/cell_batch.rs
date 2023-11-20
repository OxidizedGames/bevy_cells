use aery::edges::{Set, Unset};
use bevy::{
    ecs::{bundle::Bundle, entity::Entity, system::Command, world::World},
    log::info,
    utils::HashMap,
};
use bimap::BiMap;

use crate::prelude::{
    calculate_cell_index, calculate_chunk_coordinate, CellCoord, CellIndex, CellMapLabel, Chunk,
    InChunk,
};

use super::{remove_chunk, remove_map, spawn_or_remove_chunk, spawn_or_remove_map, GroupBy};

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
        info!("Expanding cells!");
        let (cell_cs, bundles): (Vec<[isize; N]>, Vec<B>) = self
            .cell_cs
            .into_iter()
            .map(|coord| (coord, (self.bundle_f)(coord)))
            .unzip();

        info!("Spawning cells!");
        let cells = cell_cs
            .into_iter()
            .zip(world.spawn_batch(bundles))
            .collect::<Vec<([isize; N], Entity)>>();

        info!("Inserting cells!");
        insert_cell_batch::<L, N>(world, cells);
    }
}

fn insert_cell_batch<L, const N: usize>(
    world: &mut World,
    cells: impl IntoIterator<Item = ([isize; N], Entity)>,
) where
    L: CellMapLabel + Send + 'static,
{
    info!("Chunking cells!");
    let chunked_cells = cells
        .into_iter()
        .group_by(|(cell_c, _)| calculate_chunk_coordinate(*cell_c, L::CHUNK_SIZE));

    info!("Removing maps!");
    // Remove the map, or spawn an entity to hold the map, then create an empty map
    let (map_id, mut map) = spawn_or_remove_map::<L, N>(world);

    info!("Removing chunks!");
    // Get the chunks and entities from the map
    let cells_with_chunk = Vec::from_iter(chunked_cells.into_iter().map(|(chunk_c, cells)| {
        let (chunk_id, chunk) = spawn_or_remove_chunk(world, &mut map, map_id, chunk_c);
        (chunk_id, chunk, cells)
    }));

    info!("Inserting cells into chunks!");
    for (chunk_id, mut chunk, cells) in cells_with_chunk {
        for (cell_c, cell_id) in cells {
            let cell_i = calculate_cell_index(cell_c, L::CHUNK_SIZE);

            if let Some(cell) = chunk.cells.get_mut(cell_i) {
                if let Some(old_cell_id) = cell.replace(cell_id) {
                    world.despawn(old_cell_id);
                }
            }

            Set::<InChunk<L>>::new(cell_id, chunk_id).apply(world);

            world
                .get_entity_mut(cell_id)
                .unwrap()
                .insert((CellIndex::from(cell_i), CellCoord::<N>::new(cell_c)));
        }

        info!("Inserting chunk");
        world.get_entity_mut(chunk_id).unwrap().insert(chunk);
    }

    world.get_entity_mut(map_id).unwrap().insert(map);
}

pub struct DespawnCellBatch<L, IC, const N: usize = 2>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    pub cell_cs: IC,
    pub label: std::marker::PhantomData<L>,
}
impl<L, IC, const N: usize> DespawnCellBatch<L, IC, N>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    fn take_cells(self, world: &mut World) -> Vec<([isize; N], Entity)> {
        // Group cells by chunk
        let chunked_cells = self
            .cell_cs
            .into_iter()
            .group_by(|cell_c| calculate_chunk_coordinate(*cell_c, L::CHUNK_SIZE));

        // Remove the map, or return if it doesn't exist
        let (map_id, mut map) = if let Some(map_info) = remove_map::<L, N>(world) {
            map_info
        } else {
            return Vec::new();
        };

        // Get the chunks and entities from the map
        let cells_with_chunk = chunked_cells
            .into_iter()
            .filter_map(|(chunk_c, cells)| {
                remove_chunk(world, &mut map, chunk_c)
                    .map(|chunk_info| (chunk_info.0, chunk_info.1, cells))
            })
            .map(|(chunk_id, chunk, cells)| {
                (
                    chunk_id,
                    chunk,
                    cells.into_iter().collect::<Vec<[isize; N]>>(),
                )
            })
            .collect::<Vec<(Entity, Chunk, Vec<[isize; N]>)>>();

        let mut cell_ids = Vec::new();
        for (chunk_id, mut chunk, cells) in cells_with_chunk {
            for cell_c in cells {
                let cell_i = calculate_cell_index(cell_c, L::CHUNK_SIZE);

                if let Some(cell) = chunk.cells.get_mut(cell_i) {
                    if let Some(cell_id) = cell.take() {
                        Unset::<InChunk<L>>::new(cell_id, chunk_id).apply(world);
                        cell_ids.push((cell_c, cell_id));
                    }
                }
            }

            world.get_entity_mut(chunk_id).unwrap().insert(chunk);
        }

        world.get_entity_mut(map_id).unwrap().insert(map);
        cell_ids
    }
}

impl<L, IC, const N: usize> Command for DespawnCellBatch<L, IC, N>
where
    L: CellMapLabel + Send + 'static,
    IC: IntoIterator<Item = [isize; N]> + Send + 'static,
{
    fn apply(self, world: &mut World) {
        for (_, cell_id) in self.take_cells(world) {
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

        let removed = DespawnCellBatch {
            cell_cs: cell_cs.keys().cloned().collect::<Vec<[isize; N]>>(),
            label: self.label,
        }
        .take_cells(world)
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

        let removed_left = DespawnCellBatch {
            cell_cs: cell_cs.left_values().cloned().collect::<Vec<[isize; N]>>(),
            label: self.label,
        }
        .take_cells(world)
        .into_iter()
        .map(|(cell_c, cell_id)| (*cell_cs.get_by_left(&cell_c).expect(ERR_MESSAGE), cell_id));

        let removed_right = DespawnCellBatch {
            cell_cs: cell_cs.right_values().cloned().collect::<Vec<[isize; N]>>(),
            label: self.label,
        }
        .take_cells(world)
        .into_iter()
        .map(|(cell_c, cell_id)| (*cell_cs.get_by_right(&cell_c).expect(ERR_MESSAGE), cell_id));

        insert_cell_batch::<L, N>(world, removed_left.chain(removed_right));
    }
}
