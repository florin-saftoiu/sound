mod noise_maker;
mod bindings {
    windows::include_bindings!();
}

use std::f64::consts::PI;
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

    let frequency_output = Arc::new(Mutex::new(440f64));

    {
        let frequency_output = frequency_output.clone();
        let make_noise = move |time: f64| {
            let frequency_output = frequency_output.lock().unwrap();
            let output = (*frequency_output * 2f64 * PI * time).sin();
            output * 0.5f64
        };

        noise_maker(0, 44100, 1, 8, 512, make_noise);
    }

    loop {
        if unsafe { GetAsyncKeyState(VirtualKey::Escape.0) } as u16 & 0x8000 != 0 {
            break;
        } else if unsafe { GetAsyncKeyState(VirtualKey::A.0) } as u16 & 0x8000 != 0 {
            let mut frequency_output = frequency_output.lock().unwrap();
            *frequency_output = 220f64;
        }
    }

    Ok(())
}
