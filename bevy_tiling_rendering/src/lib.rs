use std::ops::{Deref, DerefMut};

use bevy::{
    math::IVec3,
    prelude::{Commands, Component, Entity, Plugin, Query, Res, ResMut, Transform, With, Without},
    render::{
        render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
        RenderApp, RenderStage, RenderWorld,
    },
};
use bevy_tiling_chunk_ecs::{ChunkMap, ChunkMarker};
use bevy_tiling_core::{MapReader, TileMapWriter, TilingCoreStage};

pub struct TilingRenderPlugin;

impl Plugin for TilingRenderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<TilingCache>()
            .add_system_to_stage(RenderStage::Cleanup, cache_tile_rendering_entities)
            .add_system_to_stage(RenderStage::Extract, extract);
    }
}

#[derive(Component, Clone)]
pub enum TilingBuffer {
    Unloaded,
    Unmeshed(Buffer),
    Meshed {
        mesh_descriptor: BufferDescriptor<'static>,
        mesh: Buffer,
        unrendered_count: usize,
    },
}

#[derive(Component, Clone)]
/// Placed on rendering world components to point
/// towards gameplay world chunks.
pub struct RenderKey(IVec3);

#[derive(Default)]
struct TilingCache {
    cache: Vec<(Entity, (TilingBuffer, RenderKey))>,
}

impl Deref for TilingCache {
    type Target = Vec<(Entity, (TilingBuffer, RenderKey))>;

    fn deref(&self) -> &Self::Target {
        &self.cache
    }
}

impl DerefMut for TilingCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cache
    }
}

fn extract(
    mut commands: Commands,
    mut tilemap_writer: TileMapWriter,
    chunk_map: Res<ChunkMap>,
    mut chunks: Query<(Entity, &Transform), With<ChunkMarker>>,
    mut render_world: ResMut<RenderWorld>,
) {
    if let Some(mut cache) = render_world.get_resource_mut::<TilingCache>() {
        for (_, (buffer, key)) in cache.iter() {
            // Make sure this chunk still exists
            if tilemap_writer.get_chunk(&key.0).is_some() {
                // Check if the chunk hasn't been updated (since we handle that from gameplay world side)
                if !tilemap_writer.is_chunk_updated(&key.0) {
                    if let TilingBuffer::Unloaded = buffer {
                        // Check if this chunk will be in the camera view this frame, if it will, we should just update it
                        // TODO: CHECK IF IN FRAME HERE, WE NEED TO DO IT DURING EXTRACT WHILE WE HAVE ALL BUFFERS
                        tilemap_writer.mark_chunk_updated(&key.0);
                    }
                }
            }
        }
        let old_cache = std::mem::take(&mut cache.cache);
        commands.insert_or_spawn_batch(old_cache);
    }

    let render_device = render_world
        .get_resource_mut::<RenderDevice>()
        .expect("Couldn't find RenderDevice");

    for (ent, transform) in chunks.iter_mut() {
        let chunk_key = chunk_map
            .get_chunk_index(&ent)
            .expect("Couldn't find chunk in map");

        // If a chunk has been updated, we want to refresh it's tile buffer
        if tilemap_writer.is_chunk_updated(chunk_key) {
            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("raw_tile_buffer"),
                usage: BufferUsages::MAP_READ | BufferUsages::MAP_WRITE,
                contents: tilemap_writer
                    .get_chunk(chunk_key)
                    .expect("Couldn't find chunk!")
                    .as_bytes(),
            });

            commands
                .get_or_spawn(ent)
                .insert(TilingBuffer::Unmeshed(buffer))
                .insert(RenderKey(*chunk_key));

            println!("Hello");
        }

        commands.get_or_spawn(ent).insert(*transform);
    }
}

fn prepare() {}

fn cache_tile_rendering_entities(
    mut tiling_cache: ResMut<TilingCache>,
    render_chunks: Query<(Entity, &TilingBuffer, &RenderKey)>,
) {
    for (entity, buffer, key) in render_chunks.iter() {
        tiling_cache.push((entity, (buffer.clone(), key.clone())));
    }
}

#[cfg(test)]
mod tests {
    use bevy::{math::IVec3, prelude::App, render::RenderApp, DefaultPlugins};
    use bevy_tiling_chunk_ecs::TilingChunkEcsPlugin;
    use bevy_tiling_core::{Tile, TileCoord, TileMapWriter, TilingCorePlugin};

    use crate::{RenderKey, TilingRenderPlugin};

    use crate::TilingCache;

    #[test]
    fn caches_tiles() {
        // place 4 tiles in different chunks
        let mut app = App::new();
        app.add_plugins(DefaultPlugins)
            .add_plugin(TilingCorePlugin)
            .add_plugin(TilingChunkEcsPlugin)
            .add_plugin(TilingRenderPlugin);

        app.add_system(add_4_tile);

        app.update();

        let render_app = app.sub_app_mut(RenderApp);
        let cache = render_app.world.get_resource::<TilingCache>().unwrap();

        assert_eq!(cache.len(), 4);
    }

    fn add_4_tile(mut tilemap_writer: TileMapWriter) {
        tilemap_writer.set_tile(
            &TileCoord::new(IVec3::from((0, 0, 0)), 0),
            Some(Tile::new(0, 0)),
        );
        tilemap_writer.set_tile(
            &TileCoord::new(IVec3::from((1, 0, 0)), 0),
            Some(Tile::new(0, 0)),
        );
        tilemap_writer.set_tile(
            &TileCoord::new(IVec3::from((2, 0, 0)), 0),
            Some(Tile::new(0, 0)),
        );
        tilemap_writer.set_tile(
            &TileCoord::new(IVec3::from((3, 0, 0)), 0),
            Some(Tile::new(0, 0)),
        );
    }
}
