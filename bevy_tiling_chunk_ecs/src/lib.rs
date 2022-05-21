use bevy::{
    math::IVec3,
    prelude::{Commands, Component, Entity, Plugin, ResMut},
    utils::HashMap,
};
use bevy_tiling_core::{MapReader, TileMapReader, TilingCoreStage};

pub struct BevyTilingChunkEcs;

impl Plugin for BevyTilingChunkEcs {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<ChunkMap>()
            .add_system_to_stage(TilingCoreStage::Update, update_chunk_map);
    }
}

fn update_chunk_map(
    mut commands: Commands,
    tile_map_reader: TileMapReader,
    mut chunk_map: ResMut<ChunkMap>,
) {
    for chunk_update in tile_map_reader.get_chunk_updates() {
        if chunk_map.get_chunk_entity(chunk_update).is_none() {
            chunk_map.insert_chunk(chunk_update, &commands.spawn_bundle((ChunkMarker,)).id());
        }
    }
}

/// Marks an entity as a Chunk.
#[derive(Component)]
pub struct ChunkMarker;

/// Contains mappings for tiling internal chunk representations
/// to ecs entity chunk representations.
#[derive(Default)]
pub struct ChunkMap {
    ent_to_int: HashMap<Entity, IVec3>,
    int_to_ent: HashMap<IVec3, Entity>,
}

impl ChunkMap {
    /// Get the internal bevy_tiling key of a chunk entity.
    pub fn get_chunk_index(&self, ent: &Entity) -> Option<&IVec3> {
        self.ent_to_int.get(ent)
    }

    /// Get the chunk entity of an internal bevy_tiling key.
    pub fn get_chunk_entity(&self, int: &IVec3) -> Option<&Entity> {
        self.int_to_ent.get(int)
    }

    /// Add a new key <-> entity mapping, returning any previous mapping that existed.
    pub fn insert_chunk(&mut self, int: &IVec3, ent: &Entity) -> Option<(IVec3, Entity)> {
        let mut prev = None;
        if let Some(old_int) = self.ent_to_int.insert(*ent, *int) {
            prev = Some((old_int, *ent));
        }
        // only need to check the first one as these will always be in sync
        self.int_to_ent.insert(*int, *ent);
        prev
    }

    /// Removes the given chunk from the map using the tiling key.
    pub fn remove_chunk_by_key(&mut self, int: &IVec3) -> Option<Entity> {
        let entity = self.int_to_ent.remove(int);
        if let Some(ent) = entity {
            self.ent_to_int.remove(&ent);
        }
        entity
    }

    /// Removes the given chunk from the map using the entity.
    pub fn remove_chunk_by_entity(&mut self, ent: &Entity) -> Option<IVec3> {
        let key = self.ent_to_int.remove(ent);
        if let Some(key) = key {
            self.int_to_ent.remove(&key);
        }
        key
    }
}
