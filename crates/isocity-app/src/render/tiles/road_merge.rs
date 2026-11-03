//! Detects runs of parallel road tiles so they can be drawn as one wide avenue
//! or highway.

use super::map::TileMap;

// ---------------------------------------------------------------------------
// Road merge analysis (avenues & highways)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoadKind {
    Single,
    Avenue,
    Highway,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoadOrient {
    Ns,
    Ew,
    Intersection,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Left,
    Right,
    Center,
    Single,
}

#[derive(Clone, Copy, Debug)]
pub struct MergeInfo {
    pub kind: RoadKind,
    pub orient: RoadOrient,
    pub has_median: bool,
    pub plants: bool,
    pub position: i32,
    pub width: i32,
    pub side: Side,
}

pub fn analyze_merged_road(m: &TileMap, x: i32, y: i32) -> MergeInfo {
    let single = MergeInfo {
        kind: RoadKind::Single,
        orient: RoadOrient::Intersection,
        has_median: false,
        plants: false,
        position: 0,
        width: 1,
        side: Side::Single,
    };
    if !m.has_road(x, y) {
        return single;
    }
    let (n, e, s, w) = (m.has_road(x - 1, y), m.has_road(x, y - 1), m.has_road(x + 1, y), m.has_road(x, y + 1));
    let count = n as u8 + e as u8 + s as u8 + w as u8;
    if count >= 3 {
        return MergeInfo { side: Side::Center, ..single };
    }
    let ns = n || s;
    let ew = e || w;
    let build = |orient, before: i32, after: i32, first_side: Side, last_side: Side| {
        let width = before + after + 1;
        let position = before;
        let side = if width > 1 {
            if position == 0 {
                first_side
            } else if position == width - 1 {
                last_side
            } else {
                Side::Center
            }
        } else {
            Side::Single
        };
        MergeInfo {
            kind: if width >= 4 {
                RoadKind::Highway
            } else if width >= 2 {
                RoadKind::Avenue
            } else {
                RoadKind::Single
            },
            orient,
            has_median: width >= 2,
            plants: width >= 3,
            position,
            width,
            side,
        }
    };
    if ns && !ew {
        let count_dir = |dy: i32| {
            let mut c = 0;
            for d in 1..=3 {
                let yy = y + dy * d;
                if m.has_road(x, yy) && (m.has_road(x - 1, yy) || m.has_road(x + 1, yy)) {
                    c += 1;
                } else {
                    break;
                }
            }
            c
        };
        let west = count_dir(1);
        let east = count_dir(-1);
        return build(RoadOrient::Ns, east, west, Side::Right, Side::Left);
    }
    if ew && !ns {
        let count_dir = |dx: i32| {
            let mut c = 0;
            for d in 1..=3 {
                let xx = x + dx * d;
                if m.has_road(xx, y) && (m.has_road(xx, y - 1) || m.has_road(xx, y + 1)) {
                    c += 1;
                } else {
                    break;
                }
            }
            c
        };
        let north = count_dir(-1);
        let south = count_dir(1);
        return build(RoadOrient::Ew, north, south, Side::Left, Side::Right);
    }
    MergeInfo {
        orient: if count >= 2 { RoadOrient::Intersection } else { RoadOrient::Ns },
        ..single
    }
}
