mod noise_maker;
mod bindings {
    windows::include_bindings!();
}

use std::f64::consts::PI;
use std::io::{Write, stdout};
use std::sync::{Arc, Mutex};
use noise_maker::*;
use bindings::Windows::{
    Win32::UI::KeyboardAndMouseInput::GetAsyncKeyState,
    System::VirtualKey
};

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

    let frequency_output = Arc::new(Mutex::new(0f64));
    let octave_base_frequency = 110f64;
    let twelveth_root_of_2 = 2f64.powf(1f64 / 12f64);

    let frequency_output_clone = frequency_output.clone();
    let make_noise = move |time: f64| {
        let frequency_output = frequency_output_clone.lock().unwrap();
        let output = (*frequency_output * 2f64 * PI * time).sin();
        output * 0.5f64
    };

    let noise_maker = NoiseMaker::new(0, 44100, 1, 8, 256, make_noise);

    let mut current_key = -1i32;

    loop {
        let mut key_pressed = false;
        for k in 0..16 {
            if unsafe { GetAsyncKeyState(b"ZSXCFVGBNJMK\xbcL\xbe\xbf"[k] as i32) } as u16 & 0x8000 != 0 {
                if current_key != k as i32 {
                    let mut frequency_output  = frequency_output.lock().unwrap();
                    *frequency_output = octave_base_frequency * twelveth_root_of_2.powi(k as i32);
                    print!("\rNote On : {:.5}s {:.2}Hz", noise_maker.get_time(), *frequency_output);
                    let _ = stdout().flush();
                    current_key = k as i32;
                }

                key_pressed = true;
            }
        }

        if !key_pressed {
            if current_key != -1 {
                print!("\rNote Off : {:.5}s        ", noise_maker.get_time());
                let _ = stdout().flush();
                current_key = -1;
            }

            let mut frequency_output  = frequency_output.lock().unwrap();
            *frequency_output = 0f64;
        }

        if unsafe { GetAsyncKeyState(VirtualKey::Escape.0) } as u16 & 0x8000 != 0 {
            break;
        }
    }

    noise_maker.stop();

    Ok(())
}
