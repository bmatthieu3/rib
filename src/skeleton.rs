use serde::{Deserialize, Serialize};
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[derive(PartialEq)]
pub struct Skeleton {
    root: Option<Bone>,
    joint_names: Vec<String>,
}

use super::transform::to_matrix4;
impl Skeleton {
    pub fn new() -> Self {
        Skeleton {
            root: None,
            joint_names: vec![],
        }
    }

    /// Parse the first skeleton 
    pub fn from(doc: &collada::document::ColladaDocument) -> Option<Skeleton> {
        if let Some(skeletons) = &doc.get_skeletons() {
            if let Some(skeleton) = skeletons.first() {
                if let Some(bind_data_set) = &doc.get_bind_data_set() {
                    let bind_data = &bind_data_set.bind_data[0];
                    let mut s = Skeleton::new();
                    //println!("skeleton joints {} bind data {}", skeleton.joints.len(), bind_data.joint_names.len());

                    let mut prev_inv_bind_pose = Matrix4::identity().into();
                    for (joint_idx, joint) in skeleton.joints.iter().enumerate() {
                        let parent_idx = if joint.parent_index == 255 {
                            // Root case
                            None
                        } else {
                            Some(joint.parent_index as usize)
                        };
                        let bind_data_joint_idx = bind_data.joint_names.iter().position(|name| {
                            let aa = bind_data.skeleton_name.as_ref().unwrap().replace(" ", "_");
                            let bind_data_name = format!("{}_{}", aa, name);
                            &bind_data_name == &joint.name
                        });
                        let mut vertices_attached = false;
                        let mut idx_transform = None;
                        let inverse_bind_pose = if let Some(bind_data_joint_idx) = bind_data_joint_idx {
                            let inverse_bind_pose = bind_data.inverse_bind_poses[bind_data_joint_idx];
                            prev_inv_bind_pose = inverse_bind_pose.clone();
                            vertices_attached = true;
                            idx_transform = Some(bind_data_joint_idx);

                            inverse_bind_pose
                        } else {
                            prev_inv_bind_pose
                            //Matrix4::identity().into()
                        };
                        /*let bind_data_name = format!("{}_{}", bind_data.skeleton_name.as_ref().unwrap(), bind_data.joint_names[bind_joint_idx]);
                        let bind_data_name = dbg!(bind_data_name.replace(" ", "_"));
                        let inverse_bind_pose = if joint.name == bind_data_name {
                            let inverse_bind_pose = bind_data.inverse_bind_poses[bind_joint_idx];
                            prev_inv_bind_pose = inverse_bind_pose.clone();

                            bind_joint_idx += 1;
                            inverse_bind_pose
                        } else {
                            prev_inv_bind_pose
                            //Matrix4::identity().into()
                        };*/

                        let bone = Bone::new(
                            joint_idx,
                            parent_idx,
                            to_matrix4(&inverse_bind_pose),
                            vertices_attached,
                            idx_transform
                        );
                        let name = joint.name.to_string();
                        s.add(name, bone);
                    }

                    Some(s)
                } else {
                    None
                }

                /*if let Some(parent_name) = &parent_name {
                    // The bone has a parent
                    let bone = Bone::new(name, Some(parent_name.clone()), to_matrix4(&inverse_bind_pose));
                    // keep trace of the parent bone hierarchy
                    let mut bones_stack = vec![bone];
                    let mut cur_parent =  &s.joints[j.parent_index as usize];
                    while !skeleton.contains(&cur_parent.name) {
                        let parent_index = cur_parent.parent_index as usize;
                        let parent_name = if parent_index == 255 {
                            None
                        } else {
                            let parent_j = &s.joints[parent_index];
                            Some(parent_j.name.clone())
                        };
                        let inverse_bind_pose_parent = to_matrix4(&bind_data.inverse_bind_poses[parent_index]);
                        let cur_bone = Bone::new(cur_parent.name.clone(), parent_name, inverse_bind_pose_parent);
                        bones_stack.push(cur_bone);
    
                        cur_parent = &s.joints[parent_index];
                    }
    
                    // cur_parent is in the bone
                    // add the parents_bone successively
                    while !bones_stack.is_empty() {
                        let bone = bones_stack.pop().unwrap();
                        skeleton.add(bone);
                    }
                } else {*/

                //}
            } else {
                None
            }
        } else {
            None
        }
    }

    fn add(&mut self, name: String, bone: Bone) {
        if let Some(parent_name_idx) = bone.parent_name_idx {
            // By construction, the parent is already in the skeleton
            // Let's check that
            assert!(self.contains(parent_name_idx));
            self.root.as_mut().unwrap()
                .add(&bone);
        } else {
            // Bone is the root
            // Make sure there is no root present
            assert!(self.root.is_none());

            self.root = Some(bone);
        }
        self.joint_names.push(name);
    }

    pub fn get_joint_names(&self) -> &[String] {
        &self.joint_names
    }

    fn contains(&self, name_idx: usize) -> bool {
        if let Some(root) = &self.root {
            root.contains(name_idx)
        } else {
            // The skeleton is empty
            // hence it contains nothing
            false
        }
    }

    pub fn get_root(&self) -> &Option<Bone> {
        &self.root
    }

    pub fn get_num_vertices_attached_bones(&self) -> usize {
        let mut num_bones = 0;
        if let Some(root) = &self.root {
            root.get_num_vertices_attached_bones(&mut num_bones);
        }
        num_bones
    }
}
use na::Matrix4;
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[derive(PartialEq)]
pub struct Bone {
    // The name of the bone
    name_idx: usize,
    /// The name of its parent, None if the bone is the root
    parent_name_idx: Option<usize>,

    /// Node's children if there are some
    children: Option<Vec<Bone>>,
    /// The inverse going from the bind pose global space to the local bone space
    inverse_bind_pose: Matrix4<f32>,
    // A flag telling if some vertices are attached to it
    // This flag will determine if the bone needs to be sent
    // to the GPU or not
    vertices_attached: bool,
    pub idx_transform: Option<usize>,
}

impl Bone {
    fn new(name_idx: usize, parent_name_idx: Option<usize>, inverse_bind_pose: Matrix4<f32>, vertices_attached: bool, idx_transform: Option<usize>) -> Self {
        Bone {
            name_idx,
            parent_name_idx,
            children: None,
            inverse_bind_pose,
            vertices_attached,
            idx_transform
        }
    }

    pub fn add(&mut self, bone: &Bone) {
        if bone.parent_name_idx.unwrap() == self.name_idx {
            // We append bone to the children
            let bone = bone.clone();
            if let Some(children) = &mut self.children {
                children.push(bone);
            } else {
                self.children = Some(vec![bone]);
            }
        } else {
            if let Some(children) = &mut self.children {
                for child in children.iter_mut() {
                    child.add(bone);
                }
            }
        }
    }

    pub fn contains(&self, name_idx: usize) -> bool {
        if self.name_idx == name_idx {
            true
        } else {
            if let Some(children) = &self.children {
                for child in children {
                    if child.contains(name_idx) {
                        return true;
                    }
                }

                false
            } else {
                false
            }
        }
    }

    pub fn get_inverse_bind_pose(&self) -> &Matrix4<f32> {
        &self.inverse_bind_pose
    }

    pub fn get_children(&self) -> Option<&Vec<Bone>> {
        self.children.as_ref()
    }

    pub fn get_name<'a>(&self, skeleton: &'a Skeleton) -> &'a str {
        &skeleton.joint_names[self.name_idx]
    }

    pub fn has_vertices_attached(&self) -> bool {
        self.vertices_attached
    }

    pub fn get_num_vertices_attached_bones(&self, num_bones: &mut usize) {
        *num_bones = if self.vertices_attached {
            *num_bones + 1
        } else {
            *num_bones
        };

        if let Some(children) = &self.children {
            for child in children {
                child.get_num_vertices_attached_bones(num_bones);
            }
        }
    }
}