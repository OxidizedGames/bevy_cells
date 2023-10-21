use std::marker::PhantomData;

use bevy::{ecs::system::EntityCommands, prelude::*};
use tiles::CellMapLabel;

use tiles::commands::{DespawnMap, MoveCell, SpawnCell};

pub mod tiles;

pub mod prelude {
    use std::ops::Deref;

    use bevy::ecs::query::WorldQuery;

    pub use crate::tiles::cellquery::*;
    pub use crate::tiles::CellMapLabel;
    pub use crate::CellCommandExt;

    use crate::tiles;

    #[derive(WorldQuery)]
    pub struct CellIndex {
        inner: &'static tiles::CellIndex,
    }

    impl<'w> Deref for CellIndexItem<'w> {
        type Target = usize;

        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }

    #[derive(WorldQuery)]
    pub struct CellCoord<const N: usize = 2> {
        inner: &'static tiles::CellCoord<N>,
    }

    impl<'w, const N: usize> Deref for CellCoordItem<'w, N> {
        type Target = [isize; N];

        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }

    #[derive(WorldQuery)]
    pub struct ChunkCoord<const N: usize = 2> {
        inner: &'static tiles::ChunkCoord<N>,
    }

    impl<'w, const N: usize> Deref for ChunkCoordItem<'w, N> {
        type Target = [isize; N];

        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }
}

pub trait CellCommandExt<'w, 's> {
    fn spawn_cell<L, T, const N: usize>(
        &mut self,
        cell_c: [isize; N],
        bundle: T,
    ) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
        L: CellMapLabel + 'static;

    fn despawn_map<L, const N: usize>(&mut self) -> &mut Commands<'w, 's>
    where
        L: CellMapLabel + Send + 'static;

    fn move_cell<L, const N: usize>(
        &mut self,
        old_c: [isize; N],
        new_c: [isize; N],
    ) -> &mut Commands<'w, 's>
    where
        L: CellMapLabel + Send + 'static;
}

impl<'w, 's> CellCommandExt<'w, 's> for Commands<'w, 's> {
    fn spawn_cell<L, T, const N: usize>(
        &mut self,
        cell_c: [isize; N],
        bundle: T,
    ) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
        L: CellMapLabel + 'static,
    {
        let cell_e = self.spawn(bundle).id();
        self.add(SpawnCell::<L, N> {
            cell_c,
            cell_id: cell_e,
            label: std::marker::PhantomData,
        });
        self.entity(cell_e)
    }

    fn despawn_map<L, const N: usize>(&mut self) -> &mut Self
    where
        L: CellMapLabel + Send + 'static,
    {
        self.add(DespawnMap::<L, N> { label: PhantomData });
        self
    }

    fn move_cell<L, const N: usize>(&mut self, old_c: [isize; N], new_c: [isize; N]) -> &mut Self
    where
        L: CellMapLabel + Send + 'static,
    {
        self.add(MoveCell::<L, N> {
            old_c,
            new_c,
            label: PhantomData,
        });
        self
    }
}
