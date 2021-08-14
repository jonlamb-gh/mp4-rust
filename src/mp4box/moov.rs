use serde::Serialize;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::mp4box::*;
use crate::mp4box::{gps::GpsBox, mvex::MvexBox, mvhd::MvhdBox, trak::TrakBox};

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct MoovBox {
    pub mvhd: MvhdBox,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mvex: Option<MvexBox>,

    #[serde(rename = "trak")]
    pub traks: Vec<TrakBox>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps: Option<GpsBox>,
}

impl MoovBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MoovBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + self.mvhd.box_size();
        for trak in self.traks.iter() {
            size += trak.box_size();
        }
        size += self.gps.as_ref().map(|gps| gps.box_size()).unwrap_or(0);
        size
    }
}

impl Mp4Box for MoovBox {
    fn box_type(&self) -> BoxType {
        return self.get_type();
    }

    fn box_size(&self) -> u64 {
        return self.get_size();
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "traks={}, gps_found={}",
            self.traks.len(),
            self.gps.is_some()
        );
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MoovBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let mut mvhd = None;
        let mut mvex = None;
        let mut traks = Vec::new();
        let mut gps = None;

        let mut current = reader.seek(SeekFrom::Current(0))?;
        let end = start + size;
        while current < end {
            // Get box header.
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;

            match name {
                BoxType::MvhdBox => {
                    mvhd = Some(MvhdBox::read_box(reader, s)?);
                }
                BoxType::MvexBox => {
                    mvex = Some(MvexBox::read_box(reader, s)?);
                }
                BoxType::TrakBox => {
                    let trak = TrakBox::read_box(reader, s)?;
                    traks.push(trak);
                }
                BoxType::GpsBox => {
                    gps = Some(GpsBox::read_box(reader, s)?);
                }
                BoxType::UdtaBox => {
                    // XXX warn!()
                    skip_box(reader, s)?;
                }
                _ => {
                    log::warn!("Skipping box {} size {}", name, s);
                    skip_box(reader, s)?;
                }
            }

            current = reader.seek(SeekFrom::Current(0))?;
        }

        if mvhd.is_none() {
            return Err(Error::BoxNotFound(BoxType::MvhdBox));
        }

        skip_bytes_to(reader, start + size)?;

        Ok(MoovBox {
            mvhd: mvhd.unwrap(),
            mvex,
            traks,
            gps,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for MoovBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        self.mvhd.write_box(writer)?;
        for trak in self.traks.iter() {
            trak.write_box(writer)?;
        }
        if let Some(gps) = &self.gps {
            gps.write_box(writer)?;
        }
        Ok(0)
    }
}
