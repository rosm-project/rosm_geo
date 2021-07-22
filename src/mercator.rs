use crate::rect::{GeoRect, Edge};
use crate::coord::{GeoCoord, TileCoord};

use std::error;
use std::f64::consts::PI;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TileId {
    x: u32,
    y: u32,
    z: u32,
}

impl TileId {
    pub fn new(x: u32, y: u32, z: u32) -> Result<TileId, InvalidTileId> {
        let max = 2u32.pow(z);

        if x < max && y < max {
            Ok(TileId { x, y, z })
        } else {
            Err(InvalidTileId)
        }
    }

    pub fn x(&self) -> u32 {
        self.x
    }

    pub fn y(&self) -> u32 {
        self.y
    }

    pub fn z(&self) -> u32 {
        self.z
    }

    fn flip_y(&mut self) {
        self.y = 2u32.pow(self.z) - 1 - self.y
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TmsTileId(TileId);

impl TmsTileId {
    pub fn new(x: u32, y: u32, z: u32) -> Result<TmsTileId, InvalidTileId> {
        TileId::new(x, y, z).map(|tile_id| { TmsTileId { 0: tile_id } })
    }

    pub fn x(&self) -> u32 {
        self.0.x
    }

    pub fn y(&self) -> u32 {
        self.0.y
    }

    pub fn z(&self) -> u32 {
        self.0.z
    }
}

impl From<TmsTileId> for TileId {
    fn from(tms_tile_id: TmsTileId) -> Self {
        let mut t = tms_tile_id.0;
        t.flip_y();
        t
    }
}

impl From<TileId> for TmsTileId {
    fn from(tile_id: TileId) -> Self {
        let mut t = tile_id;
        t.flip_y();
        TmsTileId { 0: t }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvalidTileId;

impl fmt::Display for InvalidTileId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid tile ID given")
    }
}

impl error::Error for InvalidTileId {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

pub struct TileGrid {
    z: u32,
    tile_extent: u32,
}

impl TileGrid {
    pub fn new(z: u32, tile_extent: u32) -> TileGrid {
        // TODO: basic checks
        TileGrid {
            z, tile_extent
        }
    }

    pub fn tile_id(&self, coord: &GeoCoord) -> (TileId, TileCoord) {
        let count = 2u32.pow(self.z) as f64;

        let x = (coord.lon() + 180.0) / 360.0 * count;

        let lat_rad = coord.lat() * PI / 180.0;
        let y = count * (1.0 - ((lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI)) / 2.0;

        let tile_coord = TileCoord::new(
            (x.fract() * self.tile_extent as f64).floor() as i32,
            (y.fract() * self.tile_extent as f64).floor() as i32,
        );

        (TileId { x: x.floor() as u32, y: y.floor() as u32, z: self.z }, tile_coord)
    }

    pub fn tile_coord(&self, coord: &GeoCoord, tile_id: TileId) -> TileCoord {
        let count = 2u32.pow(self.z) as f64;

        let x = (coord.lon() + 180.0) / 360.0 * count;

        let lat_rad = coord.lat() * PI / 180.0;
        let y = count * (1.0 - ((lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI)) / 2.0;

        let abs_x = (x * self.tile_extent as f64).floor() as i64;
        let abs_y = (y * self.tile_extent as f64).floor() as i64;

        TileCoord::new(
            (abs_x - tile_id.x() as i64 * self.tile_extent as i64) as i32,
            (abs_y - tile_id.y() as i64 * self.tile_extent as i64) as i32,
        )
    }

    pub fn neighbours(&self, tile_id: TileId) -> Vec<(Edge, TileId)> {
        let count = 2u32.pow(self.z);

        let mut result = Vec::with_capacity(8);

        let left_x = if tile_id.x > 0 { tile_id.x - 1 } else { count - 1 };
        result.push((Edge::LEFT, TileId { x: left_x, ..tile_id }));

        let right_x = if tile_id.x < count - 1 { tile_id.x + 1 } else { 0 };
        result.push((Edge::RIGHT, TileId { x: right_x, ..tile_id }));
        
        if tile_id.y > 0 {
            let top_y = tile_id.y - 1;
            result.push((Edge::TOP, TileId { y: top_y, ..tile_id }));
            result.push((Edge::TOP | Edge::LEFT, TileId { x: left_x, y: top_y, ..tile_id }));
            result.push((Edge::TOP | Edge::RIGHT, TileId { x: right_x, y: top_y, ..tile_id }));
        }

        if tile_id.y < count - 1 {
            let bottom_y = tile_id.y + 1;
            result.push((Edge::BOTTOM, TileId { y: bottom_y, ..tile_id }));
            result.push((Edge::BOTTOM | Edge::LEFT, TileId { x: left_x, y: bottom_y, ..tile_id }));
            result.push((Edge::BOTTOM | Edge::RIGHT, TileId { x: right_x, y: bottom_y, ..tile_id }));
        }

        result
    }

    pub fn tile_bbox(&self, tile_id: TileId) -> GeoRect {
        let count = 2u32.pow(self.z) as f64;

        let left = tile_id.x() as f64 * 360.0 / count - 180.0;
        let top = ((PI * (1.0 - 2.0 * tile_id.y() as f64 / count)).sinh()).atan() * 180.0 / PI;

        let tl = GeoCoord::from_degrees(left, top).unwrap();

        let right = left + (360.0 / count);
        let bottom = if tile_id.y() == 2u32.pow(self.z) {
            -85.05113 // valami okosabbat
        } else {
            ((PI * (1.0 - 2.0 * (tile_id.y() + 1) as f64 / count)).sinh()).atan() * 180.0 / PI
        };

        let br = GeoCoord::from_degrees(right, bottom).unwrap();

        GeoRect::new(tl, br).unwrap()
    }

    pub fn tile_bbox_with_buf(&self, tile_id: TileId, buf: f64) -> GeoRect {
        let count = 2u32.pow(self.z) as f64;
        let abs_count = self.tile_extent as f64 * count;

        // TODO: handle dateline + poles

        let actual_buf = (buf * self.tile_extent as f64) as u32;

        let tl_abs_x = tile_id.x() * self.tile_extent - actual_buf;
        let tl_abs_y = tile_id.y() * self.tile_extent - actual_buf;

        let br_abs_x = (tile_id.x() + 1) * self.tile_extent + actual_buf;
        let br_abs_y = (tile_id.y() + 1) * self.tile_extent + actual_buf;

        let left = 360.0 * (tl_abs_x as f64 / abs_count) - 180.0;
        let top = ((PI * (1.0 - 2.0 * (tl_abs_y as f64 / abs_count))).sinh()).atan() * 180.0 / PI;

        let right = 360.0 * (br_abs_x as f64 / abs_count) - 180.0;
        let bottom = ((PI * (1.0 - 2.0 * (br_abs_y as f64 / abs_count))).sinh()).atan() * 180.0 / PI;

        /*
        let left = tile_id.x() as f64 * 360.0 / count - 180.0;
        let top = ((PI * (1.0 - 2.0 * tile_id.y() as f64 / count)).sinh()).atan() * 180.0 / PI;

        let tl = Coord::from_degrees(left, top).unwrap();

        let right = left + (360.0 / count);
        let bottom = if tile_id.y() == 2u32.pow(self.z) {
            -85.05113 // valami okosabbat
        } else {
            ((PI * (1.0 - 2.0 * (tile_id.y() + 1) as f64 / count)).sinh()).atan() * 180.0 / PI
        };
        */

        let tl = GeoCoord::from_degrees(left, top).unwrap();
        let br = GeoCoord::from_degrees(right, bottom).unwrap();

        GeoRect::new(tl, br).unwrap()
    }

    pub fn region(&self, bbox: &GeoRect) -> std::ops::RangeInclusive<TileId> {
        let tl = self.tile_id(&bbox.top_left());
        let br = self.tile_id(&bbox.bottom_right());

        tl.0 ..= br.0
    }
}

#[cfg(test)]
mod tile_id_tests {
    use super::*;

    #[test]
    fn construction() {
        assert_eq!(TileId::new(0, 2, 1), Err(InvalidTileId));
        assert_eq!(TileId::new(5, 2, 2), Err(InvalidTileId));

        assert!(TileId::new(12, 31, 5).is_ok());
    }

    #[test]
    fn tms_conversion() -> Result<(), InvalidTileId> {
        assert_eq!(TileId::new(0, 2, 1), Err(InvalidTileId));
        assert_eq!(TileId::new(5, 2, 2), Err(InvalidTileId));

        assert_eq!(TileId::new(12, 31, 5)?, TileId::from(TmsTileId::new(12, 0, 5)?));
        assert_eq!(TmsTileId::new(12, 31, 5)?, TmsTileId::from(TileId::new(12, 0, 5)?));

        Ok(())
    }
}
