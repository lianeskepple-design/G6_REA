use std::fs;
use std::path::Path;

use crate::error::AppError;

pub const FORMAT_PCM: u16 = 1;
pub const FORMAT_IEEE_FLOAT: u16 = 3;
pub const FORMAT_EXTENSIBLE: u16 = 0xFFFE;

/// Upper bound we are willing to handle; anything bigger is a malformed header.
const MAX_CHANNELS: u16 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    Pcm16,
    Pcm24,
    Pcm32,
    Float32,
}

impl SampleFormat {
    pub fn bits(self) -> u16 {
        match self {
            SampleFormat::Pcm16 => 16,
            SampleFormat::Pcm24 => 24,
            SampleFormat::Pcm32 | SampleFormat::Float32 => 32,
        }
    }

    pub fn bytes(self) -> usize {
        self.bits() as usize / 8
    }

    pub fn format_tag(self) -> u16 {
        match self {
            SampleFormat::Float32 => FORMAT_IEEE_FLOAT,
            _ => FORMAT_PCM,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SampleFormat::Pcm16 => "pcm16",
            SampleFormat::Pcm24 => "pcm24",
            SampleFormat::Pcm32 => "pcm32",
            SampleFormat::Float32 => "float32",
        }
    }

    pub fn from_label(text: &str) -> Option<SampleFormat> {
        match text.to_ascii_lowercase().as_str() {
            "pcm16" | "s16" | "int16" => Some(SampleFormat::Pcm16),
            "pcm24" | "s24" | "int24" => Some(SampleFormat::Pcm24),
            "pcm32" | "s32" | "int32" => Some(SampleFormat::Pcm32),
            "float32" | "f32" | "float" => Some(SampleFormat::Float32),
            _ => None,
        }
    }
}

/// Interleaved float samples; `samples.len()` is always `frames * channels`.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, AppError> {
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(AppError::InvalidInput(format!(
                "unsupported channel count {channels} (expected 1..={MAX_CHANNELS})"
            )));
        }
        if sample_rate == 0 {
            return Err(AppError::InvalidInput(
                "sample rate must be greater than zero".to_string(),
            ));
        }
        let channels_usize = channels as usize;
        if samples.len() % channels_usize != 0 {
            return Err(AppError::InvalidInput(format!(
                "sample count {} is not a whole number of {channels_usize}-channel frames",
                samples.len()
            )));
        }
        Ok(AudioBuffer {
            sample_rate,
            channels,
            frames: samples.len() / channels_usize,
            samples,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WavRead {
    pub buffer: AudioBuffer,
    pub format: SampleFormat,
    /// Non-fatal deviations from a strictly canonical container.
    pub warnings: Vec<String>,
}

fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn plugin_name(fourcc: &[u8]) -> String {
    String::from_utf8_lossy(fourcc)
        .trim_end_matches('\0')
        .to_string()
}

pub fn read_wav(path: &Path) -> Result<WavRead, AppError> {
    let bytes = fs::read(path).map_err(|e| {
        AppError::Io(format!(
            "cannot read input '{}': {e}",
            path.display()
        ))
    })?;
    decode_wav(&bytes, &path.display().to_string())
}

/// Decode a WAV image held in memory. `origin` is only used in error messages.
pub fn decode_wav(bytes: &[u8], origin: &str) -> Result<WavRead, AppError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AppError::Unsupported(format!(
            "{origin}: not a RIFF/WAVE file"
        )));
    }

    let mut warnings = Vec::new();
    let mut fmt: Option<(u16, u16, u32, u16, u16)> = None; // tag, channels, rate, bits, block_align
    let mut data: Option<(usize, usize)> = None;

    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let declared = read_u32(bytes, pos + 4) as usize;
        let body = pos + 8;
        let available = bytes.len() - body;

        if &fourcc[..4] == b"data" {
            if declared == 0 && available > 0 {
                // Some writers leave a zero length and put the payload inline.
                warnings.push("data chunk declares 0 bytes; using the remainder of the file".into());
            }
            let size = declared.min(available);
            if declared > available {
                warnings.push(format!(
                    "data chunk declares {declared} bytes but only {available} are present; truncating"
                ));
            }
            if data.is_none() {
                data = Some((body, size));
            } else {
                warnings.push("extra data chunk ignored (only the first is processed)".into());
            }
        } else {
            if declared > available {
                return Err(AppError::InvalidInput(format!(
                    "{origin}: chunk '{}' declares {declared} bytes but the file ends after {available}",
                    plugin_name(fourcc)
                )));
            }
            if fourcc == b"fmt " {
                if declared < 16 {
                    return Err(AppError::InvalidInput(format!(
                        "{origin}: fmt chunk is {declared} bytes, expected at least 16"
                    )));
                }
                let mut tag = read_u16(bytes, body);
                let channels = read_u16(bytes, body + 2);
                let sample_rate = read_u32(bytes, body + 4);
                let block_align = read_u16(bytes, body + 12);
                let bits = read_u16(bytes, body + 14);
                if tag == FORMAT_EXTENSIBLE {
                    if declared < 40 {
                        return Err(AppError::InvalidInput(format!(
                            "{origin}: WAVE_FORMAT_EXTENSIBLE fmt chunk is too short"
                        )));
                    }
                    let sub_tag = read_u32(bytes, body + 24) as u16;
                    tag = sub_tag;
                }
                fmt = Some((tag, channels, sample_rate, bits, block_align));
            }
        }

        // Chunks are word aligned.
        let advance = 8 + declared + (declared & 1);
        if advance == 0 {
            break;
        }
        pos = pos.saturating_add(advance);
        if pos > bytes.len() {
            break;
        }
    }

    let (tag, channels, sample_rate, bits, block_align) = fmt.ok_or_else(|| {
        AppError::Unsupported(format!("{origin}: no fmt chunk found"))
    })?;
    let (data_at, data_len) = data.ok_or_else(|| {
        AppError::Unsupported(format!("{origin}: no data chunk found"))
    })?;

    if channels == 0 || channels > MAX_CHANNELS {
        return Err(AppError::Unsupported(format!(
            "{origin}: unsupported channel count {channels}"
        )));
    }
    if sample_rate == 0 {
        return Err(AppError::InvalidInput(format!(
            "{origin}: sample rate is zero"
        )));
    }

    let format = match (tag, bits) {
        (FORMAT_PCM, 16) => SampleFormat::Pcm16,
        (FORMAT_PCM, 24) => SampleFormat::Pcm24,
        (FORMAT_PCM, 32) => SampleFormat::Pcm32,
        (FORMAT_IEEE_FLOAT, 32) => SampleFormat::Float32,
        (FORMAT_PCM, other) => {
            return Err(AppError::Unsupported(format!(
                "{origin}: {other}-bit integer PCM is not supported (pcm16/pcm24/pcm32/float32 only)"
            )))
        }
        (FORMAT_IEEE_FLOAT, other) => {
            return Err(AppError::Unsupported(format!(
                "{origin}: {other}-bit float is not supported (float32 only)"
            )))
        }
        (other, _) => {
            return Err(AppError::Unsupported(format!(
                "{origin}: WAV format tag 0x{other:04X} is not supported (pcm/float only)"
            )))
        }
    };

    let expected_align = channels as usize * format.bytes();
    if block_align as usize != expected_align {
        return Err(AppError::Unsupported(format!(
            "{origin}: block alignment {} does not match {channels} channel(s) of {} (expected {expected_align})",
            block_align,
            format.label()
        )));
    }

    let frames = data_len / expected_align;
    let usable = frames * expected_align;
    if usable != data_len {
        warnings.push(format!(
            "data chunk has {} trailing byte(s) that do not form a whole frame; ignored",
            data_len - usable
        ));
    }

    let mut samples = Vec::with_capacity(frames * channels as usize);
    let payload = &bytes[data_at..data_at + usable];
    match format {
        SampleFormat::Pcm16 => {
            for chunk in payload.chunks_exact(2) {
                let v = i16::from_le_bytes([chunk[0], chunk[1]]);
                samples.push(v as f32 / 32768.0);
            }
        }
        SampleFormat::Pcm24 => {
            for chunk in payload.chunks_exact(3) {
                let raw = (chunk[0] as i32) | ((chunk[1] as i32) << 8) | ((chunk[2] as i32) << 16);
                let v = (raw << 8) >> 8; // sign extend
                samples.push(v as f32 / 8_388_608.0);
            }
        }
        SampleFormat::Pcm32 => {
            for chunk in payload.chunks_exact(4) {
                let v = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                samples.push(v as f32 / 2_147_483_648.0);
            }
        }
        SampleFormat::Float32 => {
            for (index, chunk) in payload.chunks_exact(4).enumerate() {
                let v = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if !v.is_finite() {
                    return Err(AppError::InvalidInput(format!(
                        "{origin}: sample {index} is {v}; NaN/Inf input is rejected"
                    )));
                }
                samples.push(v);
            }
        }
    }

    let buffer = AudioBuffer {
        sample_rate,
        channels,
        frames,
        samples,
    };

    Ok(WavRead {
        buffer,
        format,
        warnings,
    })
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_fourcc(out: &mut Vec<u8>, tag: &[u8; 4]) {
    out.extend_from_slice(tag);
}

fn encode_sample(out: &mut Vec<u8>, value: f32, format: SampleFormat) {
    match format {
        SampleFormat::Pcm16 => {
            let v = (value.clamp(-1.0, 1.0) as f64 * 32767.0).round() as i16;
            out.extend_from_slice(&v.to_le_bytes());
        }
        SampleFormat::Pcm24 => {
            let v = (value.clamp(-1.0, 1.0) as f64 * 8_388_607.0).round() as i32;
            out.extend_from_slice(&v.to_le_bytes()[0..3]);
        }
        SampleFormat::Pcm32 => {
            let v = (value.clamp(-1.0, 1.0) as f64 * 2_147_483_647.0).round() as i32;
            out.extend_from_slice(&v.to_le_bytes());
        }
        SampleFormat::Float32 => out.extend_from_slice(&value.to_le_bytes()),
    }
}

/// Encode a WAV image. Multichannel (>2) output uses WAVE_FORMAT_EXTENSIBLE so
/// the channel layout is explicitly typed, matching common decoders.
pub fn encode_wav(buffer: &AudioBuffer, format: SampleFormat) -> Vec<u8> {
    let channels = buffer.channels;
    let bytes_per_sample = format.bytes() as u16;
    let block_align = channels * bytes_per_sample;
    let data_len = buffer.samples.len() * format.bytes();
    let extensible = channels > 2;

    let mut out = Vec::with_capacity(data_len + 80);
    push_fourcc(&mut out, b"RIFF");
    push_u32(&mut out, (4 + 8 + if extensible { 40 } else { 16 } + 8 + data_len) as u32);
    push_fourcc(&mut out, b"WAVE");

    push_fourcc(&mut out, b"fmt ");
    if extensible {
        push_u32(&mut out, 40);
        push_u16(&mut out, FORMAT_EXTENSIBLE);
        push_u16(&mut out, channels);
        push_u32(&mut out, buffer.sample_rate);
        push_u32(&mut out, buffer.sample_rate * block_align as u32);
        push_u16(&mut out, block_align);
        push_u16(&mut out, format.bits());
        push_u16(&mut out, 22); // cbSize
        push_u16(&mut out, format.bits()); // valid bits
        push_u32(&mut out, 0); // channel mask: unspecified
        push_u32(&mut out, format.format_tag() as u32);
        out.extend_from_slice(&[
            0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
        ]);
    } else {
        push_u32(&mut out, 16);
        push_u16(&mut out, format.format_tag());
        push_u16(&mut out, channels);
        push_u32(&mut out, buffer.sample_rate);
        push_u32(&mut out, buffer.sample_rate * block_align as u32);
        push_u16(&mut out, block_align);
        push_u16(&mut out, format.bits());
    }

    push_fourcc(&mut out, b"data");
    push_u32(&mut out, data_len as u32);
    for &sample in &buffer.samples {
        encode_sample(&mut out, sample, format);
    }
    if data_len % 2 == 1 {
        out.push(0);
    }
    out
}

pub fn write_wav(path: &Path, buffer: &AudioBuffer, format: SampleFormat) -> Result<(), AppError> {
    let bytes = encode_wav(buffer, format);
    fs::write(path, bytes)
        .map_err(|e| AppError::Io(format!("cannot write '{}': {e}", path.display())))
}
