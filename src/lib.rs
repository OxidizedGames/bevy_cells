use aery::{prelude::Set, scope::EntityMutExt};
use bevy::{
    ecs::{
        system::{Command, EntityCommands},
        world::EntityMut,
    },
    prelude::*,
};
use tiles::{
    calculate_cell_index, calculate_chunk_coordinate, CellMap, CellMapLabel, Chunk, InMap, MapLabel,
};

use crate::tiles::InChunk;

pub mod tiles;

pub trait CellCommandExt<'w, 's> {
    fn spawn_cell<L, T>(&mut self, cell_c: [isize; 2], bundle: T) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
        L: CellMapLabel + 'static;
}

impl<'w, 's> CellCommandExt<'w, 's> for Commands<'w, 's> {
    fn spawn_cell<L, T>(&mut self, cell_c: [isize; 2], bundle: T) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
        L: CellMapLabel + 'static,
    {
        let cell_e = self.spawn(bundle).id();
        self.add(SpawnCell::<L> {
            cell_c,
            cell_e,
            label: std::marker::PhantomData,
        });
        self.entity(cell_e)
    }
}

struct SpawnCell<L> {
    cell_c: [isize; 2],
    cell_e: Entity,
    label: std::marker::PhantomData<L>,
}

impl<L> Command for SpawnCell<L>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        // Get the map or insert it
        let mut map_e = if let Some(map) = world
            .query_filtered::<Entity, With<MapLabel<L>>>()
            .get_single_mut(world)
            .ok()
            .and_then(|map_id| world.get_entity_mut(map_id))
        {
            map
        } else {
            world.spawn((CellMap::default(), MapLabel::<L>::new()))
        };

        // Get the chunk or insert it
        let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE).into();
        let mut chunk_e = if let Some(chunk_id) = map_e
            .get::<CellMap>()
            .unwrap()
            .chunks
            .get(&chunk_c)
            .copied()
            .and_then(|chunk_e| {
                map_e
                    .world()
                    .get_entity(chunk_e)
                    .map(|chunk_e| chunk_e.id())
            }) {
            world.get_entity_mut(chunk_id).unwrap()
        } else {
            let mut chunk_id = None;
            let map_id = map_e.id();

            map_e.world_scope(|world| {
                chunk_id = Some(world.spawn(Chunk::new(L::CHUNK_SIZE.pow(2))).id());
                Set::<InMap<L>>::new(chunk_id.unwrap(), map_id).apply(world);
            });

            let chunk_id = chunk_id.unwrap();
            let chunk_c = calculate_chunk_coordinate(self.cell_c, L::CHUNK_SIZE).into();

            map_e
                .get_mut::<CellMap>()
                .unwrap()
                .chunks
                .insert(chunk_c, chunk_id);

            world.get_entity_mut(chunk_id).unwrap()
        };

        // Insert the tile
        let cell_i = calculate_cell_index(self.cell_c, L::CHUNK_SIZE);
        let chunk_id = chunk_e.id();
        let mut chunk = chunk_e.get_mut::<Chunk>().unwrap();
        let mut out_of_chunk = Some(self.cell_e);
        std::mem::swap(&mut chunk.cells[cell_i], &mut out_of_chunk);

        Set::<InChunk<L>>::new(self.cell_e, chunk_id).apply(world);

        // Despawn the old tile
        if let Some(cell_e) = out_of_chunk {
            world.despawn(cell_e);
        }
    }
}
