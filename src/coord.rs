use std::error;
use std::fmt;

/// WGS 84 longitude/latitude pair.
#[derive(Clone, Copy, Debug)]
pub struct GeoCoord {
    lon: f64,
    lat: f64,
}

impl GeoCoord {
    pub fn from_degrees(lon: f64, lat: f64) -> Result<Self, InvalidGeoCoord> {
        if (-180.0..=180.0).contains(&lon) && (-90.0..=90.0).contains(&lat) {
            Ok(Self { lon, lat })
        } else {
            Err(InvalidGeoCoord)
        }
    }

    pub fn from_nanodegrees(lon: i64, lat: i64) -> Result<Self, InvalidGeoCoord> {
        Self::from_degrees(lon as f64 / 1_000_000_000.0, lat as f64 / 1_000_000_000.0)
    }

    pub fn to_nanodegrees(&self) -> (i64, i64) {
        ((self.lon * 1_000_000_000f64).floor() as i64, (self.lat * 1_000_000_000f64).floor() as i64)
    }

    pub fn lon(&self) -> f64 { 
        self.lon 
    }

    pub fn lat(&self) -> f64 { 
        self.lat 
    }
}

impl PartialEq<GeoCoord> for GeoCoord {
    fn eq(&self, other: &GeoCoord) -> bool {
        if self.lon.abs() == 180.0 && other.lon.abs() == 180.0 {
            true
        } else if self.lat == other.lat && self.lat.abs() == 90.0 {
            true
        } else {
            self.lon == other.lon && self.lat == other.lat
        }
    }
}

/// A WGS84 coordinate encoded into two 32-bit integers.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CompactGeoCoord {
    lon: i32,
    lat: i32,
}

impl From<GeoCoord> for CompactGeoCoord {
    fn from(coord: GeoCoord) -> Self {
        Self {
            lon: (coord.lon / 180.0 * (1u64 << 31) as f64).floor() as i32,
            lat: (coord.lat / 90.0 * (1 << 30) as f64).floor() as i32,
        }
    }
}

impl From<CompactGeoCoord> for GeoCoord {
    fn from(coord: CompactGeoCoord) -> Self {
        Self {
            lon: coord.lon as f64 * 180.0 / (1u64 << 31) as f64,
            lat: coord.lat as f64 * 90.0 / (1 << 30) as f64,
        }
    }
}

fn interleave(x: i64, y: i64) -> i64 {
    let mut morton: i64 = 0;
    // TODO: optimize
    for i in 0..32 {
        morton |= (x & 1i64 << i) << i | (y & 1i64 << i) << (i + 1);
    }
    morton
}

impl CompactGeoCoord {
    pub fn morton_code(&self) -> i64 {
        interleave(self.lon as i64, self.lat as i64)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvalidGeoCoord;

impl fmt::Display for InvalidGeoCoord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid coordinate given")
    }
}

impl error::Error for InvalidGeoCoord {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

/// Tile relative "pixel" coordinate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

impl TileCoord {
    pub fn new(x: i32, y: i32) -> TileCoord {
        TileCoord { x, y }
    }

    pub fn diff_to(&self, to: &TileCoord) -> TileCoord {
        TileCoord { x: to.x - self.x, y: to.y - self.y }
    }
}

impl From<(i32, i32)> for TileCoord {
    fn from(pair: (i32, i32)) -> Self {
        TileCoord { x: pair.0, y: pair.1 }
    }
}

impl Into<(i32, i32)> for TileCoord {
    fn into(self) -> (i32, i32) {
        (self.x, self.y)
    }
}

#[cfg(test)]
mod geo_coord_tests {
    use super::*;

    #[test]
    fn construction() {
        assert_eq!(GeoCoord::from_degrees(0.0, 91.0), Err(InvalidGeoCoord));
        assert_eq!(GeoCoord::from_degrees(181.0, 0.0), Err(InvalidGeoCoord));

        assert_eq!(GeoCoord::from_degrees(0.0, std::f64::NAN), Err(InvalidGeoCoord));
        assert_eq!(GeoCoord::from_degrees(0.0, std::f64::INFINITY), Err(InvalidGeoCoord));

        assert!(GeoCoord::from_degrees(2.2945, 48.858222).is_ok());
    }

    #[test]
    fn equality() -> Result<(), InvalidGeoCoord> {
        let dateline_a = GeoCoord::from_degrees(-180.0, 0.0)?;
        let dateline_b = GeoCoord::from_degrees(180.0, 0.0)?;
        assert_eq!(dateline_a, dateline_b);

        let north_pole_a = GeoCoord::from_degrees(-80.0, 90.0)?;
        let north_pole_b = GeoCoord::from_degrees(80.0, 90.0)?;
        assert_eq!(north_pole_a, north_pole_b);

        let north_pole = GeoCoord::from_degrees(-80.0, 90.0)?;
        let south_pole = GeoCoord::from_degrees(80.0, -90.0)?;
        assert_ne!(north_pole, south_pole);

        Ok(())
    }

    #[test]
    fn encoding() {
        let raw_coord = GeoCoord::from_degrees(2.2945, 48.858222).unwrap();

        let encoded_coord = CompactGeoCoord::from(raw_coord);
        assert_eq!(encoded_coord, CompactGeoCoord { lon: 27374451, lat: 582901293 });
        assert_eq!(encoded_coord.morton_code(), 579221254078012839);

        let decoded_coord = GeoCoord::from(encoded_coord);
        assert_eq!(decoded_coord, GeoCoord { lon: 2.2944999765604734, lat: 48.858221964910626 });
    }
}
