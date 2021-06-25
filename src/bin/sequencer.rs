#[path ="../noise_maker.rs"]
mod noise_maker;
mod bindings {
    windows::include_bindings!();
}

use std::f64::consts::PI;
use std::io::{Write, stdout};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use noise_maker::*;
use rand::prelude::*;
use bindings::Windows::{
    Win32::{
        UI::{
            KeyboardAndMouseInput::GetAsyncKeyState,
            WindowsAndMessaging::GetForegroundWindow
        },
        System::Console::GetConsoleWindow
    },
    System::VirtualKey
};

fn w(hertz: f64) -> f64 {
    hertz * 2_f64 * PI
}

#[allow(dead_code)]
enum OscType {
    SineWave,
    SquareWave,
    TriangleWave,
    AnalogSawWave,
    DigitalSawWave,
    RandomNoise
}

fn osc(hertz: f64, time: f64, osc_type: OscType, lfo_hertz: f64, lfo_amplitude: f64) -> f64 {
    let freq = w(hertz) * time + lfo_amplitude * hertz * (w(lfo_hertz) * time).sin();

    match osc_type {
        OscType::SineWave => freq.sin(),
        OscType::SquareWave => if freq.sin() > 0_f64 { 1_f64 } else { -1_f64},
        OscType::TriangleWave => freq.sin().asin() * 2_f64 / PI,
        OscType::AnalogSawWave => (1..100).fold(0_f64, |output, n| output + ((n as f64 * freq).sin() / n as f64)) * 2_f64 / PI,
        OscType::DigitalSawWave => (2_f64 / PI) * (hertz * PI * (time % (1_f64 / hertz)) - (PI / 2_f64)),
        OscType::RandomNoise => 2_f64 * random::<f64>() - 1_f64
    }
}

struct EnvelopeADSR {
    attack_time: f64,
    decay_time: f64,
    release_time: f64,

    sustain_amplitude: f64,
    start_amplitude: f64,
}

impl Default for EnvelopeADSR {
    fn default() -> Self {
        Self {
            attack_time: 0.01_f64,
            decay_time: 0.01_f64,
            release_time: 0.02_f64,

            sustain_amplitude: 0.8_f64,
            start_amplitude: 1_f64
        }
    }
}

impl EnvelopeADSR {
    fn amplitude(&self, time: f64, time_on: f64, time_off: f64) -> f64 {
        let mut amplitude = 0_f64;
        let mut release_amplitude = 0_f64;

        if time_on > time_off { // note is on
            let life_time = time - time_on;

            // attack
            if life_time <= self.attack_time {
                amplitude = life_time / self.attack_time * self.start_amplitude;
            }

            // decay
            if life_time > self.attack_time && life_time <= self.attack_time + self.decay_time {
                amplitude = (life_time - self.attack_time) / self.decay_time * (self.sustain_amplitude - self.start_amplitude) + self.start_amplitude;
            }

            // sustain
            if life_time > self.attack_time + self.decay_time {
                amplitude = self.sustain_amplitude;
            }
        } else { // note is off
            let life_time = time_off - time_on;

            // attack
            if life_time <= self.attack_time {
                release_amplitude = life_time / self.attack_time * self.start_amplitude;
            }

            // decay
            if life_time > self.attack_time && life_time <= self.attack_time + self.decay_time {
                release_amplitude = (life_time - self.attack_time) / self.decay_time * (self.sustain_amplitude - self.start_amplitude) + self.start_amplitude;
            }

            // sustain
            if life_time > self.attack_time + self.decay_time {
                release_amplitude = self.sustain_amplitude;
            }

            amplitude = (time - time_off) / self.release_time * -release_amplitude + release_amplitude;
        }

        if amplitude <= 0_f64 {
            amplitude = 0_f64;
        }

        amplitude
    }
}

#[allow(dead_code)]
enum InstrumentType {
    Harmonica,
    Bell,
    Bell8
}

struct Instrument {
    instrument_type: InstrumentType,
    volume: f64,
    envelope: EnvelopeADSR
}

impl Instrument {
    fn new(instrument_type: InstrumentType) -> Self {
        match instrument_type {
            InstrumentType::Harmonica => Self {
                instrument_type,
                volume: 1_f64,
                envelope: EnvelopeADSR {
                    attack_time: 0.1_f64,
                    release_time: 0.2_f64,
                    ..Default::default()
                }
            },
            InstrumentType::Bell => Self {
                instrument_type,
                volume: 1_f64,
                envelope: EnvelopeADSR {
                    decay_time: 1_f64,
                    release_time: 1_f64,
                    sustain_amplitude: 0_f64,
                    ..Default::default()
                }
            },
            InstrumentType::Bell8 => Self {
                instrument_type,
                volume: 1_f64,
                envelope: EnvelopeADSR {
                    decay_time: 0.5_f64,
                    release_time: 1_f64,
                    ..Default::default()
                }
            }
        }
    }

    fn sound(&self, time: f64, n: Note) -> (f64, bool) {
        let amplitude = self.envelope.amplitude(time, n.on, n.off);
        let note_finished = amplitude <= 0_f64;
        
        (
            amplitude * 
            match self.instrument_type {
                InstrumentType::Harmonica =>
                    1_f64 * osc(scale(n.id, ScaleType::Default), n.on - time, OscType::SquareWave, 5_f64, 0.001_f64) +
                    0.5_f64 * osc(scale(n.id + 12, ScaleType::Default), n.on - time, OscType::SquareWave, 0_f64, 0_f64) +
                    0.05_f64 * osc(scale(n.id + 24, ScaleType::Default), n.on - time, OscType::RandomNoise, 0_f64, 0_f64),
                InstrumentType::Bell =>
                    1_f64 * osc(scale(n.id + 12, ScaleType::Default), n.on - time, OscType::SineWave, 5_f64, 0.001_f64) +
                    0.5_f64 * osc(scale(n.id + 24, ScaleType::Default), n.on - time, OscType::SineWave, 0_f64, 0_f64) +
                    0.25_f64 * osc(scale(n.id + 36, ScaleType::Default), n.on - time, OscType::SineWave, 0_f64, 0_f64),
                InstrumentType::Bell8 =>
                    1_f64 * osc(scale(n.id, ScaleType::Default), n.on - time, OscType::SineWave, 5_f64, 0.001_f64) +
                    0.5_f64 * osc(scale(n.id + 12, ScaleType::Default), n.on - time, OscType::SineWave, 0_f64, 0_f64) +
                    0.25_f64 * osc(scale(n.id + 24, ScaleType::Default), n.on - time, OscType::SineWave, 0_f64, 0_f64)
            } *
            self.volume,

            note_finished
        )
    }
}

#[derive(Clone, Copy)]
struct Note {
    id: i32,
    on: f64,
    off: f64,
    active: bool
}

impl Default for Note {
    fn default() -> Self {
        Self {
            id: 0,
            on: 0_f64,
            off: 0_f64,
            active: false
        }
    }
}

enum ScaleType {
    Default
}

fn scale(note_id: i32, scale_type: ScaleType) -> f64 {
    match scale_type {
        ScaleType::Default => 256_f64 * 2_f64.powf(1_f64 / 12_f64).powi(note_id)
    }
}

fn focused() -> bool {
    unsafe { GetConsoleWindow() == GetForegroundWindow() }
}

fn main() -> windows::Result<()> {
    for (id, name) in enumerate().iter() {
        println!("Found Output Device: {} - {}", id, name);
    }

    println!();
    println!("|   |   |   |   |   | |   |   |   |   | |   | |   |   |   |");
    println!("|   | S |   |   | F | | G |   |   | J | | K | | L |   |   |");
    println!("|   |___|   |   |___| |___|   |   |___| |___| |___|   |   |__");
    println!("|     |     |     |     |     |     |     |     |     |     |");
    println!("|  Z  |  X  |  C  |  V  |  B  |  N  |  M  |  ,  |  .  |  /  |");
    println!("|_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|");
    println!();

    let notes = Arc::new(Mutex::new(Vec::<(Note, Arc<Instrument>)>::new()));
    let harmonica = Arc::new(Instrument::new(InstrumentType::Harmonica));

    let make_noise = {
        let notes = notes.clone();
        move |time: f64| {
            let mut notes = notes.lock().unwrap();

            let mixed_output = notes.iter_mut().fold(0_f64, |mixed_output, (note, voice)| {
                let (output, note_finished) = voice.sound(time, *note);
                if note_finished && note.off > note.on {
                    note.active = false;
                }
                mixed_output + output
            });

            notes.retain(|(note, _)| note.active);

            mixed_output * 0.2_f64 // master volume
        }
    };

    let noise_maker = NoiseMaker::new::<i16, _>(0, 44100, 1, 8, 256, make_noise);

    let mut tp1 = Instant::now();
    let mut tp2;
    let mut _wall_time = 0_f64;

    loop {
        tp2 = Instant::now();
        let elapsed_time = tp2.duration_since(tp1).as_secs_f64();
        tp1 = tp2;
        _wall_time += elapsed_time;
        let now = noise_maker.get_time();

        if focused() {
            for k in 0..16 {
                let key_state = unsafe { GetAsyncKeyState(b"ZSXCFVGBNJMK\xbcL\xbe\xbf"[k] as i32) } as u16;
                let mut notes = notes.lock().unwrap();
                if let Some((note_found, _)) = notes.iter_mut().find(|(note, _)| note.id == k as i32) {
                    if key_state & 0x8000 != 0 { // key still held
                        if note_found.off > note_found.on { // key pressed again during release phase
                            note_found.on = now;
                            note_found.active = true;
                        }
                    } else { // key released => switch it off
                        if note_found.off < note_found.on {
                            note_found.off = now
                        }
                    }
                } else {
                    if key_state & 0x8000 != 0 { // key pressed => create new note
                        let note = Note {
                            id: k as i32,
                            on: now,
                            active: true,
                            ..Default::default()
                        };
                        notes.push((note, harmonica.clone()));
                    }
                }
            }
            print!("\rNotes: {}", notes.lock().unwrap().len());
            let _ = stdout().flush();

            if unsafe { GetAsyncKeyState(VirtualKey::Escape.0) } as u16 & 0x8000 != 0 {
                break;
            }
        }
    }

    noise_maker.stop();

    Ok(())
}
