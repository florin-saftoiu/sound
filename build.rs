fn main() {
    windows::build!(
        Windows::Win32::Media::Multimedia::{
            waveOutGetNumDevs,
            WAVEOUTCAPSW,
            waveOutGetDevCapsW,
            MMSYSERR_NOERROR,
            MAXERRORLENGTH,
            WAVEFORMATEX,
            WAVE_FORMAT_PCM,
            waveOutOpen,
            HWAVEOUT,
            MIDI_WAVE_OPEN_TYPE,
            WAVEHDR,
            WHDR_PREPARED,
            waveOutUnprepareHeader,
            waveOutGetErrorTextW,
            waveOutPrepareHeader,
            waveOutWrite,
            MM_WOM_DONE
        },
        Windows::Win32::Foundation::{
            PSTR,
            PWSTR
        },
        Windows::Win32::UI::KeyboardAndMouseInput::GetAsyncKeyState,
        Windows::System::VirtualKey,
        Windows::Win32::System::Console::GetConsoleWindow,
        Windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow
    );
}