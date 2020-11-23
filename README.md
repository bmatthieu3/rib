## rib: a Rust collada importer for your 3D game projects

rib is built on top of the [piston_collada](https://github.com/PistonDevelopers/piston_collada).
You give it the path to a directory containing all the DAE files (1 animation per file) of your 3D model
and it gives you:
- The vertices of the model. Each vertex contains a position and may contain a normal, texcoord, two bones indexes of the bones influencing this vertex as well as the weight associated to these two bones.
- The animations of the model if there are. Internally it is stored as a hashmap indexed by the DAE filename containing the animation. It is possible to query at a specific time the transform matrices of the bones in the world space.

As a user, you just need to:
- Send as vertex attributes the vertices from the model at the beginning of the program
- At each frame, query the animation to get the current bone matrices and send them, e.g. as an array of matrices 4x4 or as a texture (see this [article](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-2-animated-crowd-rendering) for more explanation)

rib offers **read** and **write** methods that use [bincode]() for (de)serializing the vertices/animations to a compressed binary format. This way, in a game for example, it will be faster to load the binaries than parsing the multiple collada files one by one for building the rib data-structure.

## Example

Here is an example of a [human low-poly](https://opengameart.org/content/animated-human-low-poly) model found on the very good opengameart.org game resources archive:

https://youtu.be/9Xwf7G9upOY

This is animated using rib:
- The vertices are sent to the GPU as vertex attribute at the beginning of the program.
- The matrices are retreived from the ***walk*** animation and sent as a uniform array of mat4.

```rust
// Get the current matrices of the bones position in the model space
let transforms = anims.query("walk", cur_time);
// Get the shader and bind it
let shader = shaders.get("animated_model").unwrap();
let shader = shader.bind(&gl);
// Send the &[nalgebra::Matrix4<f32>] to the GPU
// See https://nalgebra.org/cg_recipes/#conversions-for-shaders
unsafe {
    let num_matrices = transforms.len();
    gl.UniformMatrix4fv(location_bone_transforms, num_matrices as i32, gl::FALSE, transforms.as_slice().as_ptr() as *const f32);
};
// Draw your model
model.draw(gl, &shader, camera);
```

Here is the ***vertex shader***:
```glsl
#version 330
layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 texcoord;
layout (location = 3) in vec2 weights;
layout (location = 4) in ivec2 bones_idx;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;

const int MAX_JOINTS = 50;
uniform mat4 bone_transforms[MAX_JOINTS];

out vec2 uv;

void main() {
    mat4 transform = bone_transforms[bones_idx[0]] * weights[0] + bone_transforms[bones_idx[1]] * weights[1];

    vec4 ndc = projection * view * model * transform * vec4(position, 1);
    gl_Position = ndc;
    uv = texcoord;
}
```

And the ***fragment shader***:
```glsl
#version 330
uniform sampler2D tex;

in vec2 uv;
out vec4 finalColor;

void main() {
    finalColor = texture(tex, uv);
}
```

## How can I use it?

Some little adjustements of the .blend must be done:
1. Limit the maximum number of bones influencing each vertex to 2! (reduce from 4 to 2)
![Limit the number of bones influencing a vertex](https://github.com/bmatthieu3/rib/blob/master/misc/weights.png)
2. Select the animation you want to export in the Action Editor of blender
3. Select the mesh you want to export with its skeleton attached
3. Export to collada file (.dae)
    1. In the **Main** tab. OpenGL's up vector is the Y axis but blender's one is Z. Check the apply box with X as the forward axis and Y as the up axis.
    ![change up axis](https://github.com/bmatthieu3/rib/blob/master/misc/main.png)
    
    2. In **Geom** tab, check the Triangulate box.
    
    ![enable triangulation](https://github.com/bmatthieu3/rib/blob/master/misc/geom.png)
    
    3. In **Anim** tab, check Include Animations, Keep Keyframes, All Keyed Curves, Include all Actions. Set a very big Sampling Rate because rib needs only the keyframes (sampling is done inside of rib by specifying a sampling rate when loading the file). This prevents your collada files to get huge too!

![anim options](https://github.com/bmatthieu3/rib/blob/master/misc/anim.png)

After that, simply load the directory containing all your DAE files with rib. Each .dae will contain one animation. For the moment it is not possible to export multiple animations inside only ONE collada file because the Blender collada export does not recognize the actions stack from Blender. That is why a fix can be to:
1. Export one animation per file
2. Export one file with one animation but this animation contains all the walk keyframes next to the run keyframes next to the idle keyframes etc...

For the moment the 1. solution is handled by rib. The second solution may be implemented in the future!

## Contributing instructions

Post issues, PR if you want to participate and develop the library.

To run the tests, simply

```bash
cargo test
```

in the root of the repository.
