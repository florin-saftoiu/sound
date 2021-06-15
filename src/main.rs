use std::f64::consts::PI;
use std::mem::{size_of, MaybeUninit};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

mod bindings {
    windows::include_bindings!();
}

use bindings::Windows::Win32::{
    Media::Multimedia::{
        waveOutGetNumDevs,
        WAVEOUTCAPSW,
        waveOutGetDevCapsW,
        MMSYSERR_NOERROR,
        WAVEFORMATEX,
        WAVE_FORMAT_PCM,
        waveOutOpen,
        HWAVEOUT,
        CALLBACK_FUNCTION,
        WAVEHDR,
        WHDR_PREPARED,
        waveOutUnprepareHeader,
        waveOutGetErrorTextW,
        waveOutPrepareHeader,
        waveOutWrite,
        MM_WOM_DONE
    },
    Foundation::{
        PSTR,
        PWSTR
    }
};

unsafe impl Send for WAVEHDR {}

extern "system" fn wave_out_proc(_wave_out: HWAVEOUT, msg: u32, dw_instance: usize, _param1: u32, _param2: u32) {
    if msg != MM_WOM_DONE {
        return
    }

    BLOCK_FREE.fetch_add(1, Ordering::SeqCst);

    unsafe {
        let block_not_zero = Arc::from_raw(dw_instance as *const (Mutex<()>, Condvar));
        let _ = block_not_zero.0.lock().unwrap();
        let _ = block_not_zero.1.notify_one();
    };
}

static BLOCK_FREE: AtomicU32 = AtomicU32::new(8);

fn main() -> windows::Result<()> {
    let device_count = unsafe { waveOutGetNumDevs() };
    let mut devices: Vec<String> = Vec::new();
    let mut woc = unsafe { MaybeUninit::<WAVEOUTCAPSW>::zeroed().assume_init() };
    for i in 0..device_count as usize {
        if unsafe { waveOutGetDevCapsW(i, &mut woc, size_of::<WAVEOUTCAPSW>() as u32) } == MMSYSERR_NOERROR {
            devices.push(String::from_utf16(unsafe { &woc.szPname }).unwrap());
        }
    }

    for d in devices.iter() {
        println!("Found Output Device:{}", d);
    }

    let mut wave_format = WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nSamplesPerSec: 44100,
        wBitsPerSample: 2 * 8,
        nChannels: 1,
        nBlockAlign: ((2 * 8) / 8) * 1,
        nAvgBytesPerSec: 44100 * 2,
        cbSize: 0
    };

    let mut hw_device = unsafe { MaybeUninit::<HWAVEOUT>::zeroed().assume_init() };

    let block_not_zero = Arc::new((Mutex::new(()), Condvar::new()));

    let mmsyserr = unsafe { waveOutOpen(&mut hw_device, 0, &mut wave_format, wave_out_proc as usize, Arc::into_raw(block_not_zero.clone()) as usize, CALLBACK_FUNCTION) };
    if mmsyserr != MMSYSERR_NOERROR {
        let mut text: [u16; 512] = [0; 512];
        unsafe { waveOutGetErrorTextW(mmsyserr, PWSTR(text.as_mut_ptr()), text.len() as u32) };
        let end = text.iter().position(|&x| x == 0).unwrap();
        let text = String::from_utf16(&text[..end]).unwrap();
        panic!("CRAP !\n{}", text);
    }

    let mut block_memory = vec![0u16; 8 * 512];
    let mut wave_headers = vec![unsafe { MaybeUninit::<WAVEHDR>::zeroed().assume_init() }; 8];

    for i in 0..8 as usize {
        wave_headers[i].dwBufferLength = 512 * 2;
        wave_headers[i].lpData = PSTR(unsafe { block_memory.as_ptr().add(i * 512) } as *mut u8);
    }

    let ready = AtomicBool::new(true);
    let mut block_current = 0usize;

    let global_time_mutex = Mutex::new(0f64);

    {
        let block_not_zero = block_not_zero.clone();
        let thread = thread::spawn(move|| {
            let mut global_time = global_time_mutex.lock().unwrap();
            *global_time = 0f64;
            let time_step = 1f64 / 44100f64;

            let max_sample = 2u16.pow((2 * 8) - 1) - 1;
            let double_max_sample = max_sample as f64;
            let mut previous_sample = 0u16;

            while ready.load(Ordering::SeqCst) {
                // wait for block to become available
                if BLOCK_FREE.load(Ordering::SeqCst) == 0 {
                    let lm = block_not_zero.0.lock().unwrap();
                    let _ = block_not_zero.1.wait(lm).unwrap();
                }

                // block is here, so use it
                BLOCK_FREE.fetch_sub(1, Ordering::SeqCst);

                // prepare block for processing
                if wave_headers[block_current].dwFlags & WHDR_PREPARED != 0 {
                    unsafe { waveOutUnprepareHeader(hw_device, &mut wave_headers[block_current], size_of::<WAVEHDR>() as u32) };
                }

                let mut new_sample = 0u16;
                let current_block = block_current * 512;

                for i in 0..512usize {
                    let user_function_res = 0.5f64 * (440f64 * 2f64 * PI * *global_time).sin();
                    new_sample = (if user_function_res >= 0f64 { f64::min(user_function_res, 1f64) } else { f64::max(user_function_res, -1f64)} * double_max_sample) as u16;

                    block_memory[current_block + i] = new_sample;
                    previous_sample = new_sample;
                    *global_time += time_step;
                }

                // send block to sound device
                unsafe { waveOutPrepareHeader(hw_device, &mut wave_headers[block_current], size_of::<WAVEHDR>() as u32)};
                unsafe { waveOutWrite(hw_device, &mut wave_headers[block_current], size_of::<WAVEHDR>() as u32)};
                block_current += 1;
                block_current %= 8;
            }
        });
    }
    
    let _ = block_not_zero.0.lock().unwrap();
    let _ = block_not_zero.1.notify_one();

    loop {

    }

    Ok(())
}
