use crate::{
    error::{Error, ErrorContextExt},
    parsing::CountThenVec,
};

use std::{fs::File, path::Path};

use memmap::MmapOptions;

use deku::prelude::*;
#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub struct StreamInfo {
    min_block_size: u16,
    max_block_size: u16,

    #[deku(bits = "24")]
    min_frame_size: u32,

    #[deku(bits = "24")]
    max_frame_size: u32,

    #[deku(bits = "20")]
    sample_rate: u32,

    #[deku(bits = "3")]
    channels: u8,

    #[deku(bits = "5")]
    bits_per_sample: u8,

    #[deku(bits = "36")]
    total_samples: u64,

    md5_signature: u128,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub struct SeekPoint {
    first_sample: u64,
    offset: u64,
    samples: u16,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub struct UserComment {
    #[deku(map = "CountThenVec::<u8>::map_str")]
    comment: String,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "_: deku::ctx::Endian", endian = "little")]
pub struct VorbisComment {
    #[deku(map = "CountThenVec::<u8>::map_str")]
    vendor: String,

    #[deku(map = "CountThenVec::<UserComment>::map")]
    comments: Vec<UserComment>,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian", type = "u32", endian = "endian")]
pub enum PictureKind {
    #[deku(id = "0")]
    Other,

    #[deku(id = "1")]
    FileIcon32x32,

    #[deku(id = "2")]
    OtherFileIcon,

    #[deku(id = "3")]
    FrontCover,

    #[deku(id = "4")]
    BackCover,

    #[deku(id = "5")]
    LeafletPage,

    #[deku(id = "6")]
    Media,

    #[deku(id = "7")]
    LeadArtist,

    #[deku(id = "8")]
    Artist,

    #[deku(id = "9")]
    Conductor,

    #[deku(id = "10")]
    Band,

    #[deku(id = "11")]
    Composer,

    #[deku(id = "12")]
    Lyricist,

    #[deku(id = "13")]
    RecordingLocation,

    #[deku(id = "14")]
    DuringRecording,

    #[deku(id = "15")]
    DuringPerformance,

    #[deku(id = "16")]
    MovieCapture,

    #[deku(id = "17")]
    BrightColouredFish,

    #[deku(id = "18")]
    Illustration,

    #[deku(id = "19")]
    ArtistLogotype,

    #[deku(id = "20")]
    PublisherLogotype,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub struct Picture {
    kind: PictureKind,

    #[deku(map = "CountThenVec::<u8>::map_str")]
    mime_type: String,

    #[deku(map = "CountThenVec::<u8>::map_str")]
    description: String,

    width: u32,
    height: u32,
    depth: u32,
    colours: u32,

    #[deku(map = "CountThenVec::<u8>::map")]
    data: Vec<u8>,
}

#[derive(Debug, DekuRead)]
#[deku(ctx = "block_type: u8, size: u32", endian = "big", id = "block_type")]
pub enum Metadata {
    #[deku(id = "0")]
    StreamInfo(StreamInfo),

    #[deku(id = "1")]
    Padding {
        #[deku(count = "size / 8")]
        padding: Vec<u8>,
    },

    #[deku(id = "3")]
    SeekTable {
        #[deku(count = "size / 18")]
        seekpoints: Vec<SeekPoint>,
    },

    #[deku(id = "4")]
    VorbisComment(VorbisComment),

    #[deku(id = "6")]
    Picture(Picture),
}

impl Metadata {
    fn read_vec(
        mut rest: &BitSlice<Msb0, u8>,
    ) -> Result<(&BitSlice<Msb0, u8>, Vec<Metadata>), DekuError> {
        #[derive(DekuRead)]
        #[deku(endian = "big")]
        struct MetadataHeader {
            #[deku(bits = 1)]
            is_last: u8,

            #[deku(bits = 7)]
            block_type: u8,

            #[deku(bits = 24)]
            size: u32,
        }

        let mut vec = Vec::new();

        loop {
            let (new_rest, header) = MetadataHeader::read(rest, ())?;
            let (newer_rest, metadata) =
                Metadata::read(new_rest, (header.block_type, header.size))?;

            vec.push(metadata);

            if header.is_last != 0 {
                return Ok((newer_rest, vec));
            }

            rest = newer_rest;
        }
    }
}

#[derive(Debug, DekuRead)]
#[deku(endian = "big", magic = b"fLaC")]
pub struct Flac {
    #[deku(reader = "Metadata::read_vec(rest)")]
    metadata: Vec<Metadata>,
}

impl Flac {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Flac, Error> {
        let file = File::open(path).ctx_open_file()?;
        let mmap = unsafe { MmapOptions::new().map(&file).ctx_open_file()? };
        Ok(Flac::from_bytes((mmap.as_ref(), 0)).ctx_parse_file()?.1)
    }
}
