use bitflags::bitflags;

use crate::coord::GeoCoord;

use std::error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct GeoRect {
    top_left: GeoCoord,
    bottom_right: GeoCoord,
}

impl GeoRect {
    pub fn new(top_left: GeoCoord, bottom_right: GeoCoord) -> Result<Self, InvalidGeoRect> {
        if top_left.lat() < bottom_right.lat() {
            Err(InvalidGeoRect)
        } else {
            Ok(GeoRect { top_left, bottom_right })
        }
    }

    pub fn top_left(&self) -> GeoCoord {
        self.top_left
    }

    pub fn bottom_right(&self) -> GeoCoord {
        self.bottom_right
    }

    pub fn center(&self) -> GeoCoord {
        let lat = (self.top_left.lat() + self.bottom_right.lat()) / 2.0;

        let lon = if self.crosses_dateline() {
            let a = 180.0 - self.top_left.lon();
            let b = (-180.0 - self.bottom_right.lon()).abs();
            
            (a + b) / 2.0 + self.top_left.lon()
        } else {
            (self.top_left.lon() + self.bottom_right.lon()) / 2.0
        };

        GeoCoord::from_degrees(lon, lat).unwrap()
    }

    pub fn crosses_dateline(&self) -> bool {
        self.top_left.lon() > self.bottom_right.lon()
    }

    fn contains_lon(&self, lon: f64) -> bool {
        if !self.crosses_dateline() {
            lon >= self.top_left.lon() && lon <= self.bottom_right.lon()
        } else {
            lon >= self.top_left.lon() || lon <= self.bottom_right.lon()
        }
    }

    pub fn contains_coord(&self, coord: &GeoCoord) -> bool {
        if coord.lat() <= self.top_left.lat() && coord.lat() >= self.bottom_right.lat() {
            self.contains_lon(coord.lon())
        } else {
            false
        }
    }

    pub fn contains_rect(&self, rect: &GeoRect) -> bool {
        if !self.crosses_dateline() && rect.crosses_dateline() {
            if self.top_left.lon() > -180.0 || self.bottom_right.lon() < 180.0 {
                return false;
            }
        }

        self.contains_coord(&rect.top_left) && self.contains_coord(&rect.bottom_right)
    }

    pub fn intersects(&self, rect: &GeoRect) -> bool {
        let tl_lat = self.top_left.lat();
        let br_lat = self.bottom_right.lat();

        if rect.top_left.lat() < br_lat || rect.bottom_right.lat() > tl_lat {
            false
        } else if (tl_lat.abs() == 90.0 && tl_lat == rect.top_left.lat()) || (br_lat.abs() == 90.0 && br_lat == rect.bottom_right.lat()) {
            true
        } else {
            self.contains_lon(rect.top_left.lon()) || self.contains_lon(rect.bottom_right.lon())
        }
    }
}

bitflags! {
    pub struct Edge: u32 {
        const LEFT = 0b00000001;
        const RIGHT = 0b00000010;
        const BOTTOM = 0b00000100;
        const TOP = 0b00001000;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvalidGeoRect;

impl fmt::Display for InvalidGeoRect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid rectangle given")
    }
}

impl error::Error for InvalidGeoRect {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod geo_rect_tests {
    use super::*;

    fn coord(lon: f64, lat: f64) -> GeoCoord {
        GeoCoord::from_degrees(lon, lat).unwrap()
    }

    fn rect(tl: (f64, f64), br: (f64, f64)) -> GeoRect {
        GeoRect::new(coord(tl.0, tl.1), coord(br.0, br.1)).unwrap()
    }

    #[test]
    fn construction() {
        let normal_rect = GeoRect::new(coord(-10.0, 20.0), coord(10.0, -20.0));
        assert!(normal_rect.is_ok());

        let crossing_rect = GeoRect::new(coord(10.0, 20.0), coord(-10.0, -20.0));
        assert!(crossing_rect.is_ok());

        let invalid_rect = GeoRect::new(coord(-10.0, -20.0), coord(10.0, 20.0));
        assert!(invalid_rect.is_err());
    }

    #[test]
    fn center() {
        let normal_rect = rect((-10.0, 20.0), (10.0, -20.0));
        assert_eq!(normal_rect.center(), coord(0.0, 0.0));

        let normal_rect = rect((10.0, 20.0), (20.0, -20.0));
        assert_eq!(normal_rect.center(), coord(15.0, 0.0));

        let crossing_rect = rect((10.0, 20.0), (-10.0, -20.0));
        assert_eq!(crossing_rect.center(), coord(180.0, 0.0));

        let crossing_rect = rect((-10.0, 20.0), (-20.0, -20.0));
        assert_eq!(crossing_rect.center(), coord(165.0, 0.0));
    }

    #[test]
    fn crosses_dateline() {
        let normal_rect = rect((-10.0, 20.0), (10.0, -20.0));
        assert!(!normal_rect.crosses_dateline());

        let crossing_rect = rect((10.0, 20.0), (-10.0, -20.0));
        assert!(crossing_rect.crosses_dateline());
    }

    #[test]
    fn contains_coord() {
        let normal_rect = rect((-10.0, 20.0), (10.0, -20.0));
        assert!(normal_rect.contains_coord(&coord(0.0, 0.0)));
        assert!(!normal_rect.contains_coord(&coord(-20.0, 0.0)));
        assert!(!normal_rect.contains_coord(&coord(0.0, 30.0)));

        let crossing_rect = rect((10.0, 20.0), (-10.0, -20.0));
        assert!(crossing_rect.contains_coord(&coord(20.0, 0.0)));
        assert!(!crossing_rect.contains_coord(&coord(0.0, 0.0)));
    }

    #[test]
    fn contains_rect() {
        let normal_rect_1 = rect((-10.0, 20.0), (10.0, -20.0));
        assert!(normal_rect_1.contains_rect(&normal_rect_1));

        let normal_rect_2 = rect((-5.0, 20.0), (5.0, -20.0));
        assert!(normal_rect_1.contains_rect(&normal_rect_2));

        let normal_rect_3 = rect((10.0, 25.0), (20.0, -15.0));
        assert!(!normal_rect_1.contains_rect(&normal_rect_3));

        let crossing_rect_1 = rect((10.0, 20.0), (-10.0, -20.0));
        assert!(!normal_rect_1.contains_rect(&crossing_rect_1));

        let crossing_rect_2 = rect((20.0, 20.0), (-20.0, -20.0));
        assert!(crossing_rect_1.contains_rect(&crossing_rect_2));

        let normal_rect_4 = rect((-10.0, 15.0), (10.0, -15.0));
        assert!(crossing_rect_1.contains_rect(&normal_rect_4));

        let normal_rect_5 = rect((-180.0, 40.0), (180.0, -40.0));
        assert!(normal_rect_5.contains_rect(&crossing_rect_1));
    }

    #[test]
    fn intersects() {
        let normal_rect_1 = rect((-10.0, 20.0), (10.0, -20.0));
        assert!(normal_rect_1.intersects(&normal_rect_1));

        let normal_rect_2 = rect((-5.0, 20.0), (5.0, -20.0));
        assert!(normal_rect_1.intersects(&normal_rect_2));

        let normal_rect_3 = rect((10.0, 25.0), (20.0, -15.0));
        assert!(normal_rect_1.intersects(&normal_rect_3));

        let crossing_rect_1 = rect((10.0, 20.0), (-10.0, -20.0));
        assert!(normal_rect_1.intersects(&crossing_rect_1));

        let crossing_rect_2 = rect((5.0, 20.0), (-20.0, -20.0));
        assert!(crossing_rect_1.intersects(&crossing_rect_2));

        let normal_rect_4 = rect((-15.0, 15.0), (5.0, -15.0));
        assert!(crossing_rect_1.intersects(&normal_rect_4));

        let normal_rect_5 = rect((-175.0, 40.0), (-170.0, -40.0));
        assert!(!normal_rect_5.intersects(&crossing_rect_1));

        // GeoRects trivially intersect on the poles

        let north_pole_rect_1 = rect((-10.0, 90.0), (10.0, -20.0));
        let north_pole_rect_2 = rect((20.0, 90.0), (30.0, -20.0));
        assert!(north_pole_rect_1.intersects(&north_pole_rect_2));

        let south_pole_rect_1 = rect((-10.0, 20.0), (10.0, -90.0));
        let south_pole_rect_2 = rect((20.0, 20.0), (30.0, -90.0));
        assert!(south_pole_rect_1.intersects(&south_pole_rect_2));
    }
}
