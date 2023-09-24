use gltf_json as json;
use grim_gltf::*;
use offbeat::*;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    // Input: [ddm_file_path] [gltf_dir_path]
    let args: Vec<_> = env::args().skip(1).collect();

    if args.len() < 2 {
        println!("ddm2gltf.exe [ddm_file_path] [gltf_dir_path]");
        return;
    }

    let ddm_file_path = Path::new(&args[0]);
    let mut ddm_file = File::open(ddm_file_path).unwrap();
    let ddm = DdmFile::from_file(&mut ddm_file).unwrap();

    let gltf_output_dir_path = Path::new(&args[1]);

    //println!("{ddm:#?}");

    convert_ddm_to_gltf(ddm_file_path, &ddm, &gltf_output_dir_path);
}

fn convert_image(dds_path: &Path, png_path: &Path) {
    use image::{ImageFormat, open};

    // Create dir
    create_dir_if_not_exists(png_path.parent().unwrap()).unwrap();

    let image = open(dds_path).unwrap();
    image.save_with_format(png_path, ImageFormat::Png).unwrap();
}

fn convert_ddm_to_gltf(ddm_path: &Path, ddm: &DdmFile, output_dir_path: &Path) {
    let ddm_dir = ddm_path.parent().unwrap();
    let mut acc_builder = AccessorBuilder::new();

    let ddm_name = ddm_path
        .file_stem()
        .and_then(|f| f.to_str())
        .unwrap();

    // Process textures
    let texture_names = ddm.meshes
        .iter()
        .map(|m| {
            let tex_name = &m.tex_name;
            let in_tex_path = ddm_dir.join(format!("{tex_name}.dds"));

            let out_tex_filename = format!("{tex_name}.png");
            let out_tex_path = output_dir_path.join(&out_tex_filename);

            convert_image(&in_tex_path, &out_tex_path);
            println!("Wrote \"{out_tex_filename}\"");

            tex_name
        })
        .collect::<Vec<_>>();

    // Process meshes
    let mut meshes = Vec::new();
    let mut materials = Vec::new();
    for (mesh_idx, mesh) in ddm.meshes.iter().enumerate() {
        // Create material
        let mat_index = materials.len() as u32;
        materials.push(json::Material {
            name: Some(mesh.name.to_owned()),
            pbr_metallic_roughness: json::material::PbrMetallicRoughness {
                base_color_texture: Some(json::texture::Info {
                        index: json::Index::new(mesh_idx as u32),
                        tex_coord: 0,
                        extensions: None,
                        extras: None
                    }),
                //base_color_factor:
                ..Default::default()
            },
            emissive_factor: json::material::EmissiveFactor([0.0f32; 3]),
            alpha_mode: json::validation::Checked::Valid(json::material::AlphaMode::Mask),
            double_sided: true,
            ..Default::default()
        });

        let is_single_part = mesh.face_groups.len() <= 1;

        for (i, face_group) in mesh.face_groups.iter().enumerate() {
            let mesh_name = if is_single_part {
                mesh.name.to_owned()
            } else {
                format!("{}.{}", &mesh.name, i)
            };

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
                vertices.iter().map(|v| [-v.x, v.y, v.z])
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
                        material: Some(json::Index::new(mat_index)),
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

    // Create gltf json
    let mut gltf = json::Root {
        asset: json::Asset {
            generator: Some(format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))),
            ..Default::default()
        },
        samplers: vec![
            json::texture::Sampler {
                mag_filter: Some(json::validation::Checked::Valid(json::texture::MagFilter::Linear)),
                min_filter: Some(json::validation::Checked::Valid(json::texture::MinFilter::Nearest)),
                wrap_s: json::validation::Checked::Valid(json::texture::WrappingMode::Repeat),
                wrap_t: json::validation::Checked::Valid(json::texture::WrappingMode::Repeat),
                ..Default::default()
            }
        ],
        images: texture_names
            .iter()
            .map(|tex_name| json::Image {
                buffer_view: None,
                mime_type: Some(json::image::MimeType(String::from("image/png"))),
                name: Some(tex_name.to_string()),
                uri: Some(format!("{tex_name}.png")),
                extensions: None,
                extras: None
            })
            .collect(),
        textures: texture_names
            .iter()
            .enumerate()
            .map(|(i, tex_name)| json::Texture {
                name: Some(tex_name.to_string()),
                sampler: Some(json::Index::new(0u32)),
                source: json::Index::new(i as u32), // Image index
                extensions: None,
                extras: None
            })
            .collect(),
        nodes: {
            let mut nodes = Vec::new();

            // Root node
            nodes.push(json::Node {
                camera: None,
                children: Some((0..meshes.len())
                    .into_iter()
                    .map(|i| json::Index::new((i + 1) as u32))
                    .collect()),
                extensions: None,
                extras: None,
                matrix: None,
                mesh: None,
                name: Some(ddm_name.to_string()),
                rotation: None,
                scale: None,
                translation: None,
                skin: None,
                weights: None,
            });

            // Mesh nodes
            for i in 0..meshes.len() {
                nodes.push(json::Node {
                    camera: None,
                    children: None,
                    extensions: None,
                    extras: None,
                    matrix: None,
                    mesh: Some(json::Index::new(i as u32)),
                    name: None,
                    rotation: None,
                    scale: None,
                    translation: None,
                    skin: None,
                    weights: None,
                });
            }

            nodes
        },
        scenes: vec![
            json::Scene {
                name: None,
                //name: Some(ddm_name.to_string()),
                /*nodes: (0..meshes.len())
                    .into_iter()
                    .map(|i| json::Index::new(i as u32))
                    .collect(),*/
                nodes: vec![
                    json::Index::new(0)
                ],
                extensions: None,
                extras: None,
            }
        ],
        scene: Some(json::Index::new(0)),
        meshes,
        materials,
        ..Default::default()
    };

    // Write files
    build_binary(ddm_name, output_dir_path, &mut gltf, acc_builder);

    // Write gltf json
    let gltf_filename = format!("{ddm_name}.gltf");
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