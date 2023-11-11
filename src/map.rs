use bevy::{ecs::query, prelude::*};
use std::io::BufReader;
use std::path::Path;
use std::{fmt, fs::File};
use tokio::task::spawn_blocking;

use serde::{Deserialize, Serialize};

use tiled::{parse, LayerData};

use pathfinding::prelude::astar;

use crate::game::{PlayerId, Position};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let map = Map::load_map();
        app.insert_resource(map);
    }
}

pub const WIDTH: i32 = 60;
pub const HEIGHT: i32 = 50;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum TileType {
    Grasslands,
    Snow,
    River,
    Ocean,
    Plains,
    HillsPlains,
    Desert,
    Oasis,
    HillsDesert,
    HillsGrasslands,
    Swamp,
    HillsSnow,
    DeciduousForest,
    Rainforest,
    Jungle,
    Savanna,
    FrozenForest,
    PineForest,
    PalmForest,
    Mountain,
    Volcano,
    Unknown,
}

impl fmt::Display for TileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TileType::Grasslands => write!(f, "Grasslands"),
            TileType::Snow => write!(f, "Snow"),
            TileType::River => write!(f, "River"),
            TileType::Ocean => write!(f, "Ocean"),
            TileType::Plains => write!(f, "Plains"),
            TileType::HillsPlains => write!(f, "Plains Hills"),
            TileType::Desert => write!(f, "Desert"),
            TileType::Oasis => write!(f, "Oasis"),
            TileType::HillsDesert => write!(f, "Desert Hills"),
            TileType::HillsGrasslands => write!(f, "Grassland Hills"),
            TileType::Swamp => write!(f, "Swamp"),
            TileType::HillsSnow => write!(f, "Snow Hills"),
            TileType::DeciduousForest => write!(f, "Deciduous Forest"),
            TileType::Rainforest => write!(f, "Rain Forest"),
            TileType::Jungle => write!(f, "Jungle"),
            TileType::Savanna => write!(f, "Savanna"),
            TileType::FrozenForest => write!(f, "Frozen Forest"),
            TileType::PineForest => write!(f, "Pine Forest"),
            TileType::PalmForest => write!(f, "Palm Forest"),
            TileType::Mountain => write!(f, "Mountain"),
            TileType::Volcano => write!(f, "Volcano"),
            TileType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileInfo {
    pub tile_type: TileType,
    pub layers: Vec<u32>,
}

#[derive(Resource, Debug, Clone)]
pub struct Map {
    pub width: i32,
    pub height: i32,
    pub base: Vec<TileInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MapTile {
    pub x: i32,
    pub y: i32,
    pub t: Vec<u32>,
}

// TODO move to another module
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MapPos(pub i32, pub i32);

impl MapPos {
    fn distance(&self, other: &MapPos) -> u32 {
        // (self.0.abs_diff(other.0) + self.1.abs_diff(other.1)) as u32
        Map::distance((self.0, self.1), (other.0, other.1))
    }

    fn successors(&self, map: &Map, blocking_list: &Vec<MapPos>, landwalk: bool, waterwalk: bool, mountainwalk:bool ) -> Vec<(MapPos, u32)> {
        let &MapPos(x, y) = self;
        let s = Map::get_neighbour_tiles(x, y, map, blocking_list, landwalk, waterwalk, mountainwalk);
        s
    }
}

impl Map {
    pub fn load_map() -> Map {
        let mut map = Map {
            width: WIDTH,
            height: HEIGHT,
            base: Vec::with_capacity(3000),
        };

        let file = File::open(&Path::new("test3.tmx")).unwrap();
        println!("Opened file");
        let reader = BufReader::new(file);
        let raw_map = parse(reader).unwrap();

        for layer in raw_map.layers.iter() {
            println!("layer name: {}", layer.name);

            if layer.name == "base1" {
                if let LayerData::Finite(layer_tiles) = &layer.tiles {
                    for row in layer_tiles.iter() {
                        for col in row.iter() {
                            let tile = TileInfo {
                                tile_type: Map::gid_to_tiletype(col.gid),
                                layers: vec![col.gid],
                            };

                            map.base.push(tile);
                        }
                    }
                }
            } else if layer.name == "base2" {
                let mut index = 0;

                if let LayerData::Finite(layer_tiles) = &layer.tiles {
                    for row in layer_tiles.iter() {
                        for col in row.iter() {
                            //Do not store if tile is 0
                            if col.gid != 0 {
                                map.base[index].tile_type = Map::gid_to_tiletype(col.gid);
                                map.base[index].layers.push(col.gid);
                            }

                            index += 1;
                        }
                    }
                }
            }
        }
        map
    }

    pub fn pos_to_tuple(pos: Position) -> (i32, i32) {
        (pos.x, pos.y)
    }

    pub fn pos_to_index(x: i32, y: i32) -> usize {
        let tile_index: usize = (y as usize) * (WIDTH as usize) + (x as usize);
        return tile_index;
    }

    pub fn index_to_pos(index: usize) -> MapPos {
        let x = (index as i32) % WIDTH;
        let y = (index as i32) / WIDTH;

        return MapPos(x, y);
    }

    pub fn find_path(
        src_pos: Position,
        dst_pos: Position,
        map: &Map,
        blocking_list: &Vec<MapPos>,
        landwalk: bool,
        waterwalk: bool,
        mountainwalk: bool,
    ) -> Option<(Vec<MapPos>, u32)> {
        let goal: MapPos = MapPos(dst_pos.x, dst_pos.y);
        let result = astar(
            &MapPos(src_pos.x, src_pos.y),
            |p| p.successors(&map, &blocking_list, landwalk, waterwalk, mountainwalk),
            |p| p.distance(&goal),
            |p| *p == goal,
        );

        return result;
    }

    pub fn get_tiles_by_range(center_x: i32, center_y: i32, r: u32, map: Map) -> Vec<MapTile> {
        let mut tiles = Vec::new();
        let neighbours = Map::range((center_x, center_y), r);

        for neighbour in neighbours {
            let (x, y) = neighbour;

            //Reminder tile_index = y * width + x
            let tile_index: usize = (y as usize) * (WIDTH as usize) + (x as usize);
            let layers = map.base[tile_index].layers.clone();

            let tile = MapTile {
                x: x,
                y: y,
                t: layers,
            };

            tiles.push(tile);
        }

        tiles
    }

    pub fn pos_to_tiles(positions: &Vec<(i32, i32)>, map: &Map) -> Vec<MapTile> {
        let mut tiles = Vec::new();

        for pos in positions.iter() {
            let (x, y) = pos;

            //Reminder tile_index = y * width + x
            let tile_index = *y * WIDTH + *x;
            let tile_index_usize = tile_index as usize;
            let layers = map.base[tile_index_usize].layers.clone();

            let tile = MapTile {
                x: *x,
                y: *y,
                t: layers,
            };

            tiles.push(tile);
        }

        tiles
    }

    pub fn is_passable_by_obj(
        x: i32,
        y: i32,
        landwalk: bool,
        waterwalk: bool,
        mountainwalk: bool,
        map: &Map,
    ) -> bool {
        let tile_index = y * WIDTH + x;
        let tile_index_usize = tile_index as usize;
        //let layers = map.base[tile_index_usize].layers.clone();
        let tile_type = map.base[tile_index_usize].tile_type.clone();

        let passable = match (tile_type, landwalk, waterwalk, mountainwalk) {
            (TileType::Ocean, _, true, _) => true,
            (TileType::Ocean, _, false, _) => false,
            (TileType::River, _, true, _) => true,
            (TileType::River, _, false, _) => false,
            (TileType::Mountain, _, _, true) => true,
            (TileType::Mountain, _, _, false) => false,
            (_, false, _, _) => false,
            _ => true,
        };

        return passable;
    }

    pub fn is_passable(x: i32, y: i32, map: &Map) -> bool {
        let tile_index = y * WIDTH + x;
        let tile_index_usize = tile_index as usize;
        //let layers = map.base[tile_index_usize].layers.clone();
        let tile_type = map.base[tile_index_usize].tile_type.clone();

        let passable = match tile_type {
            TileType::Ocean => false,
            TileType::River => false,
            TileType::Mountain => false,
            _ => true,
        };

        return passable;
    }

    pub fn tile_type(x: i32, y: i32, map: &Map) -> TileType {
        let tile_index = y * WIDTH + x;
        let tile_index_usize = tile_index as usize;
        //let layers = map.base[tile_index_usize].layers.clone();
        let tile_type = map.base[tile_index_usize].tile_type.clone();

        return tile_type;
    }

    pub fn tile_name(tile_type: TileType) -> String {
        return tile_type.to_string();
    }

    pub fn movement_cost(tile_type: TileType) -> i32 {
        let movement_cost = match tile_type {
            TileType::Mountain => 5,
            TileType::HillsPlains => 3,
            TileType::HillsGrasslands => 3,
            TileType::HillsSnow => 3,
            TileType::HillsDesert => 3,
            TileType::DeciduousForest => 3,
            TileType::River => 6,
            _ => 1,
        };

        return movement_cost;
    }

    pub fn def_bonus(tile_type: TileType) -> f32 {
        let def_bonus = match tile_type {
            TileType::HillsPlains => 0.33,
            TileType::HillsGrasslands => 0.33,
            TileType::HillsSnow => 0.33,
            TileType::HillsDesert => 0.33,
            TileType::DeciduousForest => 0.5,
            TileType::PineForest => 0.5,
            TileType::FrozenForest => 0.5,
            TileType::Jungle => 0.75,
            TileType::Swamp => 0.66,
            _ => 0.0,
        };

        return def_bonus;
    }

    fn gid_to_tiletype(gid: u32) -> TileType {
        match gid {
            1 => TileType::Grasslands,
            2 => TileType::Snow,
            3 => TileType::River,
            4 => TileType::River,
            5 => TileType::Ocean,
            6 => TileType::Plains,
            7 => TileType::HillsPlains,
            8 => TileType::HillsPlains,
            9 => TileType::Plains,
            10 => TileType::Desert,
            11 => TileType::Oasis,
            12 => TileType::HillsDesert,
            13 => TileType::HillsGrasslands,
            14 => TileType::Swamp,
            15 => TileType::Swamp,
            16 => TileType::HillsSnow,
            17 => TileType::Ocean,
            18 => TileType::Swamp,
            19 => TileType::DeciduousForest,
            20 => TileType::Rainforest,
            21 => TileType::Jungle,
            22 => TileType::Savanna,
            23 => TileType::DeciduousForest,
            24 => TileType::DeciduousForest,
            25 => TileType::FrozenForest,
            26 => TileType::FrozenForest,
            27 => TileType::PineForest,
            28 => TileType::FrozenForest,
            29 => TileType::Savanna,
            30 => TileType::PalmForest,
            31 => TileType::Jungle,
            32 => TileType::Mountain,
            33 => TileType::Mountain,
            34 => TileType::Mountain,
            35 => TileType::Mountain,
            36 => TileType::Mountain,
            37 => TileType::Mountain,
            38 => TileType::Mountain,
            39 => TileType::Volcano,
            _ => TileType::Unknown,
        }
    }

    fn odd_q_to_cube((q, r): (i32, i32)) -> (i32, i32, i32) {
        let x = q;
        let z = r - (q - (q & 1)) / 2;
        let y = -x - z;
        (x, y, z)
    }

    fn cube_to_odd_q((x, y, z): (i32, i32, i32)) -> (i32, i32) {
        let q = x;
        let r = z + (x - (x & 1)) / 2;
        (q, r)
    }

    pub fn distance(src_pos: (i32, i32), dst_pos: (i32, i32)) -> u32 {
        let (sx, sy, sz) = Map::odd_q_to_cube(src_pos);
        let (dx, dy, dz) = Map::odd_q_to_cube(dst_pos);

        let distance = (((sx - dx).abs() + (sy - dy).abs() + (sz - dz).abs()) / 2) as u32;

        distance
    }

    pub fn dist(src_pos: Position, dst_pos: Position) -> u32 {
        let src_pos_tuple = (src_pos.x, src_pos.y);
        let dst_pos_tuple = (dst_pos.x, dst_pos.y);

        let (sx, sy, sz) = Map::odd_q_to_cube(src_pos_tuple);
        let (dx, dy, dz) = Map::odd_q_to_cube(dst_pos_tuple);

        let distance = (((sx - dx).abs() + (sy - dy).abs() + (sz - dz).abs()) / 2) as u32;

        distance
    }

    fn cube_direction(direction: i32) -> (i32, i32, i32) {
        let neighbours_table: Vec<(i32, i32, i32)> = vec![
            (1, -1, 0),
            (1, 0, -1),
            (0, 1, -1),
            (-1, 1, 0),
            (-1, 0, 1),
            (0, -1, 1),
        ];

        return neighbours_table[direction as usize];
    }

    fn cube_neighbour(cube: (i32, i32, i32), direction: i32) -> (i32, i32, i32) {
        return Map::cube_add(cube, Map::cube_direction(direction));
    }

    fn cube_add(cube1: (i32, i32, i32), cube2: (i32, i32, i32)) -> (i32, i32, i32) {
        return (cube1.0 + cube2.0, cube1.1 + cube2.1, cube1.2 + cube2.2);
    }

    fn cube_scale(cube: (i32, i32, i32), factor: i32) -> (i32, i32, i32) {
        return (cube.0 * factor, cube.1 * factor, cube.2 * factor);
    }

    pub fn ring((q, r): (i32, i32), radius: i32) -> Vec<(i32, i32)> {
        let mut results: Vec<(i32, i32)> = Vec::new();

        let center = Map::odd_q_to_cube((q, r));

        let mut hex = Map::cube_add(center, Map::cube_scale(Map::cube_direction(4), radius));

        for i in 0..6 {
            for _j in 0..radius {
                results.push(Map::cube_to_odd_q(hex));
                hex = Map::cube_neighbour(hex, i);
            }
        }

        return results;
    }

    pub fn range((q, r): (i32, i32), num: u32) -> Vec<(i32, i32)> {
        let n = num as i32;

        let mut result: Vec<(i32, i32)> = Vec::new();

        let (cx, cy, cz) = Map::odd_q_to_cube((q, r));

        //TODO could be optimized as per Amit's hex guide
        for sx in -n..=n {
            for sy in -n..=n {
                for sz in -n..=n {
                    if (cx + sx) + (cy + sy) + (cz + sz) == 0 {
                        let pos = Map::cube_to_odd_q(((cx + sx), (cy + sy), (cz + sz)));

                        if Map::is_valid_pos(pos) {
                            result.push(pos);
                        }
                    }
                }
            }
        }

        result
    }

    pub fn get_neighbour_tiles(
        origin_x: i32,
        origin_y: i32,
        map: &Map,
        blocking_list: &Vec<MapPos>,
        landwalk: bool,
        waterwalk: bool,
        mountainwalk: bool,
    ) -> Vec<(MapPos, u32)> {
        let neighbours_table: Vec<(i32, i32, i32)> = vec![
            (1, -1, 0),
            (1, 0, -1),
            (0, 1, -1),
            (-1, 1, 0),
            (-1, 0, 1),
            (0, -1, 1),
        ];

        let mut result: Vec<(MapPos, u32)> = Vec::new();

        let (x, y, z) = Map::odd_q_to_cube((origin_x, origin_y));

        for (nx, ny, nz) in neighbours_table {
            let neighbour_cube = (x + nx, y + ny, z + nz);
            let neighbour = Map::cube_to_odd_q(neighbour_cube);
            let neighbour_x = neighbour.0;
            let neighbour_y = neighbour.1;

            // Skip coordinates out of bounds
            if neighbour_x < 0 || neighbour_y < 0 || neighbour_x >= WIDTH || neighbour_y >= HEIGHT {
                continue;
            }

            //Reminder tile_index = y * width + x
            //debug!("neighbour_y: {:?} neighbour_x: {:?}", neighbour_y, neighbour_y);
            let tile_index = neighbour_y * WIDTH + neighbour_x;
            let tile_index_usize = tile_index as usize;
            let tile_type = map.base[tile_index_usize].tile_type.clone();

            let movement_cost = Map::movement_cost(tile_type) as u32;

            if Map::is_valid_pos(neighbour)
                && Map::is_passable_by_obj(
                    neighbour_x,
                    neighbour_y,
                    landwalk,
                    waterwalk,
                    mountainwalk,
                    map,
                )
                && Map::is_not_blocked(neighbour, blocking_list)
            {
                result.push((MapPos(neighbour_x, neighbour_y), movement_cost));
            }
        }

        return result;
    }

    fn is_not_blocked((x, y): (i32, i32), blocking_list: &Vec<MapPos>) -> bool {
        for block_pos in blocking_list {
            if x == block_pos.0 && y == block_pos.1 {
                return false;
            }
        }

        return true;
    }

    fn neighbours((q, r): (i32, i32)) -> Vec<(i32, i32)> {
        let neighbours_table: Vec<(i32, i32, i32)> = vec![
            (1, -1, 0),
            (1, 0, -1),
            (0, 1, -1),
            (-1, 1, 0),
            (-1, 0, 1),
            (0, -1, 1),
        ];

        let mut result: Vec<(i32, i32)> = Vec::new();

        let (x, y, z) = Map::odd_q_to_cube((q, r));

        for (nx, ny, nz) in neighbours_table {
            let neighbour_cube = (x + nx, y + ny, z + nz);
            let neighbour_odd = Map::cube_to_odd_q(neighbour_cube);

            if Map::is_valid_pos(neighbour_odd) {
                result.push(neighbour_odd);
            }
        }

        result
    }

    pub fn is_adjacent(source_pos: Position, target_pos: Position) -> bool {
        let neighbours = Self::neighbours((source_pos.x, source_pos.y));

        // Experimenting with treating the source position as adjacent
        if source_pos == target_pos {
            return true;
        }

        for (x, y) in neighbours {
            if x == target_pos.x && y == target_pos.y {
                return true;
            }
        }

        return false;
    }

    pub fn is_valid_pos((q, r): (i32, i32)) -> bool {
        q >= 0 && r >= 0 && q < (WIDTH as i32) && r < (HEIGHT as i32)
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_load_map() {
        let map: Map = Map::load_map();

        let tile_index: usize = (35 * WIDTH + 17).try_into().unwrap();
        assert_eq!(map.base[tile_index].layers, vec![13]);
        assert_eq!(map.base[tile_index].tile_type, TileType::HillsGrasslands);
    }

    #[test]
    fn test_get_tiles_by_range() {
        let map: Map = Map::load_map();

        let mut tiles = Map::get_tiles_by_range(16, 36, 2, map);

        let test_tiles = r#"[{"t":[13],"x":17,"y":34},{"t":[1],"x":16,"y":37},{"t":[1],"x":16,"y":35},{"t":[13],"x":18,"y":36},{"t":[5],"x":15,"y":36},{"t":[1],"x":17,"y":37},{"t":[1],"x":15,"y":34},{"t":[5],"x":14,"y":37},{"t":[13],"x":17,"y":35},{"t":[1],"x":16,"y":38},{"t":[1],"x":14,"y":35},{"t":[1],"x":16,"y":36},{"t":[1],"x":18,"y":37},{"t":[13],"x":16,"y":34},{"t":[5],"x":15,"y":37},{"t":[1],"x":18,"y":35},{"t":[13],"x":15,"y":35},{"t":[1],"x":17,"y":36},{"t":[13],"x":14,"y":36}]"#;

        let deserialized_test_tiles: Vec<MapTile> = serde_json::from_str(&test_tiles).unwrap();

        println!("{:?}", deserialized_test_tiles);

        for tile in tiles {
            assert_eq!(deserialized_test_tiles.contains(&tile), true);
        }
    }

    #[test]
    fn test_odd_q_to_cube() {
        let odd_q = (2, 1);

        assert_eq!(Map::odd_q_to_cube(odd_q), (2, -2, 0));

        let odd_q = (-1, 2);
        assert_eq!(Map::odd_q_to_cube(odd_q), (-1, -2, 3));
    }

    #[test]
    fn test_cube_to_odd_q() {
        let cube = (2, -2, 0);

        assert_eq!(Map::cube_to_odd_q(cube), (2, 1));

        let cube = (-1, -2, 3);
        assert_eq!(Map::cube_to_odd_q(cube), (-1, 2));
    }

    #[test]
    fn test_distance() {
        let src_pos = (2, 4);
        let dst_pos = (5, 3);

        assert_eq!(Map::distance(src_pos, dst_pos), 3);

        let src_pos = (2, 4);
        let dst_pos = (5, 5);

        assert_eq!(Map::distance(src_pos, dst_pos), 3);

        let src_pos = (2, 4);
        let dst_pos = (5, 6);

        assert_eq!(Map::distance(src_pos, dst_pos), 4);

        let src_pos = (-2, 4);
        let dst_pos = (5, 6);
        assert_eq!(Map::distance(src_pos, dst_pos), 7);
    }

    #[test]
    fn test_range() {
        //[{2,3},{2,2},{3,3},{3,2},{3,1},{4,3},{4,2}]
        let result = vec![(2, 3), (2, 2), (3, 3), (3, 2), (3, 1), (4, 3), (4, 2)];

        assert_eq!(Map::range((3, 2), 1), result);

        //[{0,2},{0,1},{0,0},{1,1},{1,0}]
        let result = vec![(0, 2), (0, 1), (0, 0), (1, 1), (1, 0)];

        assert_eq!(Map::range((0, 1), 1), result);
    }

    #[test]
    fn test_neighbours() {
        //[{1,2},{0,2},{0,1},{1,0},{2,1},{2,2}]
        let result = vec![(2, 2), (2, 1), (1, 0), (0, 1), (0, 2), (1, 2)];

        assert_eq!(Map::neighbours((1, 1)), result);

        //[{0,1},{1,0}]
        let result = vec![(1, 0), (0, 1)];

        assert_eq!(Map::neighbours((0, 0)), result);
    }
}
