use crate::{
    error::{Error, ErrorContextExt},
    parsing::{Buffer, BufferKind, U8ToBool},
};

use std::{fs::File, path::Path};

use memmap::MmapOptions;

use deku::prelude::*;

fn extract_28bit_size(mut bytes: u32) -> Result<u32, DekuError> {
    let mut size = 0u32;

    size += bytes & 0b00000000000000000000000001111111;
    bytes >>= 1;
    size += bytes & 0b00000000000000000011111110000000;
    bytes >>= 1;
    size += bytes & 0b00000000000111111100000000000000;
    bytes >>= 1;
    size += bytes & 0b00001111111000000000000000000000;

    Ok(size)
}

#[derive(Debug, DekuRead)]
#[deku(
    ctx = "endian: deku::ctx::Endian, kind: BufferKind, encoding: u8",
    id = "encoding",
    endian = "endian"
)]
enum EncodedStringBuffer {
    #[deku(id = "0")]
    Utf8 {
        #[deku(ctx = "kind", map = "Buffer::<u8>::map")]
        buffer: Vec<u8>,
    },

    #[deku(id = "1")]
    Ucs2 {
        bom: [u8; 2],

        #[deku(
            endian = "if bom[0] == 0xFF { deku::ctx::Endian::Little } else { deku::ctx::Endian::Big }",
            ctx = "kind.ucs2_adjusted()",
            map = "Buffer::<u16>::map"
        )]
        buffer: Vec<u16>,
    },
}

impl EncodedStringBuffer {
    fn map(self) -> Result<String, DekuError> {
        let utf8 = match self {
            EncodedStringBuffer::Utf8 { buffer } => buffer,
            EncodedStringBuffer::Ucs2 { buffer, .. } => {
                let mut decoded = Vec::new();
                decoded.resize(buffer.len() * 3, 0);
                let size = ucs2::decode(&buffer, &mut decoded)
                    .map_err(|err| DekuError::Parse(format!("Error decoding UCS2: {:#?}", err)))?;
                decoded.resize(size, 0);
                decoded
            }
        };

        let string = String::from_utf8(utf8).map_err(|err| DekuError::Parse(err.to_string()))?;
        Ok(string.trim_end_matches(char::from(0)).into())
    }
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian, size: u32", endian = "endian")]
struct EncodedString {
    encoding: u8,

    #[deku(ctx = "BufferKind::Sized(size - 1), *encoding")]
    buffer: EncodedStringBuffer,
}

impl EncodedString {
    fn map(self) -> Result<String, DekuError> {
        self.buffer.map()
    }
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian, size: u32", endian = "endian")]
pub struct Picture {
    encoding: u8,

    #[deku(
        ctx = "BufferKind::NullTerminated, 0",
        map = "EncodedStringBuffer::map"
    )]
    mime_type: String,

    picture_type: u8,

    #[deku(
        ctx = "BufferKind::NullTerminated, *encoding",
        map = "EncodedStringBuffer::map"
    )]
    description: String,

    #[deku(count = "size - (input.offset_from(rest) as u32 / 8)")]
    picture: Vec<u8>,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian, size: u32", endian = "endian")]
pub struct Ufid {
    #[deku(
        ctx = "BufferKind::NullTerminated, 0",
        map = "EncodedStringBuffer::map"
    )]
    vendor: String,

    #[deku(count = "size as usize - (vendor.len() + 1)")]
    ufid: Vec<u8>,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian, size: u32", endian = "endian")]
pub struct Comment {
    encoding: u8,

    language: [u8; 3],

    #[deku(
        ctx = "BufferKind::NullTerminated, *encoding",
        map = "EncodedStringBuffer::map"
    )]
    description: String,

    #[deku(
        ctx = "BufferKind::Sized(size - (input.offset_from(rest) as u32 / 8)), *encoding",
        map = "EncodedStringBuffer::map"
    )]
    comment: String
}

#[derive(Debug, DekuRead)]
#[deku(
    ctx = "variant: u8, frame_id: String, size: u32",
    id = "variant",
    endian = "big"
)]
pub enum Frame {
    #[deku(id = "0")]
    TextField {
        #[deku(skip, default = "frame_id")]
        id: String,

        #[deku(ctx = "size", map = "EncodedString::map")]
        text: String,
    },

    #[deku(id = "1")]
    Picture {
        #[deku(ctx = "size")]
        picture: Picture,
    },

    #[deku(id = "2")]
    Ufid {
        #[deku(ctx = "size")]
        ufid: Ufid,
    },

    #[deku(id = "3")]
    Comment {
        #[deku(ctx = "size")]
        comment: Comment,
    },
}

impl Frame {
    fn read_vec(
        mut rest: &BitSlice<Msb0, u8>,
    ) -> Result<(&BitSlice<Msb0, u8>, Vec<Frame>), DekuError> {
        #[derive(Debug, DekuRead)]
        #[deku(endian = "big")]
        struct TagHeader {
            major_version: u8,
            minor_version: u8,

            #[deku(map = "U8ToBool::map")]
            unsynchronisation: bool,

            #[deku(map = "U8ToBool::map")]
            extended_header: bool,

            #[deku(map = "U8ToBool::map")]
            experimental: bool,

            #[deku(bits = "5")]
            _reserved_bits: u8,

            #[deku(map = "extract_28bit_size")]
            size: u32,
        }

        let (post_header, header) = TagHeader::read(rest, ())?;
        rest = post_header;

        if header.unsynchronisation {
            return Err(DekuError::Parse(format!(
                "ID3v2 unsynchronisation is not yet implemented"
            )));
        }

        if header.extended_header {
            return Err(DekuError::Parse(format!(
                "ID3v2 extended headers are not yet implemented"
            )));
        }

        #[derive(Debug, DekuRead)]
        #[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
        struct FrameFlags {
            #[deku(map = "U8ToBool::map")]
            tag_alter_preservation: bool,

            #[deku(map = "U8ToBool::map")]
            file_alter_preservation: bool,

            #[deku(map = "U8ToBool::map")]
            read_only: bool,

            #[deku(bits = 5)]
            _reserved_bits: u8,

            #[deku(map = "U8ToBool::map")]
            compression: bool,

            #[deku(map = "U8ToBool::map")]
            encryption: bool,

            #[deku(map = "U8ToBool::map")]
            grouping_identity: bool,

            #[deku(bits = 5)]
            _more_reserved_bits: u8,
        }
        #[derive(Debug, DekuRead)]
        #[deku(endian = "big")]
        struct FrameHeader {
            #[deku(count = "4")]
            id: Vec<u8>,

            size: u32,
            flags: FrameFlags,

            #[deku(cond = "flags.compression")]
            decompressed_size: Option<u32>,

            #[deku(cond = "flags.encryption")]
            encryption_method: Option<u8>,

            #[deku(cond = "flags.grouping_identity")]
            group_id: Option<u8>,
        }

        let mut vec = Vec::new();

        while (post_header.offset_from(rest) as u32) < header.size {
            let (new_rest, frame_header) = FrameHeader::read(rest, ())?;
            let read_bits = rest.offset_from(new_rest);
            rest = new_rest;

            let read_bytes = read_bits / 8;
            let extra_read_bytes = read_bytes - 10; // Size of header without extra information = 10 bytes
            let frame_size = frame_header.size - extra_read_bytes as u32;

            let id = match frame_header.id.as_slice() {
                b"TIT2" | b"TPE1" | b"TRCK" | b"TALB" | b"TPOS" | b"TDAT" | b"TORY" | b"TYER"
                | b"TPUB" | b"TMED" | b"TPE2" | b"TSO2" | b"TSOP" | b"TXXX" => 0u8,
                b"APIC" => 1,
                b"UFID" => 2,
                b"COMM" => 3,
                _ => {
                    return Err(DekuError::Parse(format!(
                        "Unsupported frame ID: {}",
                        String::from_utf8_lossy(&frame_header.id)
                    )))
                }
            };

            let (new_rest, frame) = Frame::read(
                rest,
                (id, String::from_utf8(frame_header.id).unwrap(), frame_size),
            )?;
            rest = new_rest;

            vec.push(frame);
        }

        Ok((rest, vec))
    }
}

#[derive(Debug, DekuRead)]
#[deku(endian = "big", magic = b"ID3")]
pub struct Id3v2 {
    #[deku(reader = "Frame::read_vec(rest)")]
    frames: Vec<Frame>,
}

impl Id3v2 {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Id3v2, Error> {
        let file = File::open(path).ctx_open_file()?;
        let mmap = unsafe { MmapOptions::new().map(&file).ctx_open_file()? };
        Ok(Id3v2::from_bytes((mmap.as_ref(), 0)).ctx_parse_file()?.1)
    }
}
