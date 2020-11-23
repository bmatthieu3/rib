type Position = na::Point3<f32>;
type Normal = na::Vector3<f32>;
type Texcoord = na::Point2<f32>;
type Index = u32;
type Weight = f32;
type BoneIdx = i32;

use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize)]
#[derive(PartialEq)]
pub struct Vertices {
    pub positions: Vec<Position>,
    pub normals: Vec<Normal>,
    pub texcoords: Vec<Texcoord>,

    // Used for animation purposes
    // The weights of each bone for each vertices. Of size NVertices x 2
    pub weights: Option<Vec<[Weight; 2]>>,
    // The 2 bones that influences the vertices the much. Of size NVertices x 2
    pub bone_ids: Option<Vec<[BoneIdx; 2]>>,

    pub indices: Vec<Index>,
}