use std::fs;
use std::path::{Path, PathBuf};

use hound::{SampleFormat, WavReader};

pub(super) const ASR_SAMPLE_RATE: u32 = 16_000;

pub(super) fn canonicalize_existing_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Path does not exist: {}", path.display()),
        ));
    }
    fs::canonicalize(&path).map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
}

pub(super) fn decode_wav_file(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let mut reader = WavReader::open(path)
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            "WAV file has zero channels",
        ));
    }
    if spec.sample_rate == 0 {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            "WAV file has zero sample rate",
        ));
    }

    let samples = match spec.sample_format {
        SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample.map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
            })
            .collect::<Result<Vec<_>, _>>()?,
        SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|sample| {
                sample
                    .map(|value| value as f32 / i16::MAX as f32)
                    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
            })
            .collect::<Result<Vec<_>, _>>()?,
        SampleFormat::Int if spec.bits_per_sample <= 32 => {
            let scale = ((1_i64 << (spec.bits_per_sample.saturating_sub(1) as u32)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / scale)
                        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        _ => {
            return Err(crate::utils::err_msg(
                module_path!(),
                line!(),
                format!(
                    "Unsupported WAV format: {:?} {} bits",
                    spec.sample_format, spec.bits_per_sample
                ),
            ));
        }
    };

    let mono = downmix_to_mono(&samples, spec.channels as usize);
    Ok((resample_to_16khz(&mono, spec.sample_rate), ASR_SAMPLE_RATE))
}

pub(super) fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }

    let mut mono = Vec::with_capacity(samples.len() / channels.max(1));
    for frame in samples.chunks(channels) {
        let sum: f32 = frame.iter().copied().sum();
        mono.push(sum / frame.len() as f32);
    }
    mono
}

pub(super) fn resample_to_16khz(samples: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == ASR_SAMPLE_RATE || samples.is_empty() {
        return samples.to_vec();
    }

    let output_len =
        ((samples.len() as u64 * ASR_SAMPLE_RATE as u64) / source_rate as u64).max(1) as usize;
    let step = source_rate as f64 / ASR_SAMPLE_RATE as f64;
    let mut output = Vec::with_capacity(output_len);

    for index in 0..output_len {
        let source_position = index as f64 * step;
        let left_index = source_position.floor() as usize;
        let right_index = (left_index + 1).min(samples.len().saturating_sub(1));
        let fraction = (source_position - left_index as f64) as f32;
        let left = samples[left_index];
        let right = samples[right_index];
        output.push(left + (right - left) * fraction);
    }

    output
}

pub(super) fn decode_pcm_bytes(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if !bytes.len().is_multiple_of(4) {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            format!(
                "PCM payload length {} is not a multiple of 4 (f32 LE)",
                bytes.len()
            ),
        ));
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}
