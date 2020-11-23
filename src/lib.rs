extern crate nalgebra as na;

mod transform;
mod skeleton;
mod animation;
mod utils;
mod vertices;

pub use vertices::Vertices;
pub use animation::Animations;

use na::{Point3, Vector3, Point2};
use std::io;
use std::path::Path;
#[derive(Debug)]
pub enum Error {
    OpenFile {
        path: String,
    },
    EmptyFile,
    PrimitiveNotTriangles,
    SkeletonNotEqual,
    VerticesNotEqual,
    IoError(io::Error),
    Serialize(Box<bincode::ErrorKind>),
    Deserialize(Box<bincode::ErrorKind>)
}

impl<'a> From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}
use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize)]
pub struct Data {
    pub vertices: Vertices,
    pub animations: Option<Animations>,
}

pub fn load<'a, P: AsRef<Path> + std::fmt::Debug + 'a>(dirname: &'a P, fps: f32) -> Result<Data, Error> {
    let filenames = dirname.as_ref().read_dir()?
        .into_iter()
        .filter_map(|p| {
            if let Ok(entry) = p {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "dae" {
                        Some(path)
                    } else {
                        None
                    }
                } else {
                    // Discard files having bad extensions
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let docs: Result<Vec<_>, _> = filenames.iter().map(|f| {
            collada::document::ColladaDocument::from_path(f)
                .map_err(|_| {
                    Error::OpenFile {
                        path: f.to_str().unwrap().to_owned()
                    }
                })
        }).collect();
    let docs = docs?;
    let frame_time = 1.0/fps;

    let res: Result<Vec<_>, _> = docs.into_iter().zip(filenames.iter())
        .map(|(doc, filename)| {
            parse_collada_doc(filename, doc, frame_time)
        }).collect();
    let mut data = res?;

    // Check wheter the vertices and skeleton between files are equal
    for (cur, next) in data.iter().zip(data.iter().skip(1).cycle()) {
        if cur.vertices != next.vertices {
            return Err(Error::VerticesNotEqual);
        }

        if let Some(cur_anims) = &cur.animations {
            if let Some(next_anims) = &next.animations {
                // The two consecutive files contain animations,
                // so we check if their skeletons are equal
                if cur_anims.get_skeleton() != next_anims.get_skeleton() {
                    return Err(Error::SkeletonNotEqual);
                }
            } else {
                return Err(Error::SkeletonNotEqual);
            }
        } else if next.animations.is_some() {
            return Err(Error::SkeletonNotEqual);
        }
    }
    // At this point, either:
    // - same vertices shared by the files, same skeleton, different animations
    // - same vertices shared by the files, no animations (static mesh)

    // The vertices and skeleton correspond
    // Therefore we can append the animations
    let Data { vertices, animations} = data.remove(0);
    if let Some(mut animations) = animations {
        
        for d in data.into_iter() {
            animations.append(d.animations.unwrap());
        }
    
        let data = Data {
            animations: Some(animations),
            vertices
        };
    
        Ok(data)
    } else {
        // All the files do not contain any animations
        // and contain the same vertices
        Ok(Data {
            vertices,
            animations: None
        })
    }
}

fn parse_collada_doc<'a, P: AsRef<Path> + std::fmt::Debug + 'a>(path: &'a P, doc: collada::document::ColladaDocument, frame_time: f32) -> Result<Data, Error> {
    if let Some(obj_set) = doc.get_obj_set() {
        let object = obj_set.objects.first().ok_or(Error::EmptyFile)?;

        let p = &object.vertices;
        let n = &object.normals;
        let t = &object.tex_vertices;

        let mut positions = vec![];
        let mut normals = vec![];
        let mut texcoords = vec![];
        let mut indices = vec![];
        let mut weights = None;
        let mut bone_ids = None;

        let (w, b) = if doc.get_animations().is_some() {
            // If the skeleton is defined, therefore there is a bind data associated to it
            // We can unwrap to get this
            let bind_data_set = doc.get_bind_data_set().unwrap();
            let bind_data = bind_data_set.bind_data.first().unwrap();
            let mut w: Vec<[f32; 2]> = vec![[0.0; 2]; p.len()];
            let mut b: Vec<[i32; 2]> = vec![[0; 2]; p.len()];

            let mut cur_idx_weights: Vec<usize> = vec![0; p.len()];

            // Retrieve the inverse bind poses of that object
            for collada::VertexWeight { vertex, joint, weight } in &bind_data.vertex_weights {
                let cur_idx = &mut cur_idx_weights[*vertex];

                let weight = bind_data.weights[*weight];
                w[*vertex][*cur_idx] = weight;
                b[*vertex][*cur_idx] = *joint as i32;

                *cur_idx += 1;
            }

            /*for wi in w.iter() {
                assert!((wi.iter().sum::<f32>() - 1.0).abs() < 1e-3);
            }*/
            // Initialize the final vertex weights and bones
            weights = Some(vec![]);
            bone_ids = Some(vec![]);

            (w, b)
        } else {
            (vec![], vec![])
        };          

        for geometry in &object.geometry {
            for primitive in &geometry.mesh {
                match primitive {
                    collada::PrimitiveElement::Triangles(triangles) => {
                        let normals_idx = triangles.normals.as_ref().unwrap();
                        let texcoords_idx = triangles.tex_vertices.as_ref().unwrap();
                        let positions_idx = &triangles.vertices;
                        assert_eq!(positions_idx.len(), normals_idx.len());
                        assert_eq!(positions_idx.len(), texcoords_idx.len());

                        let mut idx = 0;
                        for (&vertex_idx, (&normal_idx, &tx_idx))  in positions_idx.iter().zip(normals_idx.iter().zip(texcoords_idx.iter())) {
                            positions.push(Point3::new(p[vertex_idx.0].x as f32, p[vertex_idx.0].y as f32, p[vertex_idx.0].z as f32));
                            positions.push(Point3::new(p[vertex_idx.1].x as f32, p[vertex_idx.1].y as f32, p[vertex_idx.1].z as f32));
                            positions.push(Point3::new(p[vertex_idx.2].x as f32, p[vertex_idx.2].y as f32, p[vertex_idx.2].z as f32));
                            normals.push(Vector3::new(n[normal_idx.0].x as f32, n[normal_idx.0].y as f32, n[normal_idx.0].z as f32));
                            normals.push(Vector3::new(n[normal_idx.1].x as f32, n[normal_idx.1].y as f32, n[normal_idx.1].z as f32));
                            normals.push(Vector3::new(n[normal_idx.2].x as f32, n[normal_idx.2].y as f32, n[normal_idx.2].z as f32));
                            texcoords.push(Point2::new(t[tx_idx.0].x as f32, t[tx_idx.0].y as f32));
                            texcoords.push(Point2::new(t[tx_idx.1].x as f32, t[tx_idx.1].y as f32));
                            texcoords.push(Point2::new(t[tx_idx.2].x as f32, t[tx_idx.2].y as f32));

                            if let Some(weights) = &mut weights {
                                weights.push(w[vertex_idx.0]);
                                weights.push(w[vertex_idx.1]);
                                weights.push(w[vertex_idx.2]);
                            }
                            if let Some(bone_ids) = &mut bone_ids {
                                bone_ids.push(b[vertex_idx.0]);
                                bone_ids.push(b[vertex_idx.1]);
                                bone_ids.push(b[vertex_idx.2]);
                            }

                            indices.extend([idx, idx + 1, idx + 2].iter());
                            idx += 3;
                        }
                    },
                    _ => {
                        return Err(Error::PrimitiveNotTriangles)
                    }
                }
            }
        }

        let vertices = Vertices {
            positions,
            normals,
            texcoords,
            bone_ids,
            weights,
            indices
        };

        if let Some(name) = path.as_ref().file_stem() {
            let animations = Animations::new(name.to_str().unwrap(), &doc, frame_time);
            Ok(Data { vertices, animations })
        } else {
            Err(Error::EmptyFile)
        }
    } else {
        Err(Error::EmptyFile)
    }
}

pub fn write<P: AsRef<Path>>(data: &Data, path: P) -> Result<(), Error> {
    let mut buffer = BufWriter::new(File::create(path)?);

    let encoded: Vec<u8> = bincode::serialize(data)
        .map_err(Error::Serialize)?;

    buffer.write_all(&encoded)?;
    buffer.flush()?;

    Ok(())
}

pub fn read<P: AsRef<Path>>(filename: P) -> Result<Data, Error> {
    let mut f = File::open(&filename)?;

    let mut data = Vec::new();
    f.read_to_end(&mut data)?;

    let decoded = bincode::deserialize(&data[..])
        .map_err(Error::Deserialize)?;

    Ok(decoded)
}
use std::io::BufWriter;
use std::fs::File;
use std::io::Write;

use std::io::Read;

#[cfg(test)]
mod tests {
    use super::Data;
    #[test]
    fn serialize_to_binary() {
        let model = super::load(&"./test/tube", 30.0).unwrap();
        super::write(&model, "./test/tube/tube.bin").unwrap();
    }

    #[test]
    fn deserialize() {
        let model = super::load(&"./test/tube", 30.0).unwrap();

        super::write(&model, "./test/tube/tube.bin").unwrap();
        let Data { animations: _, .. } = super::read(&"./test/tube/tube.bin").unwrap();
    }

    #[test]
    fn human() {
        let model = super::load(&"./test/human", 30.0).unwrap();
        super::write(&model, "./test/human/human.bin").unwrap();
        let Data { animations: _, .. } = super::read(&"./test/human/human.bin").unwrap();
    }

    #[test]
    fn spider() {
        let model = super::load(&"./test/spider", 30.0).unwrap();
        super::write(&model, "./test/spider/spider.bin").unwrap();
        let Data { animations: _, .. } = super::read(&"./test/spider/spider.bin").unwrap();
    }
}
