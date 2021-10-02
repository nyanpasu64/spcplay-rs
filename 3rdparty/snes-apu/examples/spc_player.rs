use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use snes_apu::apu::Apu;
use snes_apu::dsp::dsp::SAMPLE_RATE;

use spc::{Emulator, Spc};

use std::path::Path;
use std::{env, thread};

struct SpcEndState {
    sample_pos: i32,
    fade_out_sample: i32,
    end_sample: i32,
}

fn main() {
    if let Err(e) = do_it() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn do_it() -> Result<()> {
    let mut args = env::args();
    let path = match args.len() {
        0 | 1 => bail!("No file specified"),
        2 => args.nth(1).unwrap(),
        _ => bail!("Only one file argument can be specified"),
    };

    let spc = SpcPlayer::new(Path::new(&path))?;
    let driver = CpalDriver::new(spc)?;
    driver.play()?;
    loop {
        thread::park();
    }
}

fn print_spc_info(path: &Path, spc: &Spc) {
    println!("SPC: {}", path.display());
    println!(" Version Minor: {}", spc.version_minor);
    println!(" PC: {}", spc.pc);
    println!(" A: {}", spc.a);
    println!(" X: {}", spc.x);
    println!(" Y: {}", spc.y);
    println!(" PSW: {}", spc.psw);
    println!(" SP: {}", spc.sp);

    if let Some(ref id666_tag) = spc.id666_tag {
        println!(" ID666 tag present:");
        println!("  Song title: {}", id666_tag.song_title);
        println!("  Game title: {}", id666_tag.game_title);
        println!("  Dumper name: {}", id666_tag.dumper_name);
        println!("  Comments: {}", id666_tag.comments);
        println!("  Date dumped (MM/DD/YYYY): {}", id666_tag.date_dumped);
        println!(
            "  Seconds to play before fading out: {}",
            id666_tag.seconds_to_play_before_fading_out
        );
        println!("  Fade out length: {}ms", id666_tag.fade_out_length);
        println!("  Artist name: {}", id666_tag.artist_name);
        println!(
            "  Default channel disables: {}",
            id666_tag.default_channel_disables
        );
        println!(
            "  Dumping emulator: {}",
            match id666_tag.dumping_emulator {
                Emulator::Unknown => "Unknown",
                Emulator::ZSnes => "ZSnes",
                Emulator::Snes9x => "Snes9x",
            }
        );
    } else {
        println!(" No ID666 tag present.");
    };
}

struct SpcPlayer {
    apu: Box<Apu>,
    end_state: Option<SpcEndState>,
}

type FramesWritten = usize;

impl SpcPlayer {
    fn new(path: &Path) -> Result<SpcPlayer> {
        let spc = Spc::load(&path).context("Could not load spc file")?;

        print_spc_info(path, &spc);

        let mut apu = Apu::from_spc(&spc);
        // Most SPC's have crap in the echo buffer on startup, so while it's not technically correct, we'll clear that.
        // The example for blargg's APU emulator (which is known to be the most accurate there is) also does this, so I
        //  think we're OK to do it too :)
        apu.clear_echo_buffer();

        let end_state: Option<SpcEndState> = if let Some(ref id666_tag) = spc.id666_tag {
            let fade_out_sample =
                id666_tag.seconds_to_play_before_fading_out * (SAMPLE_RATE as i32);
            let end_sample =
                fade_out_sample + id666_tag.fade_out_length * (SAMPLE_RATE as i32) / 1000;
            Some(SpcEndState {
                sample_pos: 0,
                fade_out_sample: fade_out_sample,
                end_sample: end_sample,
            })
        } else {
            None
        };

        Ok(SpcPlayer { apu, end_state })
    }

    fn render(&mut self, out: &mut [i16]) -> FramesWritten {
        self.apu.render_interleaved(&mut *out);
        // TODO handle pausing and fade out
        out.len() / 2
    }
}

struct CpalDriver {
    stream: cpal::Stream,
}

impl CpalDriver {
    pub fn new(mut spc: SpcPlayer) -> Result<CpalDriver> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .context("no input device available")?;

        let supported_config: cpal::SupportedStreamConfig = {
            let supported_config_ranges: Vec<cpal::SupportedStreamConfigRange> = device
                .supported_output_configs()
                .context("error while querying configs")?
                .collect();

            let range: cpal::SupportedStreamConfigRange = supported_config_ranges
                .into_iter()
                .find(|range| {
                    range.channels() == 2 && range.sample_format() == cpal::SampleFormat::I16
                })
                .context("no supported config found")?;

            let min = range.min_sample_rate().0;
            let max = range.max_sample_rate().0;
            if !(min <= 32000 && 32000 <= max) {
                bail!(
                    "invalid sampling range {} to {} does not include 32000",
                    min,
                    max,
                );
            }

            range.with_sample_rate(cpal::SampleRate(32000))
        };

        // For some reason, converting SupportedStreamConfig into StreamConfig
        // (SupportedStreamConfig::config())
        // throws away buffer_size and replaces with BufferSize::Default.
        let config: cpal::StreamConfig = supported_config.into();

        let err_fn = |err| eprintln!("an error occurred on the input audio stream: {}", err);

        let stream = device
            .build_output_stream(
                &config,
                move |data, _info| {
                    spc.render(data);
                },
                err_fn,
            )
            .context("Error building output stream")?;

        Ok(CpalDriver { stream })
    }

    pub fn play(&self) -> Result<()> {
        println!("Playing audio device...");
        self.stream.play().context("Error playing audio device")?;
        Ok(())
    }
}
