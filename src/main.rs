use std::fs::OpenOptions;
use std::io::Cursor;

use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};

const NUM_CHANNELS: usize = 2;
const SOURCE_PATH: &str = "/home/mark/source.raw";
const TARGET_PATH: &str = "/home/mark/target.raw";

const SINC_BUFFER: [[f32; NUM_CHANNELS]; 128] = [[0f32; NUM_CHANNELS]; 128];

fn read_int_frames() -> impl Iterator<Item = [i32; NUM_CHANNELS]> {
    let bytes = std::fs::read(SOURCE_PATH).unwrap();

    println!("Read {} bytes", bytes.len());

    let mut reader = Cursor::new(bytes);

    let frames = std::iter::from_fn(move || {
        let mut buf = [0i32; NUM_CHANNELS];

        // We assume that the only error that occurs is when trying to read
        // from the cursor when it is already empty.
        reader.read_i32_into::<NativeEndian>(&mut buf).ok()?;
        Some(buf)
    });

    frames
}

fn write_int_frames(frames: impl Iterator<Item = [i32; NUM_CHANNELS]>) {
    let mut out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(TARGET_PATH)
        .unwrap();

    for frame in frames {
        for sample in std::array::IntoIter::new(frame) {
            out_file.write_i32::<NativeEndian>(sample).unwrap();
        }
    }
}

fn dasp_impl(int_frames: impl Iterator<Item = [i32; NUM_CHANNELS]>) -> impl Iterator<Item = [i32; NUM_CHANNELS]> {
    use dasp_frame::Frame;
    use dasp_sample::Sample;
    use dasp_signal::Signal;
    use dasp_ring_buffer::Fixed;
    use dasp_interpolate::sinc::Sinc;

    let frames = int_frames.map(|frame| Frame::map(frame, f32::from_sample));
    let signal = dasp_signal::from_iter(frames);

    let ring_buffer = Fixed::from(SINC_BUFFER);
    let interpolator = Sinc::new(ring_buffer);

    let interpolated_signal = signal.from_hz_to_hz(interpolator, 44100.0, 48000.0);

    let clipped_signal = interpolated_signal.map(|frame| {
        let clipped_frame: [f32; NUM_CHANNELS] = Frame::map(frame, |sample| {
            if sample > 1.0 {
                println!("Clipping detected: {}", sample);
                1.0
            } else if sample < -1.0 {
                println!("Clipping detected: {}", sample);
                -1.0
            } else {
                sample
            }
        });

        clipped_frame
    });

    let output_signal = clipped_signal.map(|frame| Frame::map(frame, i32::from_sample));

    output_signal.until_exhausted()
}

fn sampara_impl(int_frames: impl Iterator<Item = [i32; NUM_CHANNELS]>) -> impl Iterator<Item = [i32; NUM_CHANNELS]> {
    use sampara::{Frame, Sample, Signal};
    use sampara::buffer::Fixed;
    use sampara::interpolate::Sinc;

    let frames = int_frames.map(|frame| Frame::apply(frame, f32::from_sample));
    let signal = sampara::signal::from_frames(frames);

    // Note that the buffer is passed directly to the interpolator!
    let interpolator = Sinc::new(SINC_BUFFER);

    let interpolated_signal = signal.interpolate(interpolator, 44100.0 / 48000.0);

    let clipped_signal = interpolated_signal.map(|frame| {
        let clipped_frame: [f32; NUM_CHANNELS] = Frame::apply(frame, |sample| {
            if sample > 1.0 {
                println!("Clipping detected: {}", sample);
                1.0
            } else if sample < -1.0 {
                println!("Clipping detected: {}", sample);
                -1.0
            } else {
                sample
            }
        });

        clipped_frame
    });

    let output_signal = clipped_signal.map(|frame| Frame::apply(frame, i32::from_sample));

    output_signal.into_iter()
}

fn main() {
    // let the_impl = dasp_impl;
    let the_impl = sampara_impl;

    let in_int_frames = read_int_frames();
    let out_int_frames = the_impl(in_int_frames);
    write_int_frames(out_int_frames);
}
