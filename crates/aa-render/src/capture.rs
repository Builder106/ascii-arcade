//! Small dependency-free PNG and GIF encoders used by desktop shells.

use crate::PixelBuffer;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub fn write_png(image: &PixelBuffer, path: impl AsRef<Path>) -> io::Result<()> {
    if image.width == 0 || image.height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PNG dimensions are empty",
        ));
    }
    let expected = image.width as usize * image.height as usize * 4;
    if image.pixels.len() != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PNG pixel buffer size is invalid",
        ));
    }

    let mut scanlines = Vec::with_capacity(expected + image.height as usize);
    let stride = image.width as usize * 4;
    for row in image.pixels.chunks_exact(stride) {
        scanlines.push(0);
        scanlines.extend_from_slice(row);
    }
    let compressed = stored_zlib(&scanlines);
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&image.width.to_be_bytes());
    header.extend_from_slice(&image.height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    png_chunk(&mut bytes, *b"IHDR", &header);
    png_chunk(&mut bytes, *b"IDAT", &compressed);
    png_chunk(&mut bytes, *b"IEND", &[]);
    File::create(path)?.write_all(&bytes)
}

pub fn write_gif(
    frames: &[PixelBuffer],
    path: impl AsRef<Path>,
    frames_per_second: u16,
) -> io::Result<()> {
    let Some(first) = frames.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GIF has no frames",
        ));
    };
    if first.width == 0
        || first.height == 0
        || first.width > u16::MAX as u32
        || first.height > u16::MAX as u32
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GIF dimensions are invalid",
        ));
    }
    let expected = first.width as usize * first.height as usize * 4;
    if frames.iter().any(|frame| {
        frame.width != first.width || frame.height != first.height || frame.pixels.len() != expected
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GIF frames do not share one valid size",
        ));
    }

    let mut output = Vec::new();
    output.extend_from_slice(b"GIF89a");
    output.extend_from_slice(&(first.width as u16).to_le_bytes());
    output.extend_from_slice(&(first.height as u16).to_le_bytes());
    output.extend_from_slice(&[0xf7, 0, 0]);
    for index in 0..=255_u16 {
        let red = (((index >> 5) & 0x07) * 255 / 7) as u8;
        let green = (((index >> 2) & 0x07) * 255 / 7) as u8;
        let blue = ((index & 0x03) * 85) as u8;
        output.extend_from_slice(&[red, green, blue]);
    }
    output.extend_from_slice(b"!\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00");

    let delay = (100 / frames_per_second.max(1)).max(1);
    for frame in frames {
        output.extend_from_slice(&[0x21, 0xf9, 0x04, 0x00]);
        output.extend_from_slice(&delay.to_le_bytes());
        output.extend_from_slice(&[0, 0]);
        output.push(0x2c);
        output.extend_from_slice(&[0, 0, 0, 0]);
        output.extend_from_slice(&(first.width as u16).to_le_bytes());
        output.extend_from_slice(&(first.height as u16).to_le_bytes());
        output.push(0);
        output.push(8);
        write_subblocks(&mut output, &literal_lzw(&frame.pixels));
    }
    output.push(0x3b);
    File::create(path)?.write_all(&output)
}

fn stored_zlib(bytes: &[u8]) -> Vec<u8> {
    let mut output = vec![0x78, 0x01];
    let mut offset = 0;
    while offset < bytes.len() {
        let count = (bytes.len() - offset).min(u16::MAX as usize);
        let final_block = offset + count == bytes.len();
        output.push(u8::from(final_block));
        let length = count as u16;
        output.extend_from_slice(&length.to_le_bytes());
        output.extend_from_slice(&(!length).to_le_bytes());
        output.extend_from_slice(&bytes[offset..offset + count]);
        offset += count;
    }
    output.extend_from_slice(&adler32(bytes).to_be_bytes());
    output
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for chunk in bytes.chunks(5552) {
        for &byte in chunk {
            a += u32::from(byte);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

fn png_chunk(output: &mut Vec<u8>, name: [u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(&name);
    output.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&name);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn literal_lzw(rgba: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(rgba.len());
    let mut bits = 0_u32;
    let mut bit_count = 0_u8;
    {
        let mut emit = |code: u16| {
            bits |= u32::from(code) << bit_count;
            bit_count += 9;
            while bit_count >= 8 {
                output.push(bits as u8);
                bits >>= 8;
                bit_count -= 8;
            }
        };
        for pixel in rgba.as_chunks::<4>().0 {
            emit(256);
            let index = ((pixel[0] & 0xe0) | ((pixel[1] & 0xe0) >> 3) | (pixel[2] >> 6)) as u16;
            emit(index);
        }
        emit(257);
    }
    if bit_count > 0 {
        output.push(bits as u8);
    }
    output
}

fn write_subblocks(output: &mut Vec<u8>, bytes: &[u8]) {
    for block in bytes.chunks(u8::MAX as usize) {
        output.push(block.len() as u8);
        output.extend_from_slice(block);
    }
    output.push(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_path(extension: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("aa-capture-{}.{}", std::process::id(), extension))
    }

    #[test]
    fn writes_png_header_and_end_chunk() {
        let path = temporary_path("png");
        write_png(
            &PixelBuffer {
                width: 1,
                height: 1,
                pixels: vec![1, 2, 3, 255],
            },
            &path,
        )
        .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.ends_with(b"IEND\xaeB`\x82"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_empty_or_malformed_png_buffers() {
        let path = temporary_path("invalid.png");
        assert!(write_png(
            &PixelBuffer {
                width: 0,
                height: 1,
                pixels: Vec::new()
            },
            &path
        )
        .is_err());
        assert!(write_png(
            &PixelBuffer {
                width: 1,
                height: 1,
                pixels: Vec::new()
            },
            &path
        )
        .is_err());
    }

    #[test]
    fn writes_animated_gif_header_and_trailer() {
        let path = temporary_path("gif");
        let frame = PixelBuffer {
            width: 1,
            height: 1,
            pixels: vec![1, 2, 3, 255],
        };
        write_gif(&[frame], &path, 15).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"GIF89a"));
        assert_eq!(bytes.last(), Some(&0x3b));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_empty_or_malformed_gif_frames() {
        let path = temporary_path("invalid.gif");
        assert!(write_gif(&[], &path, 15).is_err());
        assert!(write_gif(
            &[PixelBuffer {
                width: 0,
                height: 1,
                pixels: Vec::new()
            }],
            &path,
            15
        )
        .is_err());
        assert!(write_gif(
            &[PixelBuffer {
                width: u16::MAX as u32 + 1,
                height: 1,
                pixels: Vec::new()
            }],
            &path,
            15
        )
        .is_err());
        assert!(write_gif(
            &[PixelBuffer {
                width: 1,
                height: 1,
                pixels: Vec::new()
            }],
            &path,
            15
        )
        .is_err());
        assert!(write_gif(
            &[
                PixelBuffer {
                    width: 1,
                    height: 1,
                    pixels: vec![0; 4]
                },
                PixelBuffer {
                    width: 2,
                    height: 1,
                    pixels: vec![0; 8]
                },
            ],
            &path,
            15
        )
        .is_err());
    }
}
