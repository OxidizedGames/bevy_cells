use aery::edges::CheckedDespawn;
use bevy::ecs::{entity::Entity, query::With, system::Command, world::World};

use crate::prelude::{CellMap, CellMapLabel};

pub struct DespawnMap<L, const N: usize = 2> {
    pub label: std::marker::PhantomData<L>,
}

impl<L, const N: usize> Command for DespawnMap<L, N>
where
    L: CellMapLabel + Send + 'static,
{
    fn apply(self, world: &mut World) {
        if let Ok(map_id) = world
            .query_filtered::<Entity, With<CellMap<L, N>>>()
            .get_single(world)
        {
            CheckedDespawn(map_id).apply(world);
        }
    }
}
