use rodio::{source::SineWave, OutputStream, Sink, Source};

pub struct AudioHandler {
    track: Sink,
    _stream: OutputStream,
}

impl AudioHandler {
    pub fn init() -> Self {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let beep = SineWave::new(440.0).amplify(0.2).repeat_infinite();
        sink.append(beep.clone());

        // stream should not be dropped while we need to play the sound.
        Self {
            track: sink,
            _stream,
        }
    }

    pub fn tick(&self, timer: u8) {
        if timer > 0 as u8 {
            self.track.play()
        } else {
            self.track.pause()
        }
    }
}
