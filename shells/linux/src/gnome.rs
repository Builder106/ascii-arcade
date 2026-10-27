//! Frame transport for the GNOME Shell extension: AA01, little-endian width
//! and height, followed by tightly packed RGBA frames at 20 fps.

use aa_render::RenderOptions;
use std::io::{self, Write};
use std::time::{Duration, Instant};

pub fn stream(
    scene_id: &str,
    opts: RenderOptions,
    width: u32,
    height: u32,
    output: &mut impl Write,
) -> Result<(), String> {
    if width == 0 || height == 0 || width > 1920 || height > 1080 {
        return Err("GNOME frame dimensions must be within 1x1 and 1920x1080".into());
    }
    if !width.is_multiple_of(opts.cell_w.max(1)) || !height.is_multiple_of(opts.cell_h.max(1)) {
        return Err("GNOME frame dimensions must be multiples of the cell dimensions".into());
    }
    let mut scene =
        aa_core::scenes::make(scene_id).ok_or_else(|| format!("unknown scene '{scene_id}'"))?;
    scene.apply_base_color(opts.theme.text);
    scene.set_grid(
        (width / opts.cell_w.max(1)) as usize,
        (height / opts.cell_h.max(1)) as usize,
    );
    scene.start();
    let result = (|| -> io::Result<()> {
        output.write_all(b"AA01")?;
        output.write_all(&width.to_le_bytes())?;
        output.write_all(&height.to_le_bytes())?;
        let start = Instant::now();
        loop {
            let tick = Instant::now();
            let frame = scene.frame(start.elapsed().as_secs_f64());
            output.write_all(&aa_render::render(&frame, &opts).pixels)?;
            output.flush()?;
            std::thread::sleep(Duration::from_millis(50).saturating_sub(tick.elapsed()));
        }
    })();
    scene.stop();
    match result {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.to_string()),
        Ok(()) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OneFrame(Vec<u8>);

    impl Write for OneFrame {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.0.len() >= 12 + 16 * 16 * 4 {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn sends_header_and_exact_rgba_frame_then_exits_on_closed_pipe() {
        let mut output = OneFrame(Vec::new());
        stream("matrix", RenderOptions::default(), 16, 16, &mut output).unwrap();
        assert_eq!(&output.0[..4], b"AA01");
        assert_eq!(&output.0[4..8], &16_u32.to_le_bytes());
        assert_eq!(&output.0[8..12], &16_u32.to_le_bytes());
        assert_eq!(output.0.len(), 12 + 16 * 16 * 4);
        assert!(output.0[12..]
            .as_chunks::<4>()
            .0
            .iter()
            .all(|px| px[3] == 255));
    }

    #[test]
    fn rejects_invalid_dimensions_and_scenes_before_writing() {
        for (scene, width, height) in [
            ("donut", 0, 16),
            ("donut", 1928, 16),
            ("donut", 16, 1088),
            ("donut", 17, 16),
            ("missing", 16, 16),
        ] {
            let mut output = Vec::new();
            assert!(stream(scene, RenderOptions::default(), width, height, &mut output).is_err());
            assert!(output.is_empty());
        }
    }
}
