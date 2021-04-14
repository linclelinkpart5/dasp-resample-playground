use std::fs::OpenOptions;
use std::io::Cursor;

use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};
use dasp_frame::Frame;
use dasp_sample::Sample;
use dasp_signal::Signal;
use dasp_ring_buffer::Fixed;
use dasp_interpolate::sinc::Sinc;

const NUM_CHANNELS: usize = 2;
const SOURCE_PATH: &str = "/home/mark/source.raw";
const TARGET_PATH: &str = "/home/mark/target.raw";

fn main() {
    let bytes = std::fs::read(SOURCE_PATH).unwrap();

    println!("Read {} bytes", bytes.len());

    let mut reader = Cursor::new(bytes);

    let samples = std::iter::from_fn(move || {
        reader.read_i32::<NativeEndian>().ok().map(|x| f32::from_sample(x))
    });

    let signal = dasp_signal::from_interleaved_samples_iter::<_, [f32; NUM_CHANNELS]>(samples);

    let ring_buffer = Fixed::from([[0f32; NUM_CHANNELS]; 128]);
    let interpolator = Sinc::new(ring_buffer);

    let interpolated_signal = signal.from_hz_to_hz(interpolator, 44100.0, 48000.0);
    // let out_samples = interpolated_signal.into_interleaved_samples();

    let mut out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(TARGET_PATH)
        .unwrap();

    let mut num_clipped = 0;
    for frame in interpolated_signal.until_exhausted() {
        let clipped_frame: [f32; NUM_CHANNELS] = frame.map(|sample| {
            if sample > 1.0 {
                num_clipped += 1;
                1.0
            } else if sample < -1.0 {
                num_clipped += 1;
                -1.0
            } else {
                sample
            }
        });

        for sample in clipped_frame.channels() {
            let int_sample: i32 = sample.to_sample();

            out_file.write_i32::<NativeEndian>(int_sample).unwrap();
        }
    }

    println!("Clipped samples: {}", num_clipped);
}
