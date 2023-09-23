use gltf_json as json;
use grim_gltf::*;
use offbeat::*;
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    // Input: [ddm_path] [gltf_dir_path]
    let args: Vec<_> = env::args().skip(1).collect();

    let ddm_file_path = Path::new(&args[0]);
    let mut ddm_file = File::open(ddm_file_path).unwrap();
    let ddm = DdmFile::from_file(&mut ddm_file).unwrap();

    let gltf_output_dir_path = Path::new(&args[1]);

    //println!("{ddm:#?}");

    convert_ddm_to_gltf(&ddm, &gltf_output_dir_path);
}

fn convert_ddm_to_gltf(ddm: &DdmFile, output_dir_path: &Path) {
    for mesh in ddm.meshes.iter() {

        for face_group in mesh.face_groups.iter() {
            // 3 indicies = 1 triangle
            let index_start = face_group.triangle_start_idx as usize;
            let index_count = (face_group.triangle_count * 3) as usize;

            let triangle_indicies = &ddm.triangles[index_start..(index_start + index_count)];
        }
    }
}