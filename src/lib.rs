#![deny(clippy::all)]

pub mod tiles;

pub mod prelude {
    use std::ops::Deref;

    use bevy::ecs::query::WorldQuery;

    pub use crate::tiles::cellquery::*;
    pub use crate::tiles::commands::{CellCommandExt, CellCommands};
    pub use crate::tiles::CellMapLabel;

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
