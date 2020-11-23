use na::{Matrix4, Quaternion, UnitQuaternion, Vector3};
#[derive(PartialEq, Clone, Debug)]
pub struct Transform {
    // Translation vector
    t: Vector3<f32>,
    // Rotating component
    r: UnitQuaternion<f32>,
}

// Code adapted to Rust from: https://www.euclideanspace.com/maths/geometry/rotations/conversions/matrixToQuaternion/
fn quat_from_mat4(m: &Matrix4<f32>) -> UnitQuaternion<f32> {
    let tr = m[(0, 0)] + m[(1, 1)] + m[(2, 2)];
    let q = if tr > 0.0 {
        let s = 0.5 / (tr + 1.0).sqrt();
        [
            0.25 / s,
            (m[(2, 1)] - m[(1, 2)]) * s,
            (m[(0, 2)] - m[(2, 0)]) * s,
            (m[(1, 0)] - m[(0, 1)]) * s,
        ]
    } else if m[(0, 0)] > m[(1, 1)] && m[(0, 0)] > m[(2, 2)] {
        let s = 2.0 * (1.0 + m[(0, 0)] - m[(1, 1)] - m[(2, 2)]).sqrt();
        [
            (m[(2, 1)] - m[(1, 2)]) / s,
            0.25 * s,
            (m[(0, 1)] + m[(1, 0)]) / s,
            (m[(0, 2)] + m[(2, 0)]) / s,
        ]
    } else if m[(1, 1)] > m[(2, 2)] {
        let s = 2.0 * (1.0 + m[(1, 1)] - m[(0, 0)] - m[(2, 2)]).sqrt();
        [
            (m[(0, 2)] - m[(2, 0)]) / s,
            (m[(0, 1)] + m[(1, 0)]) / s,
            0.25 * s,
            (m[(1, 2)] + m[(2, 1)]) / s,
        ]
    } else {
        let s = 2.0 * (1.0 + m[(2, 2)] - m[(0, 0)] - m[(1, 1)]).sqrt();
        [
            (m[(1, 0)] - m[(0, 1)]) / s,
            (m[(0, 2)] + m[(2, 0)]) / s,
            (m[(1, 2)] + m[(2, 1)]) / s,
            0.25 * s,
        ]
    };

    UnitQuaternion::new_normalize(Quaternion::new(q[0], q[1], q[2], q[3]))
}

impl From<&Matrix4<f32>> for Transform {
    fn from(m: &Matrix4<f32>) -> Self {
        let r = quat_from_mat4(m);
        let t = Vector3::<f32>::new(m[(0, 3)], m[(1, 3)], m[(2, 3)]);
        Transform { t, r }
    }
}

pub fn to_matrix4(m: &[[f32; 4]; 4]) -> Matrix4<f32> {
    let m = unsafe { std::slice::from_raw_parts(m.as_ptr() as *const f32, 16) };
    Matrix4::from_row_slice(m)
}
impl From<&[[f32; 4]; 4]> for Transform {
    fn from(m: &[[f32; 4]; 4]) -> Self {
        let mat = to_matrix4(m);
        (&mat).into()
    }
}

impl From<Transform> for Matrix4<f32> {
    fn from(t: Transform) -> Self {
        let Transform { t, r } = t;

        // Convert the quaternion to a matrix
        // describing a pure rotation
        let mut mat: Matrix4<f32> = r.into();
        // Set the translation part
        mat[(0, 3)] = t.x;
        mat[(1, 3)] = t.y;
        mat[(2, 3)] = t.z;

        mat
    }
}

impl Transform {
    pub fn interpolate(&self, rhs: &Self, alpha: f32) -> Self {
        // Linear interpolation for the translation part
        let t = self.t.lerp(&rhs.t, alpha);
        // Spherical interpolation between the rotation parts
        let r = self.r.slerp(&rhs.r, alpha);

        Transform { t, r }
    }
}
