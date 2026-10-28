//! Frame transport for the GNOME Shell extension: AA01, little-endian width
//! and height, followed by tightly packed RGBA frames at 20 fps.

use aa_render::{render_to_size, RenderOptions};
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

pub fn stream(
    scene_id: &str,
    opts: RenderOptions,
    width: u32,
    height: u32,
    output: &mut impl Write,
) -> Result<(), String> {
    stream_with_controls(
        GnomeStreamOptions {
            scene_id,
            render: opts,
            width,
            height,
            doom_enabled: false,
            settings: &[],
            capture_keys: false,
        },
        output,
    )
}

pub fn stream_with_doom(
    scene_id: &str,
    opts: RenderOptions,
    width: u32,
    height: u32,
    doom_enabled: bool,
    output: &mut impl Write,
) -> Result<(), String> {
    stream_with_controls(
        GnomeStreamOptions {
            scene_id,
            render: opts,
            width,
            height,
            doom_enabled,
            settings: &[],
            capture_keys: false,
        },
        output,
    )
}

pub fn stream_with_doom_settings(
    scene_id: &str,
    opts: RenderOptions,
    width: u32,
    height: u32,
    doom_enabled: bool,
    settings: &[(String, f64)],
    output: &mut impl Write,
) -> Result<(), String> {
    stream_with_controls(
        GnomeStreamOptions {
            scene_id,
            render: opts,
            width,
            height,
            doom_enabled,
            settings,
            capture_keys: false,
        },
        output,
    )
}

pub struct GnomeStreamOptions<'a> {
    pub scene_id: &'a str,
    pub render: RenderOptions,
    pub width: u32,
    pub height: u32,
    pub doom_enabled: bool,
    pub settings: &'a [(String, f64)],
    pub capture_keys: bool,
}

pub fn stream_with_controls(
    options: GnomeStreamOptions<'_>,
    output: &mut impl Write,
) -> Result<(), String> {
    if options.width == 0 || options.height == 0 || options.width > 1920 || options.height > 1080 {
        return Err("GNOME frame dimensions must be within 1x1 and 1920x1080".into());
    }
    if !options.width.is_multiple_of(options.render.cell_w.max(1))
        || !options.height.is_multiple_of(options.render.cell_h.max(1))
    {
        return Err("GNOME frame dimensions must be multiples of the cell dimensions".into());
    }
    let mut scene = super::make_scene(
        options.scene_id,
        options.doom_enabled,
        (options.width / options.render.cell_w.max(1)) as usize,
        (options.height / options.render.cell_h.max(1)) as usize,
        options.settings,
    )?;
    scene.apply_base_color(options.render.theme.text);
    scene.set_grid(
        (options.width / options.render.cell_w.max(1)) as usize,
        (options.height / options.render.cell_h.max(1)) as usize,
    );
    scene.start();
    let key_events = if options.capture_keys && scene.is_interactive() {
        Some(read_stdin_events()?)
    } else {
        None
    };
    let result = (|| -> io::Result<()> {
        output.write_all(b"AA01")?;
        output.write_all(&options.width.to_le_bytes())?;
        output.write_all(&options.height.to_le_bytes())?;
        let start = Instant::now();
        loop {
            let tick = Instant::now();
            if let Some(events) = &key_events {
                for bytes in events.try_iter() {
                    scene.send_key(&bytes);
                }
            }
            let frame = scene.frame(start.elapsed().as_secs_f64());
            output.write_all(
                &render_to_size(&frame, &options.render, options.width, options.height).pixels,
            )?;
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

fn read_stdin_events() -> Result<Receiver<Vec<u8>>, String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("ascii-arcade-doom-input".into())
        .spawn(move || {
            let mut input = io::stdin().lock();
            let mut bytes = [0_u8; 64];
            loop {
                match input.read(&mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(count) if sender.send(bytes[..count].to_vec()).is_err() => break,
                    Ok(_) => {}
                }
            }
        })
        .map_err(|error| format!("could not start DOOM input reader: {error}"))?;
    Ok(receiver)
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
