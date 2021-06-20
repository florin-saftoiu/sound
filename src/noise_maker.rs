use std::mem::{size_of, MaybeUninit};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

mod bindings {
    windows::include_bindings!();
}

use bindings::Windows::{
    Win32::{
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
    }
};

unsafe impl Send for WAVEHDR {}

// callback for the sound driver to request more data
extern "system" fn wave_out_proc(_wave_out: HWAVEOUT, msg: u32, dw_instance: usize, _dw_param1: usize, _dw_param2: usize) {
    if msg != MM_WOM_DONE {
        return
    }

    let block_not_zero = unsafe { Arc::from_raw(dw_instance as *mut (Mutex<usize>, Condvar)) };
    let mut block_free = block_not_zero.0.lock().unwrap();
    *block_free += 1;
    block_not_zero.1.notify_one();
}

pub fn enumerate() -> Vec<(usize, String)> {
    let device_count = unsafe { waveOutGetNumDevs() };
    let mut devices: Vec<(usize, String)> = Vec::new();
    let mut woc = unsafe { MaybeUninit::<WAVEOUTCAPSW>::zeroed().assume_init() };
    for i in 0..device_count as usize {
        if unsafe { waveOutGetDevCapsW(i, &mut woc, size_of::<WAVEOUTCAPSW>() as u32) } == MMSYSERR_NOERROR {
            devices.push((i, String::from_utf16(unsafe { &woc.szPname }).unwrap()));
        }
    }
    devices
}

fn clip(sample: f64, max: f64) -> f64 {
    if sample >= 0f64 {
        f64::min(sample, max)
    } else {
        f64::max(sample, -max)
    }
}

pub struct NoiseMaker {
    global_time: Arc<Mutex<f64>>,
    ready: Arc<AtomicBool>,
    thread_handle: JoinHandle<()>
}

impl NoiseMaker {
    pub fn new<F>(device_id: usize, sample_rate: u32, channels: u16, blocks: usize, block_samples: u32, user_function: F) -> Self where F: Fn(f64) -> f64 + Send + 'static {
        let block_not_zero = Arc::new((Mutex::new(blocks), Condvar::new()));
        let global_time = Arc::new(Mutex::new(0f64));
        let ready = Arc::new(AtomicBool::new(true));
        
        let mut wave_format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nSamplesPerSec: sample_rate,
            wBitsPerSample: 2 * 8,
            nChannels: channels,
            nBlockAlign: ((2 * 8) / 8) * channels,
            nAvgBytesPerSec: sample_rate * ((2 * 8) / 8) * channels as u32,
            cbSize: 0
        };
        let mut hw_device = unsafe { MaybeUninit::<HWAVEOUT>::zeroed().assume_init() };

        let mmsyserr = unsafe { waveOutOpen(&mut hw_device, device_id as u32, &mut wave_format, wave_out_proc as usize, Arc::into_raw(block_not_zero.clone()) as usize, CALLBACK_FUNCTION) };
        if mmsyserr != MMSYSERR_NOERROR {
            let mut text: [u16; 512] = [0; 512];
            unsafe { waveOutGetErrorTextW(mmsyserr, PWSTR(text.as_mut_ptr()), text.len() as u32) };
            let end = text.iter().position(|&x| x == 0).unwrap();
            let text = String::from_utf16(&text[..end]).unwrap();
            panic!("Error calling waveOutOpen {}", text);
        }

        let mut block_memory = vec![0i16; blocks * block_samples as usize];
        let mut wave_headers = vec![unsafe { MaybeUninit::<WAVEHDR>::zeroed().assume_init() }; blocks];

        for i in 0..blocks as usize {
            wave_headers[i].dwBufferLength = block_samples * 2;
            wave_headers[i].lpData = PSTR(unsafe { block_memory.as_ptr().add(i * block_samples as usize) } as *mut u8);
        }

        let mut block_current = 0usize;
        
        // spawn a thread to fill blocks with audio data, waiting for the sound
        // driver to be done with them
        let thread_handle = thread::spawn({
            let block_not_zero = block_not_zero.clone();
            let global_time = global_time.clone();
            let ready = ready.clone();
            move || {
                let time_step = 1f64 / 44100f64;

                let max_sample = (2u16.pow((2 * 8) - 1) - 1) as f64;

                while ready.load(Ordering::SeqCst) {
                    // wait for block to become available
                    let mut block_free = block_not_zero.0.lock().unwrap();
                    while *block_free == 0  {
                        block_free = block_not_zero.1.wait(block_free).unwrap();
                    }

                    // block is here, so use it
                    *block_free -= 1;
                    // allow wave_out_proc to increment the count
                    drop(block_free);

                    // prepare block for processing
                    if wave_headers[block_current].dwFlags & WHDR_PREPARED != 0 {
                        unsafe { waveOutUnprepareHeader(hw_device, &mut wave_headers[block_current], size_of::<WAVEHDR>() as u32) };
                    }

                    let current_block = block_current * block_samples as usize;

                    for i in 0..block_samples as usize {
                        let global_time_value = {
                            let global_time = global_time.lock().unwrap();
                            *global_time
                        };
                        let new_sample = (clip(user_function(global_time_value), 1f64) * max_sample) as i16;

                        block_memory[current_block + i] = new_sample;
                        let mut global_time = global_time.lock().unwrap();
                        *global_time += time_step;
                    }

                    // send block to sound device
                    unsafe { waveOutPrepareHeader(hw_device, &mut wave_headers[block_current], size_of::<WAVEHDR>() as u32)};
                    unsafe { waveOutWrite(hw_device, &mut wave_headers[block_current], size_of::<WAVEHDR>() as u32)};
                    block_current += 1;
                    block_current %= blocks;
                }
            }
        });
        
        let block_free = block_not_zero.0.lock().unwrap();
        block_not_zero.1.notify_one();
        drop(block_free);

        Self {
            global_time,
            ready,
            thread_handle
        }
    }

    pub fn get_time(&self) -> f64 {
        let global_time = self.global_time.lock().unwrap();
        *global_time
    }

    pub fn stop(self) {
        self.ready.store(false, Ordering::SeqCst);
        self.thread_handle.join().expect("Could not join NoiseMaker thread");
    }
}
