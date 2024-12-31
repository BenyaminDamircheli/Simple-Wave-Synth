use rodio::{OutputStream, Sink, Source};
use serde::Deserialize;
use std::time::Duration;
use std::f32::consts::PI;
use std::path::Path;

const SAMPLE_RATE: f32 = 44100.0;
const A4_FREQ: f32 = 440.0;
const OCTAVE_SEMITONES: i32 = 12;

#[derive(Deserialize)]
struct Note {
    note: String,
    duration: f32
}

impl Note {
    // f = 2^(n/12 * 440) where n is the number of semitones above or below A4.
    fn frequency(self: &Self) -> f32 {
        let note: char;


        let relative_octave: i32;
        let mut accidental_offset:i32 = 0;

        match self.note.len() {
            2 => {
                note = self.note.chars().nth(0).unwrap();
                relative_octave = self.note.chars().nth(1).unwrap().to_digit(10).unwrap() as i32 - 4;
            }
            3 => {
                note = self.note.chars().nth(0).unwrap();
                let accidental = self.note.chars().nth(1).unwrap();
                accidental_offset = match accidental {
                    'b' => -1,
                    '#' => 1,
                    _ => {
                        panic!("Invalid accidental: {}", accidental);
                    }
                };
                
                relative_octave = self.note.chars().nth(2).unwrap().to_digit(10).unwrap() as i32 - 4;
            }
            _ => {
                panic!("Invalid note: {}", &self.note);
            }
        }

        // Semitones that A4/B4/C4/etc is from A4
        let n: i32 = match note {
            'A' => 0,
            'B' => 2,
            'C' => -9,
            'D' => -7,
            'E' => -5,
            'F' => -4,
            'G' => -2,
            _ => {
                panic!("Invalid note: {}", note);
            }
        };

        let semitones_from_a4 = n + relative_octave * OCTAVE_SEMITONES + accidental_offset;

        let freq = 2.0_f32.powf(semitones_from_a4 as f32 / 12.0) * A4_FREQ;
        return freq;
    }
}

struct SineWave {
    frequency: f32,
    duration: f32,
    current_sample: f32,
    total_samples: f32
}

impl SineWave {
    fn new(frequency: f32, duration: f32) -> Self {
        Self {
            frequency,
            duration,
            current_sample: 0.0,
            total_samples: duration * SAMPLE_RATE,
        }
    }
}

impl Iterator for SineWave {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.current_sample >= self.total_samples {
            // No more notes
            return None;
        }

        let t = self.current_sample / self.sample_rate() as f32; // time in seconds
        let output = (2.0 * PI * self.frequency * t).sin();

        self.current_sample += 1.0;
        Some(output * 0.5) // reduce amplitude by half to reduce clipping
    }
}

impl Source for SineWave {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        44100
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.duration))
    }
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Available songs:");
        for entry in std::fs::read_dir("songs").expect("Failed to read songs directory") {
            if let Ok(entry) = entry {
                println!("  {}", entry.file_name().to_string_lossy());
            }
        }
        println!("\nUsage: {} <song_name>", args[0]);
        std::process::exit(1);
    }

    let mut song_path = format!("songs/{}", args[1]);
    if !song_path.ends_with(".json") {
        song_path.push_str(".json");
    }

    if !Path::new(&song_path).exists() {
        println!("Song '{}' not found", args[1]);
        std::process::exit(1);
    }

    println!("Playing: {}", args[1]);
    let file_content = std::fs::read_to_string(&song_path)
        .expect("Failed to read song file");

    let notes: Vec<Note> = serde_json::from_str(&file_content)
        .expect("Failed to parse JSON");

    let (_stream, output_stream_handle) = OutputStream::try_default().unwrap();
    let output_sink = Sink::try_new(&output_stream_handle).unwrap();

    for note in notes {
        let freq = note.frequency();
        output_sink.append(SineWave::new(freq, note.duration));
        output_sink.append(SineWave::new(0.0, 0.005));
    }

    output_sink.sleep_until_end();
}
