use bevy::{
    ecs::system::SystemParam,
    math::IVec3,
    prelude::{CoreStage, Plugin, Res, ResMut},
    utils::{HashMap, HashSet},
};

pub struct TilingPlugin<T>(std::marker::PhantomData<T>);

impl<T> Plugin for TilingPlugin<T>
where
    T: Default + Copy + Clone + Eq + PartialEq + Send + Sync + 'static,
{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<TileMap<T>>()
            .init_resource::<TileMapUpdates<T>>()
            .add_system_to_stage(CoreStage::PreUpdate, clear_tile_updates::<T>);
    }
}

fn clear_tile_updates<T>(mut updates: ResMut<TileMapUpdates<T>>)
where
    T: Send + Sync + 'static,
{
    updates.chunks.clear();
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Tile {
    sheet: u16,
    index: u16,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct TileCoord {
    index: u8,
    chunk: IVec3,
}

pub struct Chunk<T> {
    tiles: [T; 256],
    valid: [bool; 256],
}

impl<T: Copy + Default> Default for Chunk<T> {
    fn default() -> Self {
        Self {
            tiles: [T::default(); 256],
            valid: [false; 256],
        }
    }
}

impl<T: Copy> Chunk<T> {
    pub fn get_tile(&self, coord: u8) -> Option<&T> {
        if self.valid[coord as usize] {
            return Some(&self.tiles[coord as usize]);
        }
        None
    }

    pub fn get_tile_mut(&mut self, coord: u8) -> Option<&mut T> {
        if self.valid[coord as usize] {
            return Some(&mut self.tiles[coord as usize]);
        }
        None
    }

    pub fn set_tile(&mut self, coord: u8, tile: Option<T>) -> Option<T> {
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
}

#[derive(Default)]
pub struct TileMap<T> {
    chunks: HashMap<IVec3, Chunk<T>>,
}

impl<T: Copy + Default> TileMap<T> {
    pub fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk<T>> {
        self.chunks.get(coord)
    }

    pub fn get_chunk_mut(&mut self, coord: &IVec3) -> Option<&mut Chunk<T>> {
        self.chunks.get_mut(coord)
    }

    pub fn set_tile(&mut self, coord: &TileCoord, tile: Option<T>) -> Option<T> {
        match self.chunks.get_mut(&coord.chunk) {
            Some(chunk) => chunk.set_tile(coord.index, tile),
            None => {
                if tile.is_none() {
                    None
                } else {
                    self.chunks.insert(coord.chunk, Chunk::<T>::default());
                    None
                }
            }
        }
    }
}

#[derive(Default)]
pub struct TileMapUpdates<T> {
    chunks: HashMap<IVec3, HashSet<u8>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TileMapUpdates<T> {
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
}

#[derive(SystemParam)]
pub struct TileMapReader<'w, 's, T: Send + Sync + 'static> {
    pub chunks: Res<'w, TileMap<T>>,
    pub updates: Res<'w, TileMapUpdates<T>>,
    #[system_param(ignore)]
    marker: std::marker::PhantomData<&'s T>,
}

#[derive(SystemParam)]
pub struct TileMapWriter<'w, 's, T: Send + Sync + 'static> {
    pub chunks: ResMut<'w, TileMap<T>>,
    pub updates: ResMut<'w, TileMapUpdates<T>>,
    #[system_param(ignore)]
    marker: std::marker::PhantomData<&'s T>,
}

pub trait MapReader<T> {
    fn get_tile(&self, coord: &TileCoord) -> Option<&T>;

    fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk<T>>;
}

impl<'w, 's, T: Copy + Default + Send + Sync + 'static> MapReader<T> for TileMapReader<'w, 's, T> {
    #[inline]
    fn get_tile(&self, coord: &TileCoord) -> Option<&T> {
        if let Some(chunk) = self.chunks.get_chunk(&coord.chunk) {
            return chunk.get_tile(coord.index);
        }
        None
    }

    #[inline]
    fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk<T>> {
        self.chunks.get_chunk(coord)
    }
}

impl<'w, 's, T: Copy + Default + Send + Sync + 'static> MapReader<T> for TileMapWriter<'w, 's, T> {
    #[inline]
    fn get_tile(&self, coord: &TileCoord) -> Option<&T> {
        if let Some(chunk) = self.chunks.get_chunk(&coord.chunk) {
            return chunk.get_tile(coord.index);
        }
        None
    }

    fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk<T>> {
        self.chunks.get_chunk(coord)
    }
}

impl<'w, 's, T: Copy + Default + PartialEq + Eq + Send + Sync + 'static> TileMapWriter<'w, 's, T> {
    /// Sets the tile at a given coordinate to a new tile, or removes it if None is given.
    /// This method causes updates.
    #[inline]
    pub fn set_tile(&mut self, coord: &TileCoord, tile: Option<T>) -> Option<T> {
        let old = self.chunks.set_tile(coord, tile);
        if old != tile {
            self.updates.set_update(coord);
        }
        old
    }

    /// Sets the tile at a given coordinate to a new tile, or removes it if None is given.
    /// This method does not cause updates.
    #[inline]
    pub fn set_tile_no_update(&mut self, coord: &TileCoord, tile: Option<T>) -> Option<T> {
        self.chunks.set_tile(coord, tile)
    }

    /// Accessing a tile via this method does not cause updates.
    #[inline]
    pub fn get_tile_mut(&mut self, coord: &TileCoord) -> Option<&mut T> {
        if let Some(chunk) = self.chunks.get_chunk_mut(&coord.chunk) {
            return chunk.get_tile_mut(coord.index);
        }
        None
    }

    /// Accessing a chunk via this method does not cause updates.
    #[inline]
    pub fn get_chunk_mut(&mut self, coord: &IVec3) -> Option<&mut Chunk<T>> {
        self.chunks.get_chunk_mut(coord)
    }

    /// Get mutable access to a tile from a shared reference.
    /// # Safety
    /// This function breaks basic borrowing rules, it should be used not at all or very carefully.
    /// This is mainly included to make a particular implementation of autotiling possible.
    #[inline]
    pub unsafe fn get_tile_mut_unchecked(&self, coord: &TileCoord) -> Option<&mut T> {
        self.get_tile(coord)
            .map(|tile| unsafe { (tile as *const T as *mut T).as_mut().unwrap() })
    }

    /// Get mutable access to a tile from a shared reference.
    /// # Safety
    /// This function breaks basic borrowing rules, it should be used not at all or very carefully.
    /// This is mainly included to make a particular implementation of autotiling possible.
    #[inline]
    pub unsafe fn get_chunk_mut_unchecked(&self, coord: &IVec3) -> Option<&mut Chunk<T>> {
        self.get_chunk(coord).map(|chunk| unsafe {
            (chunk as *const Chunk<T> as *mut Chunk<T>)
                .as_mut()
                .unwrap()
        })
    }
}
