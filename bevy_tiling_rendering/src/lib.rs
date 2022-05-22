use bevy::{
    math::IVec3,
    prelude::{Commands, Component, Entity, Plugin, Query, Res, ResMut, Transform, With},
    render::{
        render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
        RenderApp, RenderStage, RenderWorld,
    },
};
use bevy_tiling_chunk_ecs::{ChunkMap, ChunkMarker};
use bevy_tiling_core::{MapReader, TileMapWriter};

pub struct TilingRenderPlugin;

impl Plugin for TilingRenderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_system_to_stage(RenderStage::Extract, extract);
    }
}

#[derive(Component)]
pub enum TilingBuffer {
    Unloaded,
    Unmeshed(Buffer),
    Meshed {
        mesh_descriptor: BufferDescriptor<'static>,
        mesh: Buffer,
        unrendered_count: usize,
    },
}

#[derive(Component)]
/// Placed on gameplay world components to point towards
/// rendering world components.
struct RenderEntity(Option<Entity>);

#[derive(Component)]
/// Placed on rendering world components to point
/// towards gameplay world chunks.
struct RenderKey(IVec3);

fn extract(
    mut commands: Commands,
    mut tilemap_writer: TileMapWriter,
    chunk_map: Res<ChunkMap>,
    mut chunks: Query<(Entity, &Transform, &mut RenderEntity), With<ChunkMarker>>,
    mut render_world: ResMut<RenderWorld>,
) {
    // Remove the render device so we can perform other borrows from the render_world
    let render_device = render_world
        .remove_resource::<RenderDevice>()
        .expect("Couldn't find RenderDevice");

    let mut rendering_entity_query = render_world.query::<(Entity, &TilingBuffer, &RenderKey)>();
    for (entity, buffer, key) in rendering_entity_query.iter(&render_world) {
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
        } else {
            // Remove nonexistent chunk
            commands.entity(entity).despawn();
        }
    }

    // If a chunk has been updated, we want to refresh it's tile buffer
    for (ent, transform, mut render_ent) in chunks.iter_mut() {
        let chunk_key = chunk_map
            .get_chunk_index(&ent)
            .expect("Couldn't find chunk in map");
        if tilemap_writer.is_chunk_updated(chunk_key) {
            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("raw_tile_buffer"),
                usage: BufferUsages::MAP_READ & BufferUsages::MAP_WRITE,
                contents: tilemap_writer
                    .get_chunk(chunk_key)
                    .expect("Couldn't find chunk!")
                    .as_bytes(),
            });

            let new_ent = commands.spawn().insert(TilingBuffer::Unmeshed(buffer)).id();
            if let Some(old_ent) = render_ent.0.replace(new_ent) {
                commands.entity(old_ent).despawn();
            }
        }
        // Update the render entities position
        if let Some(render_ent) = render_ent.0 {
            commands.entity(render_ent).insert(*transform);
        }
    }

    // Reinsert the render device
    render_world.insert_resource(render_device);
}

fn prepare() {}

fn cleanup() {}
