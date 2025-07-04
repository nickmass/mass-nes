use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};

use byteorder::{LE, WriteBytesExt};

pub struct WavWriter {
    file: BufWriter<File>,
    sample_rate: u32,
    sample_count: u32,
    buffer: Vec<u8>,
}

impl WavWriter {
    pub fn new(file: File, sample_rate: u32) -> std::io::Result<Self> {
        let mut wav = Self {
            file: BufWriter::new(file),
            sample_rate,
            sample_count: 0,
            buffer: Vec::with_capacity(1024),
        };

        wav.write_header()?;

        Ok(wav)
    }

    pub fn write_samples(&mut self, samples: &[i16]) -> std::io::Result<()> {
        let out_samples = samples.iter().flat_map(|s| s.to_le_bytes());
        self.buffer.clear();
        self.buffer.extend(out_samples);

        self.sample_count += samples.len() as u32;

        self.file.write_all(&self.buffer[0..(samples.len() * 2)])
    }

    fn write_header(&mut self) -> std::io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(b"RIFF")?;
        let total_size = (44 + self.sample_count * 2) - 8;
        let bytes_per_sample = std::mem::size_of::<i16>() as u32;
        self.file.write_u32::<LE>(total_size)?;
        self.file.write_all(b"WAVE")?;
        self.file.write_all(b"fmt ")?;
        self.file.write_u32::<LE>(16)?; // Chunk size
        self.file.write_u16::<LE>(1)?; // Audio Format (1: integer)
        self.file.write_u16::<LE>(1)?; // Number of channels
        self.file.write_u32::<LE>(self.sample_rate)?;
        self.file
            .write_u32::<LE>(self.sample_rate * bytes_per_sample)?; // Byte per second
        self.file.write_u16::<LE>(bytes_per_sample as u16)?; // Bytes per block
        self.file.write_u16::<LE>(16)?; // Bits per sample
        self.file.write_all(b"data")?;
        self.file
            .write_u32::<LE>(self.sample_count * bytes_per_sample)?;

        Ok(())
    }

    pub fn finalize(mut self) -> std::io::Result<()> {
        self.write_header()
    }
}
