`bevy_cells` is a general purpose grided entity library meant to support tilemap libraries, or other libraries that require accessing entities in a grid based manner built on top of the [`aery`](https://github.com/iiYese/aery) relations crate.

Currently, `bevy_cells` supports the following:
* Automatic chunking (including access to chunk entities)
* Automatic map creation
* Hierarchical despawning of chunks and maps
* N-dimensional map support
* Map based quiries
* Spatial queries

Upcoming features:
* Batched operations for better performance
* Automatigically handle hierarchical deletes (via aery support or supported directly in this crate)
* Sort cells in memory based on chunk and map (will require bevy API additions in the future)