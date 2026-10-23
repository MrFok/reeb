#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod audition;
mod config;
mod icon;
mod motion;
#[cfg(windows)]
mod popup;
mod preview;
mod theme;
mod ui;

use eframe::egui;
use reeb::{
    audio::Audio,
    keys::KeyEvent,
    mixer::{Mixer, Settings, Shared},
    samples::SampleBank,
};
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|v| v == "--render-demo") {
        let path = args.get(2).ok_or("Usage: reeb --render-demo output.wav")?;
        return render_demo(Path::new(path));
    }
    if args.get(1).is_some_and(|v| v == "--smoke-audio") {
        return smoke_audio();
    }
    #[cfg(windows)]
    if reeb::tray::Tray::open_existing() {
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon::data())
            .with_decorations(false)
            .with_inner_size([840.0, 660.0])
            .with_min_inner_size([720.0, 580.0]),
        window_builder: Some(Box::new(|builder| {
            builder
                .with_inner_size([840.0, 660.0])
                .with_maximized(false)
                .with_fullscreen(false)
        })),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "reeb",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)?))),
    )?;
    Ok(())
}

fn scan_for_key(key: egui::Key) -> Option<u16> {
    use egui::Key::*;
    Some(match key {
        Escape => 0x01,
        Num1 => 0x02,
        Num2 => 0x03,
        Num3 => 0x04,
        Num4 => 0x05,
        Num5 => 0x06,
        Num6 => 0x07,
        Num7 => 0x08,
        Num8 => 0x09,
        Num9 => 0x0a,
        Num0 => 0x0b,
        Minus => 0x0c,
        Equals => 0x0d,
        Backspace => 0x0e,
        Tab => 0x0f,
        Q => 0x10,
        W => 0x11,
        E => 0x12,
        R => 0x13,
        T => 0x14,
        Y => 0x15,
        U => 0x16,
        I => 0x17,
        O => 0x18,
        P => 0x19,
        OpenBracket => 0x1a,
        CloseBracket => 0x1b,
        Enter => 0x1c,
        A => 0x1e,
        S => 0x1f,
        D => 0x20,
        F => 0x21,
        G => 0x22,
        H => 0x23,
        J => 0x24,
        K => 0x25,
        L => 0x26,
        Semicolon => 0x27,
        Quote => 0x28,
        Backtick => 0x29,
        Backslash => 0x2b,
        Z => 0x2c,
        X => 0x2d,
        C => 0x2e,
        V => 0x2f,
        B => 0x30,
        N => 0x31,
        M => 0x32,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        Space => 0x39,
        ArrowLeft => 0x14b,
        ArrowRight => 0x14d,
        ArrowUp => 0x148,
        ArrowDown => 0x150,
        Delete => 0x153,
        _ => return None,
    })
}

fn render_demo(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let shared = Shared::new(&Settings::default());
    let mut mixer = Mixer::new(Arc::new(SampleBank::bundled()?), shared.clone(), 48000);
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for (index, scan) in [0x1e, 0x1f, 0x20, 0x21, 0x39, 0x1c, 0x10, 0x19, 0x0e, 0x39]
        .into_iter()
        .enumerate()
    {
        shared.push(KeyEvent {
            scan,
            pressed: true,
        });
        for block in 0..45 {
            if block == 12 {
                shared.push(KeyEvent {
                    scan,
                    pressed: false,
                });
            }
            if index == 5 && block == 8 {
                shared.push(KeyEvent {
                    scan: 0x10,
                    pressed: true,
                });
            }
            let mut samples = [0.0_f32; 512];
            mixer.render(&mut samples, 2);
            for value in samples {
                writer.write_sample((value * 32767.0) as i16)?;
            }
        }
    }
    writer.finalize()?;
    println!("Rendered {}", path.display());
    Ok(())
}

fn smoke_audio() -> Result<(), Box<dyn std::error::Error>> {
    let shared = Shared::new(&Settings::default());
    let audio = Audio::start(Arc::new(SampleBank::bundled()?), shared.clone(), None)?;
    for scan in [0x1e, 0x21, 0x39, 0x1c] {
        shared.push(KeyEvent {
            scan,
            pressed: true,
        });
        std::thread::sleep(Duration::from_millis(80));
        shared.push(KeyEvent {
            scan,
            pressed: false,
        });
        std::thread::sleep(Duration::from_millis(100));
    }
    if shared.stream_failed.load(Ordering::Relaxed) || shared.frames.load(Ordering::Relaxed) == 0 {
        return Err("The output stream did not render successfully".into());
    }
    println!(
        "Audio stream passed: {}; {} frames",
        audio.description,
        shared.frames.load(Ordering::Relaxed)
    );
    Ok(())
}
