use crate::color::*;
use crate::octree::{TreeBody, VoxelTree};

use brickadia::save as brs;
use cgmath::{Vector3, Vector4};

pub fn simplify(
    octree: &mut VoxelTree<Vector4<u8>>,
    save_data: &mut brs::SaveData,
    bricktype: String,
    match_to_colorset: bool,
) {
    let colorset = convert_colorset_to_hsv(&save_data.header2.colors);

    loop {
        let mut colors = Vec::<Vector4<u8>>::new();
        let x;
        let y;
        let z;
        {
            let (location, voxel) = octree.get_any_mut_or_create();

            x = location[0];
            y = location[1];
            z = location[2];

            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    colors.push(*leaf_color);
                }
                _ => break,
            }
        }

        let mut xp = x + 1;
        let mut yp = y + 1;
        let mut zp = z + 1;

        // Expand z direction first due to octree ordering followed by y and x
        // Ensures blocks are simplified in the pattern of Morton coding
        // Saves us having to check in the negative directions
        while zp - z < 200 {
            let voxel = octree.get_mut_or_create(Vector3::new(x, y, zp));
            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    colors.push(*leaf_color);
                    zp += 1
                }
                _ => break,
            }
        }

        while yp - y < 200 {
            let mut pass = true;
            for sz in z..zp {
                let voxel = octree.get_mut_or_create(Vector3::new(x, yp, sz));
                match voxel {
                    TreeBody::Leaf(leaf_color) => colors.push(*leaf_color),
                    _ => {
                        pass = false;
                        break;
                    }
                }
            }
            if !pass {
                break;
            }
            yp += 1;
        }

        while xp - x < 200 {
            let mut pass = true;
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(xp, sy, sz));
                    match voxel {
                        TreeBody::Leaf(leaf_color) => colors.push(*leaf_color),
                        _ => {
                            pass = false;
                            break;
                        }
                    }
                }
                if !pass {
                    break;
                }
            }
            if !pass {
                break;
            }
            xp += 1;
        }

        // Clear nodes
        // This cant be done during the loops above unless you keep track
        // of which nodes you have already deleted
        for sx in x..xp {
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(sx, sy, sz));
                    *voxel = TreeBody::Empty;
                }
            }
        }

        let avg_color = hsv_average(&colors);
        let color = if match_to_colorset {
            brs::BrickColor::Index(match_hsv_to_colorset(&colorset, &avg_color) as u32)
        } else {
            let rgba = gamma_correct(hsv2rgb(avg_color));
            brs::BrickColor::Unique(brs::Color {
                r: rgba[0],
                g: rgba[1],
                b: rgba[2],
                a: rgba[3],
            })
        };

        let width = xp - x;
        let height = yp - y;
        let depth = zp - z;

        let scales: (isize, isize, isize) = if bricktype == "micro" {
            (1, 1, 1)
        } else {
            (5, 5, 2)
        };

        save_data.bricks.push(brs::Brick {
            asset_name_index: if bricktype == "micro" { 0 } else { 1 },
            // Coordinates are rotated
            size: brs::Size::Procedural(5 * width as u32, 5 * depth as u32, 2 * height as u32),
            position: (
                (scales.0 * width + 2 * scales.0 * x) as i32,
                (scales.1 * depth + 2 * scales.1 * z) as i32,
                (scales.2 * height + 2 * scales.2 * y) as i32,
            ),
            color,
            ..Default::default()
        });
    }
}

pub fn simplify_lossless(
    octree: &mut VoxelTree<Vector4<u8>>,
    save_data: &mut brs::SaveData,
    bricktype: String,
    match_to_colorset: bool,
) {
    let d: isize = 1 << octree.size;
    let len = d + 1;

    let colorset = convert_colorset_to_hsv(&save_data.header2.colors);

    loop {
        let matched_color;
        let unmatched_color;
        let x;
        let y;
        let z;
        {
            let (location, voxel) = octree.get_any_mut_or_create();

            x = location[0];
            y = location[1];
            z = location[2];

            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    matched_color = match_hsv_to_colorset(&colorset, &rgb2hsv(*leaf_color));
                    let final_color = gamma_correct(*leaf_color);
                    unmatched_color = brs::BrickColor::Unique(brs::Color {
                        r: final_color[0],
                        g: final_color[1],
                        b: final_color[2],
                        a: final_color[3],
                    });
                }
                _ => break,
            }
        }

        let mut xp = x + 1;
        let mut yp = y + 1;
        let mut zp = z + 1;

        // Expand z direction first due to octree ordering followed by y
        // Ensures blocks are simplified in the pattern of Morton coding
        while zp < len && (zp - z) < 200 {
            let voxel = octree.get_mut_or_create(Vector3::new(x, y, zp));
            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    let color_temp = match_hsv_to_colorset(&colorset, &rgb2hsv(*leaf_color));
                    if color_temp != matched_color {
                        break;
                    }
                    zp += 1;
                }
                _ => break,
            }
        }

        while yp < len && (yp - y) < 200 {
            let mut pass = true;
            for sz in z..zp {
                let voxel = octree.get_mut_or_create(Vector3::new(x, yp, sz));
                match voxel {
                    TreeBody::Leaf(leaf_color) => {
                        let color_temp = match_hsv_to_colorset(&colorset, &rgb2hsv(*leaf_color));
                        if color_temp != matched_color {
                            pass = false;
                            break;
                        }
                    }
                    _ => {
                        pass = false;
                        break;
                    }
                }
            }
            if !pass {
                break;
            }
            yp += 1;
        }

        while xp < len && (xp - x) < 200 {
            let mut pass = true;
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(xp, sy, sz));
                    match voxel {
                        TreeBody::Leaf(leaf_color) => {
                            let color_temp =
                                match_hsv_to_colorset(&colorset, &rgb2hsv(*leaf_color));
                            if color_temp != matched_color {
                                pass = false;
                                break;
                            }
                        }
                        _ => {
                            pass = false;
                            break;
                        }
                    }
                }
                if !pass {
                    break;
                }
            }
            if !pass {
                break;
            }
            xp += 1;
        }

        // Clear nodes
        // This cant be done during the loops above unless you keep track
        // of which nodes you have already deleted
        for sx in x..xp {
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(sx, sy, sz));
                    *voxel = TreeBody::Empty;
                }
            }
        }

        let width = xp - x;
        let height = yp - y;
        let depth = zp - z;

        let scale = if bricktype == "micro" {
            5
        } else {
            1
        };

        let color = if match_to_colorset {
            brs::BrickColor::Index(matched_color as u32)
        } else {
            unmatched_color
        };

        save_data.bricks.push(brs::Brick {
            asset_name_index: if bricktype == "micro" { 0 } else { 1 },
            // Coordinates are rotated
            size: brs::Size::Procedural(
                (5 * width) as u32 / scale as u32,
                (5 * depth) as u32 / scale as u32,
                (2 * height) as u32 / scale as u32,
            ),
            position: (
                (5 * width + 10 * x) as i32 / scale,
                (5 * depth + 10 * z) as i32 / scale,
                (5 * height + 4 * y) as i32 / scale,
            ),
            color,
            owner_index: 1,
            ..Default::default()
        });
    }
}
