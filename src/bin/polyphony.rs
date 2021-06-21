#[path ="../noise_maker.rs"]
mod noise_maker;
mod bindings {
    windows::include_bindings!();
}

use std::f64::consts::PI;
use std::io::{Write, stdout};
use std::sync::{Arc, Mutex};
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

    trigger_on_time: f64,
    trigger_off_time: f64,

    note_on: bool
}

impl Default for EnvelopeADSR {
    fn default() -> Self {
        Self {
            attack_time: 0.01_f64,
            decay_time: 0.01_f64,
            release_time: 0.02_f64,

            sustain_amplitude: 0.8_f64,
            start_amplitude: 1_f64,

            trigger_on_time: 0_f64,
            trigger_off_time: 0_f64,

            note_on: false
        }
    }
}

impl EnvelopeADSR {
    fn get_amplitude(&self, time: f64) -> f64 {
        let mut amplitude = 0_f64;
        let life_time = time - self.trigger_on_time;

        if self.note_on {
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
                amplitude = self.sustain_amplitude
            }
        } else {
            amplitude = (time - self.trigger_off_time) / self.release_time * -self.sustain_amplitude + self.sustain_amplitude;
        }

        if amplitude <= 0.0001_f64 {
            amplitude = 0_f64;
        }

        amplitude
    }

    fn note_on(&mut self, time_on: f64) {
        self.trigger_on_time = time_on;
        self.note_on = true;
    }

    fn note_off(&mut self, time_off: f64) {
        self.trigger_off_time = time_off;
        self.note_on = false;
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

    let frequency_output = Arc::new(Mutex::new(0_f64));
    let octave_base_frequency = 220_f64;
    let twelveth_root_of_2 = 2_f64.powf(1_f64 / 12_f64);
    let envelope = Arc::new(Mutex::new(EnvelopeADSR {
        attack_time: 0.1_f64,
        release_time: 0.2_f64,
        ..Default::default()
    }));

    let frequency_output_clone = frequency_output.clone();
    let envelope_clone = envelope.clone();
    let make_noise = move |time: f64| {
        let frequency_output = frequency_output_clone.lock().unwrap();
        let envelope = envelope_clone.lock().unwrap();
        let output = envelope.get_amplitude(time) * (
            1_f64 * osc(*frequency_output, time, OscType::SquareWave, 5_f64, 0.01_f64)
        );
        output * 0.5_f64 // master volume
    };

    let noise_maker = NoiseMaker::new::<i16, _>(0, 44100, 1, 8, 256, make_noise);

    let mut current_key = -1_i32;

    loop {
        let mut key_pressed = false;
        for k in 0..16 {
            if focused() && unsafe { GetAsyncKeyState(b"ZSXCFVGBNJMK\xbcL\xbe\xbf"[k] as i32) } as u16 & 0x8000 != 0 {
                if current_key != k as i32 {
                    let mut frequency_output  = frequency_output.lock().unwrap();
                    let mut envelope = envelope.lock().unwrap();
                    *frequency_output = octave_base_frequency * twelveth_root_of_2.powi(k as i32);
                    envelope.note_on(noise_maker.get_time());
                    print!("\rNote On : {:.5}s {:.2}Hz", noise_maker.get_time(), *frequency_output);
                    let _ = stdout().flush();
                    current_key = k as i32;
                }

                key_pressed = true;
            }
        }

        if !key_pressed {
            if current_key != -1 {
                let mut envelope = envelope.lock().unwrap();
                envelope.note_off(noise_maker.get_time());
                print!("\rNote Off : {:.5}s        ", noise_maker.get_time());
                let _ = stdout().flush();
                current_key = -1;
            }
        }

        if focused() && unsafe { GetAsyncKeyState(VirtualKey::Escape.0) } as u16 & 0x8000 != 0 {
            break;
        }
    }

    noise_maker.stop();

    Ok(())
}
