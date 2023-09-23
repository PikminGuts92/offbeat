use crate::ByteReader;
use std::io::{Read, Seek};

#[derive(Debug, Default)]
pub struct DdmFaceGroup {
    pub indicies: [u16; 30],
    pub triangle_start_idx: u32,
    pub triangle_count: u32,
}

#[derive(Debug, Default)]
pub struct DdmMesh {
    pub name: String,
    pub transform: [f32; 16],
    pub tex_name: String,
    pub tex_ext: String,
    pub face_groups: Vec<DdmFaceGroup>,
}

#[derive(Debug, Default)]
pub struct DdmBone {
    pub name: String,
    pub transform: [f32; 16],
    pub id: u32
}

#[derive(Clone, Debug, Default)]
pub struct DdmVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    pub u: f32,
    pub v: f32,
    pub bone_0: f32,
    pub bone_1: f32,
    pub bone_2: f32,
    pub bone_3: f32,
    pub weight_0: f32,
    pub weight_1: f32,
    pub weight_2: f32,
    pub weight_3: f32,
}

#[derive(Debug, Default)]
pub struct DdmFile {
    pub meshes: Vec<DdmMesh>,
    pub bones: Vec<DdmBone>,
    pub triangles: Vec<u16>,
    pub vertices: Vec<DdmVertex>,
}

impl DdmFile {
    pub fn from_file<T: Read + Seek>(stream: &mut T) -> Result<Self, std::io::Error> {
        let mut reader = ByteReader::new(stream);

        let magic = reader.read_bytes::<4>()?;

        // TODO: Use custom error type
        let is_skinned = match &magic {
            b"srdd" => true,  // Model with 64-byte sized vertices
            b"mrdd" => false, // Model with 32-byte sized vertices
            _ => panic!("Unsupported magic of \"{magic:?}\"")
        };

        let mut ddm = DdmFile::default();
        reader.skip(4)?;

        // Read meshes
        let mesh_count = reader.read::<u32>()?;
        for _ in 0..mesh_count {
            let mut mesh = DdmMesh::default();

            // Read name
            mesh.name = reader.read_string::<64>()?;
            reader.skip(8)?;

            // Read transform
            for t in mesh.transform.iter_mut() {
                *t = reader.read()?;
            }

            // Read texture name + ext
            reader.skip(4)?;
            let raw_string = reader.read_bytes::<256>()?;
            let (tex_name, tex_ext) = split_str(&raw_string);
            mesh.tex_name = tex_name.to_string();
            mesh.tex_ext = tex_ext.to_string();

            // Read face groups
            let group_count = if is_skinned {
                reader.read::<u32>()?
            } else {
                1u32
            };

            for _ in 0..group_count {
                let mut group = DdmFaceGroup::default();

                if is_skinned {
                    reader.skip(4)?; // Index count
                    for ind in group.indicies.iter_mut() {
                        *ind = reader.read()?;
                    }
                }

                group.triangle_start_idx = reader.read()?;
                group.triangle_count = reader.read()?;

                mesh.face_groups.push(group);
            }

            ddm.meshes.push(mesh);
        }

        // Read bones
        let bone_count = if is_skinned {
            reader.read()?
        } else {
            0u32
        };

        for _ in 0..bone_count {
            let mut bone = DdmBone::default();

            // Read transform
            for t in bone.transform.iter_mut() {
                *t = reader.read()?;
            }

            // Read name
            bone.name = reader.read_string::<64>()?;

            // Read id
            bone.id = reader.read()?;

            ddm.bones.push(bone);
        }

        // Read faces
        let face_count = reader.read::<u32>()? as usize;
        let face_buffer = reader.read_n_bytes(face_count * 2)?;

        ddm.triangles = vec![0u16; face_count];
        for (i, ee) in face_buffer.chunks_exact(2).enumerate() {
            match ee {
                &[e0, e1] => {
                    ddm.triangles[i] = u16::from_le_bytes([e0, e1]);
                },
                _ => unreachable!()
            }
        }

        // Read vertices
        let vertex_count = reader.read::<u32>()? as usize;
        //let vertex_size = if is_skinned { 64 } else { 32 };
        //let vertex_buffer = reader.read_n_bytes(vertex_count * vertex_size)?;

        ddm.vertices = vec![DdmVertex::default(); vertex_count];
        for v in ddm.vertices.iter_mut() {
            // Read pos
            v.x = reader.read()?;
            v.y = reader.read()?;
            v.z = reader.read()?;

            // Read normals
            v.nx = reader.read()?;
            v.ny = reader.read()?;
            v.nz = reader.read()?;

            // Read uv
            v.u = reader.read()?;
            v.v = reader.read()?;

            if !is_skinned {
                continue;
            }

            // Read bone indices
            v.bone_0 = reader.read()?;
            v.bone_1 = reader.read()?;
            v.bone_2 = reader.read()?;
            v.bone_3 = reader.read()?;

            // Read weights
            v.weight_0 = reader.read()?;
            v.weight_1 = reader.read()?;
            v.weight_2 = reader.read()?;
            v.weight_3 = reader.read()?;
        }

        Ok(ddm)
    }
}

fn split_str<'a>(raw: &'a [u8]) -> (&'a str, &'a str) {
    let mut first_size: Option<usize> = None;
    let mut second_size: Option<usize> = None;

    for (i, c) in raw.iter().enumerate() {
        if first_size.is_some() && second_size.is_some() {
            break;
        } else if c.ne(&b'\0') {
            continue;
        }

        match (&mut first_size, &mut second_size) {
            (s, _) if s.is_none() => {
                *s = Some(i);
            },
            (Some(b), s) if s.is_none() => {
                *s = Some(i - (*b + 1));
            },
            _ => unreachable!()
        }
    }

    let (s0, s1) = (first_size.unwrap(), second_size.unwrap());

    (
        std::str::from_utf8(&raw[..s0]).unwrap(),
        std::str::from_utf8(&raw[(s0 + 1)..((s0 + 1) + s1)]).unwrap()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_str_test() {
        let (str1, str2) = split_str(b"hello\0world\0");
        assert_eq!("hello", str1);
        assert_eq!("world", str2);
    }
}