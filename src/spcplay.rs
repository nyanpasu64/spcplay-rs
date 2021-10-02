use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use snes_apu::apu::Apu;
use snes_apu::dsp::dsp::SAMPLE_RATE;

use spc::{Emulator, Spc};

use std::path::{Path, PathBuf};

struct SpcEndState {
    sample_pos: i32,
    fade_out_sample: i32,
    end_sample: i32,
}

fn get_spc_info(path: &Path, spc: &Spc) -> String {
    let mut buf = String::new();
    use std::fmt::Write;

    (|| -> Result<(), std::fmt::Error> {
        writeln!(buf, "SPC: {}", path.display())?;
        writeln!(buf, " Version Minor: {}", spc.version_minor)?;
        writeln!(buf, " PC: {}", spc.pc)?;
        writeln!(buf, " A: {}", spc.a)?;
        writeln!(buf, " X: {}", spc.x)?;
        writeln!(buf, " Y: {}", spc.y)?;
        writeln!(buf, " PSW: {}", spc.psw)?;
        writeln!(buf, " SP: {}", spc.sp)?;

        if let Some(ref id666_tag) = spc.id666_tag {
            writeln!(buf, " ID666 tag present:")?;
            writeln!(buf, "  Song title: {}", id666_tag.song_title)?;
            writeln!(buf, "  Game title: {}", id666_tag.game_title)?;
            writeln!(buf, "  Dumper name: {}", id666_tag.dumper_name)?;
            writeln!(buf, "  Comments: {}", id666_tag.comments)?;
            writeln!(buf, "  Date dumped (MM/DD/YYYY): {}", id666_tag.date_dumped)?;
            writeln!(
                buf,
                "  Seconds to play before fading out: {}",
                id666_tag.seconds_to_play_before_fading_out
            )?;
            writeln!(buf, "  Fade out length: {}ms", id666_tag.fade_out_length)?;
            writeln!(buf, "  Artist name: {}", id666_tag.artist_name)?;
            writeln!(
                buf,
                "  Default channel disables: {}",
                id666_tag.default_channel_disables
            )?;
            writeln!(
                buf,
                "  Dumping emulator: {}",
                match id666_tag.dumping_emulator {
                    Emulator::Unknown => "Unknown",
                    Emulator::ZSnes => "ZSnes",
                    Emulator::Snes9x => "Snes9x",
                }
            )?;
        } else {
            writeln!(buf, " No ID666 tag present.")?;
        }

        Ok(())
    })()
    .expect("a formatting trait implementation returned an error");

    buf
}

pub struct SpcPlayer {
    path: PathBuf,
    spc: Spc,
    apu: Box<Apu>,
    end_state: Option<SpcEndState>,
}

pub type FramesWritten = usize;

impl SpcPlayer {
    pub fn new(path: &Path) -> Result<SpcPlayer> {
        let spc = Spc::load(&path).context("Could not load spc file")?;

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

        Ok(SpcPlayer {
            path: path.to_owned(),
            spc,
            apu,
            end_state,
        })
    }

    pub fn get_spc_info(&self) -> String {
        get_spc_info(&self.path, &self.spc)
    }

    pub fn render(&mut self, out: &mut [i16]) -> FramesWritten {
        self.apu.render_interleaved(&mut *out);
        // TODO handle pausing and fade out
        out.len() / 2
    }
}

pub struct CpalDriver {
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
        self.stream.play().context("Error playing audio device")?;
        Ok(())
    }
}
