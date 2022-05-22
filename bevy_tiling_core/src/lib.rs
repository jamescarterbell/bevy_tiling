use std::{mem::size_of, slice::from_raw_parts};

use bevy::{
    ecs::system::SystemParam,
    math::IVec3,
    prelude::{CoreStage, Plugin, Res, ResMut, StageLabel, SystemStage},
    utils::{hashbrown::hash_map::Keys, HashMap, HashSet},
};

pub struct TilingCorePlugin;

impl Plugin for TilingCorePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<TileMap>()
            .init_resource::<TileMapUpdates>()
            .add_stage_after(
                CoreStage::Update,
                TilingCoreStage::Update,
                SystemStage::parallel(),
            )
            .add_stage_after(
                TilingCoreStage::Update,
                TilingCoreStage::Clear,
                SystemStage::parallel(),
            )
            .add_system_to_stage(CoreStage::PreUpdate, clear_tile_updates);
    }
}

fn clear_tile_updates(mut updates: ResMut<TileMapUpdates>) {
    updates.chunks.clear();
}

#[derive(StageLabel, PartialEq, Eq, Clone, Hash, Debug)]
pub enum TilingCoreStage {
    Update,
    Clear,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Tile {
    sheet: u16,
    index: u16,
}

impl Tile {
    /// Create a new [`Tile`] from raw chunk and index info.
    /// # Notes
    /// Recommended for internal and library use only.
    pub fn new(sheet: u16, index: u16) -> Self {
        Self { sheet, index }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct TileCoord {
    index: u8,
    chunk: IVec3,
}

impl TileCoord {
    /// Create a new [`TileCoord`] from raw chunk and index info.
    /// # Notes
    /// Recommended for internal and library use only.
    pub fn new(chunk: IVec3, index: u8) -> Self {
        Self { chunk, index }
    }
}

pub struct Chunk {
    tiles: [Tile; 256],
    valid: [bool; 256],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            tiles: [Tile { sheet: 0, index: 0 }; 256],
            valid: [false; 256],
        }
    }
}

impl Chunk {
    #[inline]
    pub fn get_tile(&self, coord: u8) -> Option<&Tile> {
        if self.valid[coord as usize] {
            return Some(&self.tiles[coord as usize]);
        }
        None
    }

    #[inline]
    pub fn get_tile_mut(&mut self, coord: u8) -> Option<&mut Tile> {
        if self.valid[coord as usize] {
            return Some(&mut self.tiles[coord as usize]);
        }
        None
    }

    #[inline]
    pub fn set_tile(&mut self, coord: u8, tile: Option<Tile>) -> Option<Tile> {
        let mut res = None;
        if self.valid[coord as usize] {
            res = Some(self.tiles[coord as usize]);
        }
        match tile {
            Some(tile) => {
                self.tiles[coord as usize] = tile;
                self.valid[coord as usize] = true;
            }
            None => self.valid[coord as usize] = false,
        };
        res
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            from_raw_parts(
                self.tiles.as_ptr() as *const u8,
                size_of::<Tile>() * 256 + size_of::<bool>() * 256,
            )
        }
    }
}

#[derive(Default)]
pub struct TileMap {
    chunks: HashMap<IVec3, Chunk>,
}

impl TileMap {
    pub fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk> {
        self.chunks.get(coord)
    }

    pub fn get_chunk_mut(&mut self, coord: &IVec3) -> Option<&mut Chunk> {
        self.chunks.get_mut(coord)
    }

    pub fn set_tile(&mut self, coord: &TileCoord, tile: Option<Tile>) -> Option<Tile> {
        match self.chunks.get_mut(&coord.chunk) {
            Some(chunk) => chunk.set_tile(coord.index, tile),
            None => {
                if tile.is_none() {
                    None
                } else {
                    self.chunks.insert(coord.chunk, Chunk::default());
                    None
                }
            }
        }
    }
}

#[derive(Default)]
pub struct TileMapUpdates {
    chunks: HashMap<IVec3, HashSet<u8>>,
}

impl TileMapUpdates {
    pub fn set_update(&mut self, coord: &TileCoord) {
        let chunk = match self.chunks.get_mut(&coord.chunk) {
            Some(chunk) => chunk,
            None => {
                self.chunks.insert(coord.chunk, HashSet::default());
                self.chunks.get_mut(&coord.chunk).unwrap()
            }
        };
        chunk.insert(coord.index);
    }

    pub fn get_chunk_updates(&self) -> Keys<IVec3, HashSet<u8>> {
        self.chunks.keys()
    }
}

#[derive(SystemParam)]
pub struct TileMapReader<'w, 's> {
    chunks: Res<'w, TileMap>,
    updates: Res<'w, TileMapUpdates>,
    #[system_param(ignore)]
    marker: std::marker::PhantomData<&'s Tile>,
}

#[derive(SystemParam)]
pub struct TileMapWriter<'w, 's> {
    chunks: ResMut<'w, TileMap>,
    updates: ResMut<'w, TileMapUpdates>,
    #[system_param(ignore)]
    marker: std::marker::PhantomData<&'s Tile>,
}

pub trait MapReader {
    fn get_tile(&self, coord: &TileCoord) -> Option<&Tile>;

    fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk>;

    fn get_chunk_updates(&self) -> Keys<IVec3, HashSet<u8>>;

    fn is_chunk_updated(&self, coord: &IVec3) -> bool;
}

impl<'w, 's> MapReader for TileMapReader<'w, 's> {
    #[inline]
    fn get_tile(&self, coord: &TileCoord) -> Option<&Tile> {
        if let Some(chunk) = self.chunks.get_chunk(&coord.chunk) {
            return chunk.get_tile(coord.index);
        }
        None
    }

    #[inline]
    fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk> {
        self.chunks.get_chunk(coord)
    }

    #[inline]
    fn get_chunk_updates(&self) -> Keys<IVec3, HashSet<u8>> {
        self.updates.get_chunk_updates()
    }

    #[inline]
    fn is_chunk_updated(&self, coord: &IVec3) -> bool {
        self.updates.chunks.contains_key(coord)
    }
}

impl<'w, 's> MapReader for TileMapWriter<'w, 's> {
    #[inline]
    fn get_tile(&self, coord: &TileCoord) -> Option<&Tile> {
        if let Some(chunk) = self.chunks.get_chunk(&coord.chunk) {
            return chunk.get_tile(coord.index);
        }
        None
    }

    #[inline]
    fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk> {
        self.chunks.get_chunk(coord)
    }

    #[inline]
    fn get_chunk_updates(&self) -> Keys<IVec3, HashSet<u8>> {
        self.updates.get_chunk_updates()
    }

    #[inline]
    fn is_chunk_updated(&self, coord: &IVec3) -> bool {
        self.updates.chunks.contains_key(coord)
    }
}

impl<'w, 's> TileMapWriter<'w, 's> {
    /// Sets the tile at a given coordinate to a new tile, or removes it if None is given.
    /// This method causes updates.
    #[inline]
    pub fn set_tile(&mut self, coord: &TileCoord, tile: Option<Tile>) -> Option<Tile> {
        let old = self.chunks.set_tile(coord, tile);
        if old != tile {
            self.updates.set_update(coord);
        }
        old
    }

    /// Sets the tile at a given coordinate to a new tile, or removes it if None is given.
    /// This method does not cause updates.
    #[inline]
    pub fn set_tile_no_update(&mut self, coord: &TileCoord, tile: Option<Tile>) -> Option<Tile> {
        self.chunks.set_tile(coord, tile)
    }

    /// Accessing a tile via this method does not cause updates.
    #[inline]
    pub fn get_tile_mut(&mut self, coord: &TileCoord) -> Option<&mut Tile> {
        if let Some(chunk) = self.chunks.get_chunk_mut(&coord.chunk) {
            return chunk.get_tile_mut(coord.index);
        }
        None
    }

    /// Accessing a chunk via this method does not cause updates.
    #[inline]
    pub fn get_chunk_mut(&mut self, coord: &IVec3) -> Option<&mut Chunk> {
        self.chunks.get_chunk_mut(coord)
    }

    /// Manually mark a chunk as updated, without actually changing values in the chunk.
    #[inline]
    pub fn mark_chunk_updated(&mut self, coord: &IVec3) {
        if !self.is_chunk_updated(coord) {
            self.updates.chunks.insert(*coord, HashSet::default());
        }
    }

    /// Get mutable access to a tile from a shared reference.
    /// # Safety
    /// This function breaks basic borrowing rules, it should be used not at all or very carefully.
    /// This is mainly included to make a particular implementation of autotiling possible.
    #[inline]
    pub unsafe fn get_tile_mut_unchecked(&self, coord: &TileCoord) -> Option<&mut Tile> {
        self.get_tile(coord)
            .map(|tile| unsafe { (tile as *const Tile as *mut Tile).as_mut().unwrap() })
    }

    /// Get mutable access to a tile from a shared reference.
    /// # Safety
    /// This function breaks basic borrowing rules, it should be used not at all or very carefully.
    /// This is mainly included to make a particular implementation of autotiling possible.
    #[inline]
    pub unsafe fn get_chunk_mut_unchecked(&self, coord: &IVec3) -> Option<&mut Chunk> {
        self.get_chunk(coord)
            .map(|chunk| unsafe { (chunk as *const Chunk as *mut Chunk).as_mut().unwrap() })
    }
}

#[cfg(test)]
mod test {
    use core::mem::size_of;

    use crate::{Chunk, Tile};

    #[test]
    fn chunk_layout() {
        assert_eq!(
            size_of::<Chunk>(),
            size_of::<Tile>() * 256 + size_of::<bool>() * 256
        );
        let chunk = Chunk::default();
        let tiles = &chunk.tiles[..];
        let valid = &chunk.valid[..];
        let tiles_end = tiles.as_ptr().wrapping_add(tiles.len()) as *const u8;
        assert_eq!(tiles_end, valid.as_ptr() as *const u8);
    }
}
