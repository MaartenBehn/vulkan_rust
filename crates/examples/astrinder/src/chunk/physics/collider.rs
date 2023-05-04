use app::glam::{IVec2, ivec2};
use rapier2d::prelude::*;

use crate::{chunk::{CHUNK_PART_SIZE, part::ChunkPart}, math::*};

use super::{Chunk, PhysicsController};


impl PhysicsController {
    pub fn update_collider(&mut self, chunk: &mut Chunk){

        self.collider_set.remove(chunk.collider_handle, &mut self.island_manager, &mut self.rigid_body_set, false);

        fn get_index(x: i32, y: i32) -> usize {
            (x * CHUNK_PART_SIZE + y) as usize
        }        

        struct ColliderMaker {
            corners: [IVec2; 6],
        }

        struct ExpandData {
            corner0: usize,
            corner1: usize,
            corner2: usize,

            offset: IVec2,
            dir0: IVec2,
            dir1: IVec2,
        }

        let expand_data = [
            ExpandData {
                corner0: 1,
                corner1: 2,
                corner2: 3,

                offset: ivec2(-1, 1),
                dir0: ivec2(0, 1),
                dir1: ivec2(1, 0),
            },
            ExpandData {
                corner0: 2,
                corner1: 3,
                corner2: 4,

                offset: ivec2(0, 1),
                dir0: ivec2(1, 0),
                dir1: ivec2(1, -1),
            },
            ExpandData {
                corner0: 3,
                corner1: 4,
                corner2: 5,

                offset: ivec2(1, 0),
                dir0: ivec2(1, -1),
                dir1: ivec2(0, -1),
            },
        ];

        let mut shapes = Vec::new();

        for part in chunk.parts.iter() {
            let part_pos = part_pos_to_chunk(part.pos);

            let mut search_x = 0;
            let mut search_y = 0;

            let mut current_x = 0;
            let mut current_y = 0;

            let mut used_points = vec![false; part.particles.len()];
            fn set_point_used(x: i32, y: i32, used_points: &mut Vec<bool>) {
                used_points[get_index(x, y)] = true;
            }

            fn check_point(x: i32, y: i32, part: &ChunkPart, used_points: &Vec<bool>) -> bool {
                if x < 0 || x >= CHUNK_PART_SIZE || y < 0 || y >= CHUNK_PART_SIZE  {
                    return false;
                }

                let index = get_index(x, y); 
                part.particles[index].material != 0 && !used_points[index]
            }

            let offsets = neigbor_pos_offsets();
            'outer: loop {
                'search: loop {
                    if search_y >= CHUNK_PART_SIZE {
                        break 'outer;
                    }

                    loop {
                        if search_x >= CHUNK_PART_SIZE {
                            search_y += 1;
                            search_x = 0;
                            break;
                        }

                        if check_point(search_x, search_y, part, &used_points) {
                            current_x = search_x;
                            current_y = search_y;
                            search_x += 1;

                            break 'search;
                        }

                        search_x += 1;
                    }
                }

                let mut cm = ColliderMaker { 
                    corners: [ivec2(current_x, current_y); 6],
                };
                set_point_used(current_x, current_y, &mut used_points);

                loop {
                    let mut expaned = false;
                    for (_, data) in expand_data.iter().enumerate() {

                        let corner0 = cm.corners[data.corner0];
                        let corner1 = cm.corners[data.corner1];
                        let corner2 = cm.corners[data.corner2];
        
                        let start = corner0 + data.offset;
                        let middle = corner1 + data.offset;
                        let end = corner2 + data.offset;

                        let mut dir = data.dir0;
                        let mut pos = start;
        
                        let mut points = Vec::new();
                        let expand = loop {
                            if !check_point(pos.x, pos.y, part, &used_points) {
                                break false;
                            }
                            points.push(pos);

                            if pos == middle {
                                dir = data.dir1;
                            }
        
                            if pos == end {
                                break true;
                            }

                            pos += dir;
                        };
        
                        if expand {
                            for point in points {
                                set_point_used(point.x, point.y, &mut used_points);
                            }
                            
                            cm.corners[data.corner0] = start;
                            cm.corners[data.corner1] = middle;
                            cm.corners[data.corner2] = end;

                            expaned = true;
                        }
                    }

                    if !expaned {

                        let mut vertex = Vec::new();
                        for (i, corner) in cm.corners.iter().enumerate() {
                            let pos = hex_to_coord(*corner) + offsets[i] + part_pos;
                            vertex.push(point![pos.x , pos.y])
                        }

                        let shape = SharedShape::convex_hull(&vertex).unwrap();
                        shapes.push((Isometry::default(), shape));

                        break;
                    }
                }
            }
        }

        let compound_collider = ColliderBuilder::compound(shapes);
        chunk.collider_handle = self.collider_set.insert_with_parent(
            compound_collider, chunk.rb_handle, &mut self.rigid_body_set);

        
    }
}

