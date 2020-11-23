

use std::collections::HashMap;
use na::Matrix4;

use serde::{Serialize, Deserialize};
use super::transform::Transform;
#[derive(Clone)]
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
struct Keyframe {
    // The transform matrix of each bone per each keyframe
    //pub bone_transforms: HashMap<String, Transform>,
    //pub bone_transforms: HashMap<String, Transform>,
    pub transforms: Vec<Matrix4<f32>>,
    // Times when each keyframe begins. Of size Nkeyframe
    pub start_time: f32
}

impl PartialEq for Keyframe {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time
    }
}

impl Keyframe {
    fn new(skeleton: &Skeleton, bone_animations: &Vec<collada::Animation>, start_time: f32, idx_keyframe: usize, alpha: f32) -> Self {
        let mut local_transforms = HashMap::with_capacity(bone_animations.len());
        let global_inverse_transform = Matrix4::identity();

        for collada::Animation { target, sample_poses, .. } in bone_animations.iter() {
            //let aa = dbg!(sample_poses.len());

            // Skip bones that are not referred by bind_data
            // These are bones for which no vertices is bound to
            // So we can ignore them
            // 1. Get the bone name focused by the current animation
            let bone_name = target.split('/').collect::<Vec<_>>();
            let bone_name = bone_name[0];

            let t0: Transform = (&sample_poses[idx_keyframe - 1]).into();
            let t1: Transform = (&sample_poses[idx_keyframe]).into();

            let t = t0.interpolate(&t1, alpha).into();

            local_transforms.insert(bone_name.to_string(), t);
        }

        let transforms = compute_final_transforms(skeleton, &local_transforms, &global_inverse_transform);
        //unreachable!();
        Keyframe {
            transforms,
            start_time
        }
    }
}

use std::cmp::{PartialOrd, Ordering};

impl PartialOrd for Keyframe {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.start_time.partial_cmp(&other.start_time)
    }
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Animation {
    /// The duration of an animation
    duration: f32,
    // A sorted-by-time keyframe vector
    keys: Vec<Keyframe>,
    frame_time: f32,
}
impl Animation {
    pub fn new(skeleton: &Skeleton, bone_animations: Vec<collada::Animation>, frame_time: f32) -> Self {
        let duration = {
            let first_anim_j = bone_animations.first().unwrap();

            let start_time = *first_anim_j.sample_times.first().unwrap();
            let end_time = *first_anim_j.sample_times.last().unwrap();
            let duration = end_time - start_time;

            duration
        };
        //let final_transforms = Vec::with_capacity(bone_animations.len());

        // At least two keyframes
        let mut idx_keyframe = 1;
        let first_bone_animation = &bone_animations[0];
        let mut end_time_keyframe = first_bone_animation.sample_times[1];

        //let duration = (num_frames as f32) * FRAME_TIME;
        let mut keys =  Vec::new();

        //let mut frame_idx = 0;
        let mut time = 0.0;

        while time < duration {
            if time >= end_time_keyframe {
                idx_keyframe += 1;
                end_time_keyframe = bone_animations[0].sample_times[idx_keyframe];
            }
            let d0 = dbg!(first_bone_animation.sample_times[idx_keyframe - 1]);
            let d1 = dbg!(first_bone_animation.sample_times[idx_keyframe]);
            let dur_keyframe = d1 - d0;
            let alpha = if dur_keyframe > 0.0 {
                (time - d0) / dur_keyframe
            } else {
                0.0
            };

            let keyframe = Keyframe::new(skeleton, &bone_animations, time, idx_keyframe, alpha);
            keys.push(keyframe);

            time += frame_time;
            //frame_idx += 1;
        }

        let bb = dbg!(idx_keyframe, duration);
        keys.push(Keyframe::new(
            skeleton,
            &bone_animations,
            duration,
            idx_keyframe,
            1.0
        ));

        Animation {
            duration,
            keys,
            frame_time
        }
    }

    /// Animation contains at least one keyframe
    /*fn get_in_between_keyframes(&self, time: f32) -> (&Keyframe, &Keyframe) {
        let num_keyframes = self.keys.len();

        assert!(num_keyframes >= 1);
        if time >= self.duration || num_keyframes == 1 {
            let last_key = self.keys.last().unwrap();
            (last_key, last_key)
        } else if time <= 0.0 {
            let first_key = self.keys.first().unwrap();
            (first_key, first_key)
        } else {
            // Binary search on keyframe starting times
            let num_step = utils::log_2(self.keys.len() as i32);
            let mut i = self.keys.len() >> 1;
            
            let mut a = 0;
            let mut b = num_keyframes - 1;

            for _ in 0..(num_step + 1) {
                // time < anim duration
                let key = &self.keys[i];
                if time == key.start_time {
                    break;
                } else if time < key.start_time {
                    b = i - 1;
                } else {
                    a = i + 1;
                }
                i = (a + b) / 2;
            }

            (&self.keys[i], &self.keys[i+1])
        }
    }*/

    pub fn query(&self, time: f32) -> &Vec<Matrix4<f32>> {
        let key = if time <= 0.0 {
            &self.keys[0]
        } else if time >= self.duration {
            &self.keys.last().unwrap()
        } else {
            let frame_idx = (time / self.frame_time) as usize;
            &self.keys[frame_idx]
        };

        &key.transforms
    }

    pub fn get_duration(&self) -> f32 {
        self.duration
    }
}

fn compute_final_transforms(
    skeleton: &Skeleton,
    bone_local_transforms: &HashMap<String, Matrix4<f32>>,
    global_inverse_transform: &Matrix4<f32>,
) -> Vec<Matrix4<f32>>{
    let root = skeleton.get_root().as_ref().unwrap();

    let mut transforms = vec![Matrix4::identity(); skeleton.get_num_vertices_attached_bones()];
    recursive_final_transforms(skeleton, &root, &Matrix4::identity(), dbg!(bone_local_transforms), global_inverse_transform, &mut transforms);
    transforms
}
fn recursive_final_transforms(
    skeleton: &Skeleton,
    bone: &Bone,
    parent_transform: &Matrix4<f32>,
    bone_local_transforms: &HashMap<String, Matrix4<f32>>,
    global_inverse_transform: &Matrix4<f32>,

    final_transforms: &mut Vec<Matrix4<f32>>
) {
    let name = bone.get_name(skeleton);

    let bone_local_transform = bone_local_transforms.get(name).unwrap();
    let local_transform = parent_transform * bone_local_transform;

    if bone.has_vertices_attached() {
        let final_transform = global_inverse_transform * local_transform * bone.get_inverse_bind_pose();

        let idx_transform = bone.idx_transform.unwrap();
        final_transforms[idx_transform] = final_transform;
    }

    if let Some(children) = bone.get_children() {
        for child in children {
            recursive_final_transforms(skeleton, child, &local_transform, bone_local_transforms, global_inverse_transform, final_transforms);
        }
    }
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Animations {
    anims: HashMap<String, Animation>,

    skeleton: Skeleton,
}

use super::skeleton::{Bone, Skeleton};
fn extract_anim_name_from_target(target: &str) -> &str {
    target.split('/').collect::<Vec<_>>()[1]
}

impl Animations {
    pub fn new(name: &str, doc: &collada::document::ColladaDocument, frame_time: f32) -> Option<Self> {
        if let Some(skeleton) = Skeleton::from(doc) {
            if let Some(animations) = doc.get_animations() {
                // If the skeleton and animations are defined, therefore there is a bind data associated to it
                // We can unwrap to get this
                let bind_data_set = doc.get_bind_data_set().unwrap();
                let bind_data = bind_data_set.bind_data.first().unwrap();

                /*let mut cur_name_anim = None;
                let mut cur_targets_in_anim = vec![];
                for anim in animations.into_iter() {
                    if let Some(cur_name) = cur_name_anim.to_owned() {
                        let new_name_anim = extract_anim_name_from_target(&anim.target).to_string();
                        if new_name_anim != cur_name {
                            let cur_anims = cur_targets_in_anim.drain(..).collect();
                            let res = 
                            anims.insert(cur_name, res);

                            cur_name_anim = Some(new_name_anim);
                        }
                    } else {
                        cur_name_anim = Some(extract_anim_name_from_target(&anim.target).to_string());
                    }

                    cur_targets_in_anim.push(anim);
                }
                // Add the last animation
                if let Some(cur_name) = cur_name_anim {
                    let res = Animation::new(&skeleton, bind_data, cur_targets_in_anim, frame_time);
                    anims.insert(cur_name, res);
                }
                */
                let anim = Animation::new(&skeleton, dbg!(animations), frame_time);
                let mut anims = HashMap::new();
                anims.insert(name.to_string(), anim);

                Some(
                    Animations {
                        anims,
                        skeleton,
                    }
                )
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn append(&mut self, other: Self) {
        assert_eq!(&self.skeleton, &other.skeleton);
        assert_eq!(other.anims.keys().len(), 1);

        for (name, anim) in other.anims {
            self.anims.insert(name, anim);
        }
    }

    pub fn get_animation(&self, name: &str) -> Option<&Animation> {
        self.anims.get(name)
    }

    pub fn query(&self, name: &str, time: f32) -> &Vec<Matrix4<f32>> {
        self.anims[name].query(time)
    }

    pub fn get_skeleton(&self) -> &Skeleton {
        &self.skeleton
    }
}