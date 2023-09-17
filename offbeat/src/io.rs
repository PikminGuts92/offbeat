use std::io::{Error as IOError, Read, Seek, SeekFrom};

pub trait Primitive : Sized {
    //fn is_sized() -> bool;
    //fn size() -> usize;
    fn from_reader<'a, T: Read + Seek>(reader: &mut ByteReader<'a, T>) -> Result<Self, IOError>;
}

impl Primitive for f32 {
    fn from_reader<'a, T: Read + Seek>(reader: &mut ByteReader<'a, T>) -> Result<Self, IOError> {
        let mut buffer = [0u8; std::mem::size_of::<Self>()];
        reader.stream.read_exact(&mut buffer)?;

        Ok(Self::from_le_bytes(buffer))
    }
}

impl Primitive for u16 {
    fn from_reader<'a, T: Read + Seek>(reader: &mut ByteReader<'a, T>) -> Result<Self, IOError> {
        let mut buffer = [0u8; std::mem::size_of::<Self>()];
        reader.stream.read_exact(&mut buffer)?;

        Ok(Self::from_le_bytes(buffer))
    }
}

impl Primitive for u32 {
    fn from_reader<'a, T: Read + Seek>(reader: &mut ByteReader<'a, T>) -> Result<Self, IOError> {
        let mut buffer = [0u8; std::mem::size_of::<Self>()];
        reader.stream.read_exact(&mut buffer)?;

        Ok(Self::from_le_bytes(buffer))
    }
}

pub struct ByteReader<'a, T: Read + Seek> {
    stream: &'a mut T,
}

impl<'a, T: Read + Seek> ByteReader<'a, T> {
    pub fn new(stream: &'a mut T) -> Self {
        ByteReader {
            stream
        }
    }

    pub fn read<S: Primitive>(&mut self) -> Result<S, IOError> {
        S::from_reader(self)
    }

    pub fn read_bytes<const S: usize>(&mut self) -> Result<[u8; S], IOError> {
        let mut buffer = [0u8; S];
        self.stream.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    pub fn read_n_bytes(&mut self, size: usize) -> Result<Vec<u8>, IOError> {
        let mut buffer = vec![0u8; size];
        self.stream.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    pub fn read_string<const S: usize>(&mut self) -> Result<String, IOError> {
        let mut buffer = [0u8; S];
        self.stream.read_exact(&mut buffer)?;

        // Interpret an non null-terminated string
        if buffer.iter().all(|c| c.ne(&b'\0')) {
            let res = std::str::from_utf8(&buffer)
                .unwrap()
                .to_string();

            return Ok(res);
        }

        let res = std::ffi::CStr::from_bytes_until_nul(&buffer)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        Ok(res)
    }

    pub fn seek(&mut self, offset: u64) -> Result<(), IOError> {
        self.stream.seek(SeekFrom::Start(offset)).map(|_| ())
    }

    pub fn skip(&mut self, offset: i64) -> Result<(), IOError> {
        self.stream.seek(SeekFrom::Current(offset)).map(|_| ())
    }
}