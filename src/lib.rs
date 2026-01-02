use nih_plug::log::*;
use nih_plug::prelude::*;
use rubato::Resampler;
use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::playing_sample::PlayingSample;

pub mod playing_sample;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

#[derive(Default, Clone)]
pub struct LoadedSample(Arc<[Arc<[f32]>]>);

struct OneNine {
    pub params: Arc<OneNineParams>,
    pub samples: Arc<[Arc<[LoadedSample]>]>,
    pub sample_rate: f32,
    pub note_mappings: FxHashMap<u8, usize>,
    pub playing_samples: Vec<PlayingSample>,
}

#[derive(Params)]
struct OneNineParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "Base Drum Select"]
    pub bd_i: IntParam,
    #[id = "Base Drum Volume"]
    pub bd_vol: FloatParam,

    #[id = "Crash Select"]
    pub cr_i: IntParam,
    #[id = "Crash Volume"]
    pub cr_vol: FloatParam,

    #[id = "Clap Select"]
    pub c_i: IntParam,
    #[id = "Clap Volume"]
    pub c_vol: FloatParam,

    #[id = "HiHat Closed Select"]
    pub hhc_i: IntParam,
    #[id = "HiHat Closed Volume"]
    pub hhc_vol: FloatParam,

    #[id = "HiHat Open Select"]
    pub hho_i: IntParam,
    #[id = "HiHat Open Volume"]
    pub hho_vol: FloatParam,

    #[id = "Ride Select"]
    pub ride_i: IntParam,
    #[id = "Ride Volume"]
    pub ride_vol: FloatParam,

    #[id = "Rimshot Volume"]
    pub rs_vol: FloatParam,

    #[id = "Snare Select"]
    pub sn_i: IntParam,
    #[id = "Snare Volume"]
    pub sn_vol: FloatParam,
}

fn uninterleave(samples: Vec<f32>, channels: usize) -> LoadedSample {
    // input looks like:
    // [a, b, a, b, a, b, ...]
    //
    // output should be:
    // [
    //    [a, a, a, ...],
    //    [b, b, b, ...]
    // ]

    let mut new_samples = vec![Vec::with_capacity(samples.len() / channels); channels];

    for sample_chunk in samples.chunks(channels) {
        // sample_chunk is a chunk like [a, b]
        for (i, sample) in sample_chunk.into_iter().enumerate() {
            new_samples[i].push(sample.clone());
        }
    }

    LoadedSample(
        new_samples
            .into_iter()
            .map(|channel| channel.into())
            .collect::<Vec<_>>()
            .into(),
    )
}

fn resample(samples: LoadedSample, sample_rate_in: f32, sample_rate_out: f32) -> LoadedSample {
    let samples = samples.0;
    let mut resampler = rubato::FftFixedIn::<f32>::new(
        sample_rate_in as usize,
        sample_rate_out as usize,
        samples[0].len(),
        8,
        samples.len(),
    )
    .unwrap();

    match resampler.process(&samples, None) {
        Ok(mut waves_out) => {
            // get the duration of leading silence introduced by FFT
            // https://github.com/HEnquist/rubato/blob/52cdc3eb8e2716f40bc9b444839bca067c310592/src/synchro.rs#L654
            let silence_len = resampler.output_delay();

            for channel in waves_out.iter_mut() {
                channel.drain(..silence_len);
                channel.shrink_to_fit();
            }

            LoadedSample(
                waves_out
                    .into_iter()
                    .map(|channel| channel.into())
                    .collect::<Vec<_>>()
                    .into(),
            )
        }
        Err(_) => LoadedSample(vec![].into()),
    }
}

impl Default for OneNine {
    fn default() -> Self {
        // let samples = Self::load_samples();
        let samples = Vec::default().into();
        let mut note_mappings = FxHashMap::default();
        // Base/Kick Drum
        note_mappings.insert(36, 0);
        // Crash
        note_mappings.insert(57, 1);
        // Clap
        note_mappings.insert(39, 2);
        // Closed HiHat
        note_mappings.insert(42, 3);
        // Open HiHat
        note_mappings.insert(46, 4);
        // Ride
        note_mappings.insert(59, 5);
        // Rimshot
        note_mappings.insert(37, 6);
        // Snare Drum
        note_mappings.insert(40, 7);

        Self {
            params: Arc::new(OneNineParams::default()),
            samples,
            sample_rate: 48_000.0,
            note_mappings,
            playing_samples: Vec::with_capacity(8),
        }
    }
}

impl OneNine {
    fn load_samples(&mut self) {
        self.samples = vec![
            // 0
            vec![
                self.load_sample(include_bytes!("../samples/BD0.WAV")),
                self.load_sample(include_bytes!("../samples/BD1.WAV")),
                self.load_sample(include_bytes!("../samples/BD2.WAV")),
                self.load_sample(include_bytes!("../samples/BD3.WAV")),
                self.load_sample(include_bytes!("../samples/BD4.WAV")),
                self.load_sample(include_bytes!("../samples/BD5.WAV")),
            ]
            .into(),
            // 1
            vec![
                self.load_sample(include_bytes!("../samples/CR0.WAV")),
                self.load_sample(include_bytes!("../samples/CR1.WAV")),
            ]
            .into(),
            // 2
            vec![
                self.load_sample(include_bytes!("../samples/HC0.WAV")),
                self.load_sample(include_bytes!("../samples/HC1.WAV")),
            ]
            .into(),
            // 3
            // closed HH
            vec![
                self.load_sample(include_bytes!("../samples/HH3.WAV")),
                self.load_sample(include_bytes!("../samples/HH4.WAV")),
                self.load_sample(include_bytes!("../samples/HH5.WAV")),
                self.load_sample(include_bytes!("../samples/HHX.WAV")),
            ]
            .into(),
            // 4
            // open HH
            vec![
                self.load_sample(include_bytes!("../samples/HH0.WAV")),
                self.load_sample(include_bytes!("../samples/HH1.WAV")),
                self.load_sample(include_bytes!("../samples/HH2.WAV")),
            ]
            .into(),
            // 5
            vec![
                self.load_sample(include_bytes!("../samples/RD0.WAV")),
                self.load_sample(include_bytes!("../samples/RD1.WAV")),
            ]
            .into(),
            // 6
            vec![self.load_sample(include_bytes!("../samples/RS1.WAV"))].into(),
            // 7
            vec![
                self.load_sample(include_bytes!("../samples/SN0.WAV")),
                self.load_sample(include_bytes!("../samples/SN1.WAV")),
                self.load_sample(include_bytes!("../samples/SN2.WAV")),
                self.load_sample(include_bytes!("../samples/SN3.WAV")),
                self.load_sample(include_bytes!("../samples/SN4.WAV")),
                self.load_sample(include_bytes!("../samples/SN5.WAV")),
                self.load_sample(include_bytes!("../samples/SN6.WAV")),
                self.load_sample(include_bytes!("../samples/SN7.WAV")),
            ]
            .into(),
        ]
        .into();
    }

    fn load_sample(&self, wav_bytes: &[u8]) -> LoadedSample {
        // wav only for now
        let cursor = std::io::Cursor::new(wav_bytes);
        let reader = hound::WavReader::new(cursor);

        if let Ok(mut reader) = reader {
            let spec = reader.spec();
            let sample_rate = spec.sample_rate as f32;
            let channels = spec.channels as usize;

            let interleaved_samples = match spec.sample_format {
                hound::SampleFormat::Int => reader
                    .samples::<i32>()
                    .map(|s| (s.unwrap_or_default() as f32 * 256.0) / i32::MAX as f32)
                    .collect::<Vec<f32>>(),
                hound::SampleFormat::Float => reader
                    .samples::<f32>()
                    .map(|s| s.unwrap_or_default())
                    .collect::<Vec<f32>>(),
            };

            let mut samples = uninterleave(interleaved_samples, channels);

            // resample if needed
            if sample_rate != self.sample_rate {
                samples = resample(samples, sample_rate, self.sample_rate);
            }

            // self.loaded_samples.insert(path.clone(), samples);

            samples
        } else {
            LoadedSample::default()
        }
    }
}

impl Default for OneNineParams {
    fn default() -> Self {
        Self {
            bd_i: IntParam::new("Base Drum Select", 0, IntRange::Linear { min: 0, max: 5 }),
            bd_vol: FloatParam::new(
                "Bass Drum Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            cr_i: IntParam::new("Crash Select", 0, IntRange::Linear { min: 0, max: 1 }),
            cr_vol: FloatParam::new(
                "Crash Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            c_i: IntParam::new("Clap Select", 1, IntRange::Linear { min: 0, max: 1 }),
            c_vol: FloatParam::new(
                "Clap Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            hhc_i: IntParam::new(
                "HiHat Closed Select",
                0,
                IntRange::Linear { min: 0, max: 3 },
            ),
            hhc_vol: FloatParam::new(
                "HiHat Closed Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            hho_i: IntParam::new("HiHat Open Select", 0, IntRange::Linear { min: 0, max: 2 }),
            hho_vol: FloatParam::new(
                "HiHat Open Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            ride_i: IntParam::new("Ride Select", 0, IntRange::Linear { min: 0, max: 1 }),
            ride_vol: FloatParam::new(" Volume", 0.8, FloatRange::Linear { min: 0.0, max: 1.0 }),
            rs_vol: FloatParam::new(
                "Rimshot Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            sn_i: IntParam::new("Snare Select", 0, IntRange::Linear { min: 0, max: 7 }),
            sn_vol: FloatParam::new(
                "Snare Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
        }
    }
}

impl Plugin for OneNine {
    const NAME: &'static str = "1.909";
    const VENDOR: &'static str = "Calacuda";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "pls-dont-email-me";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(0),
        main_output_channels: NonZeroU32::new(1),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        info!("changed sample rate to {}", buffer_config.sample_rate);

        self.sample_rate = buffer_config.sample_rate;
        self.load_samples();

        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Base/Kick Drum
        // Crash
        // Clap
        // Closed HiHat
        // Open HiHat
        // Ride
        // Rimshot
        // Snare Drum
        let params = [
            (self.params.bd_i.value(), self.params.bd_vol.value()),
            (self.params.cr_i.value(), self.params.cr_vol.value()),
            (self.params.c_i.value(), self.params.c_vol.value()),
            (self.params.hhc_i.value(), self.params.hhc_vol.value()),
            (self.params.hho_i.value(), self.params.hho_vol.value()),
            (self.params.ride_i.value(), self.params.ride_vol.value()),
            (0, self.params.rs_vol.value()),
            (self.params.sn_i.value(), self.params.sn_vol.value()),
        ];

        while let Some(event) = context.next_event() {
            // info!("recieved event");

            match event {
                NoteEvent::NoteOn {
                    timing: _,
                    voice_id: _,
                    channel: _,
                    note,
                    velocity,
                } => {
                    // info!("playing {note}");

                    if let Some(sample_sel) = self.note_mappings.get(&note) {
                        let param = params[*sample_sel];
                        let sample_i = param.0 as usize;
                        let sample = (*sample_sel, sample_i);
                        let sample_len = self.samples[*sample_sel][sample_i].0[0].len();
                        // info!("sample: {sample:?}");

                        let sample_cursor =
                            PlayingSample::new(sample, sample_len, velocity * param.1);
                        self.playing_samples
                            .retain(|sample_cursor| sample_cursor.sample.0 != sample.0);
                        self.playing_samples.push(sample_cursor);
                        // info!("")
                    }
                }
                NoteEvent::NoteOff {
                    timing: _,
                    voice_id: _,
                    channel: _,
                    note,
                    velocity: _,
                } => {
                    if let Some(sample_sel) = self.note_mappings.get(&note) {
                        self.playing_samples
                            .retain(|sample_cursor| sample_cursor.sample.0 != *sample_sel);
                    }
                }
                _ => {}
            }
        }

        for channel_samples in buffer.iter_samples() {
            let value: f32 = self
                .playing_samples
                .iter_mut()
                .filter_map(|sample_cursor| {
                    sample_cursor.step().map(|sample_position| {
                        self.samples[sample_cursor.sample.0][sample_cursor.sample.1].0[0]
                            [sample_position]
                            * sample_cursor.gain
                    })
                })
                .sum();
            let value = value.tanh();

            self.playing_samples
                .retain(|sample_cursor| !sample_cursor.is_done());

            for sample in channel_samples {
                *sample = value;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for OneNine {
    const CLAP_ID: &'static str = "online.eoghan-west.1_909";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("sample based, 909 style drum machine");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Drum,
        ClapFeature::Sampler,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for OneNine {
    const VST3_CLASS_ID: [u8; 16] = *b"1.909\0\0\0\0\0\0\0\0\0\0\0";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Drum,
        Vst3SubCategory::Sampler,
        Vst3SubCategory::Mono,
    ];
}

nih_export_clap!(OneNine);
nih_export_vst3!(OneNine);
