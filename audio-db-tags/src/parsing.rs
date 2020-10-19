use deku::prelude::*;

#[derive(DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub(crate) struct CountThenVec<T: DekuRead<deku::ctx::Endian>> {
    #[allow(dead_code)]
    count: u32,

    #[deku(count = "*count")]
    vec: Vec<T>,
}

impl<T: DekuRead<deku::ctx::Endian>> CountThenVec<T> {
    pub(crate) fn map(self) -> Result<Vec<T>, DekuError> {
        Ok(self.vec)
    }
}

impl CountThenVec<u8> {
    pub fn map_str(self) -> Result<String, DekuError> {
        let vec = self.map()?;
        String::from_utf8(vec).map_err(|err| DekuError::Parse(err.to_string()))
    }
}

#[derive(DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub(crate) struct U8ToBool {
    #[deku(bits = 1)]
    value: u8,
}

impl U8ToBool {
    pub(crate) fn map(self) -> Result<bool, DekuError> {
        Ok(self.value != 0)
    }
}

pub(crate) enum BufferKind {
    NullTerminated,
    Sized(u32)
}

impl BufferKind {
    pub(crate) fn ucs2_adjusted(self) -> BufferKind {
        match self {
            BufferKind::NullTerminated => BufferKind::NullTerminated,
            BufferKind::Sized(size) => BufferKind::Sized((size / 2) - 1)
        }
    }
}

pub(crate) struct Buffer<T> {
    buffer: Vec<T>
}

impl<T> Buffer<T> {
    pub(crate) fn map(self) -> Result<Vec<T>, DekuError> {
        Ok(self.buffer)
    }
}

impl<T> From<Vec<T>> for Buffer<T> {
    fn from(vec: Vec<T>) -> Buffer<T> {
        Buffer{buffer: vec}
    }
}

impl<T> DekuRead<(deku::ctx::Endian, BufferKind)> for Buffer<T> where T: DekuRead<deku::ctx::Endian> + PartialEq + Default + std::fmt::Debug{
    fn read(mut rest: &BitSlice<Msb0, u8>, (endian, kind): (deku::ctx::Endian, BufferKind)) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized 
    {
        match kind {
            BufferKind::NullTerminated => {
                let mut vec = Vec::new();

                loop {
                    let (new_rest, value) = T::read(rest, endian)?;
                    rest = new_rest;

                    if value == T::default() {
                        return Ok((rest, vec.into()))
                    }

                    vec.push(value);
                }
            }

            BufferKind::Sized(size) => {
                let (new_rest, vec) = Vec::<T>::read(rest, (deku::ctx::Count(size as usize), endian))?;
                Ok((new_rest, vec.into()))
            }
        }
    }
}
