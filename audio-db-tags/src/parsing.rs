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
