use std::marker::PhantomData;

use aery::{
    edges::CheckedDespawn,
    prelude::{Set, Unset},
    scope::EntityMutExt,
};
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

    fn despawn_map<L>(&mut self) -> &mut Commands<'w, 's>
    where
        L: CellMapLabel + Send + 'static;

    fn move_cell<L>(&mut self, old_c: [isize; 2], new_c: [isize; 2]) -> &mut Commands<'w, 's>
    where
        L: CellMapLabel + Send + 'static;
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

    fn despawn_map<L>(&mut self) -> &mut Self
    where
        L: CellMapLabel + Send + 'static,
    {
        self.add(DespawnMap::<L> { label: PhantomData });
        self
    }

    fn move_cell<L>(&mut self, old_c: [isize; 2], new_c: [isize; 2]) -> &mut Self
    where
        L: CellMapLabel + Send + 'static,
    {
        self.add(MoveCell::<L> {
            old_c,
            new_c,
            label: PhantomData,
        });
        self
    }
}

struct MoveCell<L> {
    old_c: [isize; 2],
    new_c: [isize; 2],
    label: std::marker::PhantomData<L>,
}

impl<L> Command for MoveCell<L>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        if self.old_c == self.new_c {
            return;
        }

        // Get the map or return
        let mut map_e = if let Some(map) = world
            .query_filtered::<Entity, With<MapLabel<L>>>()
            .get_single_mut(world)
            .ok()
            .and_then(|map_id| world.get_entity_mut(map_id))
        {
            map
        } else {
            return;
        };

        // Get the old chunk or return
        let old_chunk_c = calculate_chunk_coordinate(self.old_c, L::CHUNK_SIZE).into();
        let mut old_chunk_e = if let Some(chunk_id) = map_e
            .get::<CellMap>()
            .unwrap()
            .chunks
            .get_by_left(&old_chunk_c)
            .copied()
            .and_then(|chunk_e| {
                map_e
                    .world()
                    .get_entity(chunk_e)
                    .map(|chunk_e| chunk_e.id())
            }) {
            world.get_entity_mut(chunk_id).unwrap()
        } else {
            return;
        };

        // Remove the old entity or return if the old entity is already deleted
        let mut old_chunk = old_chunk_e.get_mut::<Chunk>().unwrap();
        let old_cell_i = calculate_cell_index(self.old_c, L::CHUNK_SIZE);
        let old_cell_id = if let Some((_, cell_id)) = old_chunk.cells.remove_by_left(&old_cell_i) {
            cell_id
        } else {
            return;
        };

        let old_chunk_id = old_chunk_e.id();

        if world.get_entity(old_cell_id).is_none() {
            return;
        }

        // Remove the old relation
        Unset::<InChunk<L>>::new(old_cell_id, old_chunk_id).apply(world);

        SpawnCell::<L> {
            cell_c: self.new_c,
            cell_e: old_cell_id,
            label: self.label,
        }
        .apply(world);
    }
}

struct DespawnMap<L> {
    label: std::marker::PhantomData<L>,
}

impl<L> Command for DespawnMap<L>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        if let Ok(map_id) = world
            .query_filtered::<Entity, With<MapLabel<L>>>()
            .get_single(world)
        {
            CheckedDespawn(map_id).apply(world);
        }
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
            .get_by_left(&chunk_c)
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

        let out_of_chunk = chunk.cells.insert(cell_i, self.cell_e);

        match out_of_chunk {
            bimap::Overwritten::Neither => {}
            // We replaced an old index
            bimap::Overwritten::Left(_, cell_id) => {
                world.despawn(cell_id);
            }
            _ => panic!("The same entity found in the map twice"),
        };

        Set::<InChunk<L>>::new(self.cell_e, chunk_id).apply(world);
    }
}
