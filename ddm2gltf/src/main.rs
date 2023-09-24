use gltf_json as json;
use grim_gltf::*;
use offbeat::*;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::Write;
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
    let mut acc_builder = AccessorBuilder::new();
    let mut meshes = Vec::new();

    // Process meshes
    for (i, mesh) in ddm.meshes.iter().enumerate() {
        for face_group in mesh.face_groups.iter() {
            let mesh_name = format!("{}.{}", &mesh.name, i);

            // 3 indicies = 1 triangle
            let index_start = face_group.triangle_start_idx as usize;
            let index_count = (face_group.triangle_count * 3) as usize;

            let triangle_indicies = &ddm.triangles[index_start..(index_start + index_count)];

            let mut vertices = Vec::new();
            let mut faces = Vec::new();
            let mut vert_map = HashMap::new(); // old face idx -> new face idx

            // Map verts and faces
            for old_idx in triangle_indicies {
                if let Some(new_idx) = vert_map.get(old_idx) {
                    faces.push(*new_idx);
                } else {
                    let vert = &ddm.vertices[*old_idx as usize];
                    let new_idx = vertices.len() as u16;

                    vert_map.insert(*old_idx, new_idx);
                    faces.push(new_idx);
                    vertices.push(vert);
                }
            }

            let pos_idx = acc_builder.add_array(
                format!("{}_pos", &mesh_name),
                vertices.iter().map(|v| [v.x, v.y, v.z])
            );

            let norm_idx = acc_builder.add_array(
                format!("{}_norm", &mesh_name),
                vertices.iter().map(|v| [v.nx, v.ny, v.nz])
            );

            let uv_idx = acc_builder.add_array(
                format!("{}_uv", &mesh_name),
                vertices.iter().map(|v| [v.u, v.v])
            );

            // Need to be scalar for some reason
            let face_idx = acc_builder.add_scalar(
                format!("{}_face", &mesh_name),
                faces
            );

            meshes.push(json::Mesh {
                name: Some(mesh_name.to_owned()),
                primitives: vec![
                    json::mesh::Primitive {
                        attributes: {
                            let mut map = BTreeMap::new();

                            // Add positions
                            if let Some(acc_idx) = pos_idx {
                                map.insert(
                                    json::validation::Checked::Valid(json::mesh::Semantic::Positions),
                                    json::Index::new(acc_idx as u32)
                                );
                            }

                            // Add normals
                            if let Some(acc_idx) = norm_idx {
                                map.insert(
                                    json::validation::Checked::Valid(json::mesh::Semantic::Normals),
                                    json::Index::new(acc_idx as u32)
                                );
                            }

                            // Add uvs
                            if let Some(acc_idx) = uv_idx {
                                map.insert(
                                    json::validation::Checked::Valid(json::mesh::Semantic::TexCoords(0)),
                                    json::Index::new(acc_idx as u32)
                                );
                            }

                            // Add weights
                            /*if let Some(acc_idx) = weight_idx {
                                map.insert(
                                    json::validation::Checked::Valid(json::mesh::Semantic::Weights(0)),
                                    json::Index::new(acc_idx as u32)
                                );
                            }

                            // Add bones
                            if let Some(acc_idx) = bone_idx {
                                map.insert(
                                    json::validation::Checked::Valid(json::mesh::Semantic::Joints(0)),
                                    json::Index::new(acc_idx as u32)
                                );
                            }

                            // Add tangents
                            if let Some(acc_idx) = tan_idx {
                                map.insert(
                                    json::validation::Checked::Valid(json::mesh::Semantic::Tangents),
                                    json::Index::new(acc_idx as u32)
                                );
                            }*/

                            map
                        },
                        indices: face_idx
                            .map(|idx| json::Index::new(idx as u32)),
                        /*material: mat_map
                            .get(&mesh.mat)
                            .map(|idx| json::Index::new(*idx as u32)),*/
                        material: None,
                        mode: json::validation::Checked::Valid(gltf::mesh::Mode::Triangles),
                        targets: None,
                        extras: None,
                        extensions: None
                    },
                ],
                weights: None,
                extras: None,
                extensions: None
            });
        }
    }

    // TODO: Process textures

    // Create gltf json
    let mut gltf = json::Root {
        asset: json::Asset {
            generator: Some(format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))),
            ..Default::default()
        },
        nodes: meshes
            .iter()
            .enumerate()
            .map(|(i, m)| json::Node {
                camera: None,
                children: None,
                extensions: None,
                extras: None,
                matrix: None,
                mesh: Some(json::Index::new(i as u32)),
                name: None, //Some(dir_name.to_string()),
                rotation: None,
                scale: None,
                translation: None,
                skin: None,
                weights: None,
            })
            .collect(),
        scenes: vec![
            json::Scene {
                name: None,
                nodes: (0..meshes.len())
                    .into_iter()
                    .map(|i| json::Index::new(i as u32))
                    .collect(),
                extensions: None,
                extras: None,
            }
        ],
        scene: Some(json::Index::new(0)),
        meshes,
        ..Default::default()
    };

    // Write files
    let basename = "ddm_model";
    build_binary(basename, output_dir_path, &mut gltf, acc_builder);

    // Write gltf json
    let gltf_filename = format!("{basename}.gltf");
    let gltf_path = output_dir_path.join(&gltf_filename);
    let writer = std::fs::File::create(&gltf_path).expect("I/O error");
    json::serialize::to_writer_pretty(writer, &gltf).expect("Serialization error");

    println!("Wrote \"{gltf_filename}\"");
}

fn create_dir_if_not_exists<T>(dir_path: T) -> Result<(), std::io::Error> where T: AsRef<Path> {
    let dir_path = dir_path.as_ref();

    if !dir_path.exists() {
        // Not found, create directory
        std::fs::create_dir_all(&dir_path)?;
    }

    Ok(())
}

fn build_binary(basename: &str, output_dir: &Path, gltf: &mut json::Root, acc_builder: AccessorBuilder) {
    // Write as external file
    create_dir_if_not_exists(output_dir).unwrap();

    //let basename = self.get_basename();
    let filename = format!("{basename}.bin");
    let bin_path = output_dir.join(&filename);

    let (accessors, views, buffer, data) = acc_builder.generate(&filename);

    let mut writer = std::fs::File::create(&bin_path).unwrap();
    writer.write_all(&data).unwrap();

    println!("Wrote \"{filename}\"");

    /*buffer.uri = {
        use base64::{Engine as _, engine::{self, general_purpose}, alphabet};

        let mut str_data = String::from("data:application/octet-stream;base64,");
        general_purpose::STANDARD.encode_string(&data, &mut str_data);

        Some(str_data)
    };*/

    // Update bin data
    gltf.accessors = accessors;
    gltf.buffers = vec![buffer];
    gltf.buffer_views = views;
}