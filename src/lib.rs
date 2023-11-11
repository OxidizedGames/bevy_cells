#![deny(clippy::all)]

pub mod cells;

pub mod prelude {
    use std::ops::Deref;

    use bevy::ecs::query::WorldQuery;

    pub use crate::cells::cell_query::*;
    pub use crate::cells::commands::{CellCommandExt, CellCommands};
    pub use crate::cells::CellMapLabel;

    use crate::cells;

    #[derive(WorldQuery)]
    pub struct CellIndex {
        inner: &'static cells::CellIndex,
    }

    impl<'w> Deref for CellIndexItem<'w> {
        type Target = usize;

        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }

    #[derive(WorldQuery)]
    pub struct CellCoord<const N: usize = 2> {
        inner: &'static cells::CellCoord<N>,
    }

    impl<'w, const N: usize> Deref for CellCoordItem<'w, N> {
        type Target = [isize; N];

        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }

    #[derive(WorldQuery)]
    pub struct ChunkCoord<const N: usize = 2> {
        inner: &'static cells::ChunkCoord<N>,
    }

    impl<'w, const N: usize> Deref for ChunkCoordItem<'w, N> {
        type Target = [isize; N];

        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }
}

