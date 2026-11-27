# /// script
# requires-python = ">=3.10"
# dependencies = ["numpy", "kaldi-native-fbank"]
# ///
"""Writes the kaldi-native-fbank reference of the CAM++ front end test
(`src-tauri/src/diarization/fbank.rs`): the raw 80 bin log fbank of an
analytic signal, f32 little-endian, frame-major.

Usage: uv run scripts/gen-fbank-fixture.py
"""

from pathlib import Path

import kaldi_native_fbank as knf
import numpy as np

OUT = Path(__file__).parent.parent / "src-tauri/tests/fixtures/diarization/fbank-reference.bin"

# Same formula as `signal()` in fbank.rs: f64, then cast to f32.
t = np.arange(16000 + 77, dtype=np.float64) / 16000
tau = 2 * np.pi
chirp = np.sin(tau * (100 * t + 1500 * t * t))
tone = 0.3 * np.sin(tau * 440 * t)
envelope = 0.5 + 0.4 * np.sin(tau * 1.5 * t)
signal = (0.3 * envelope * (chirp + tone) + 0.01).astype(np.float32)

opts = knf.FbankOptions()
opts.frame_opts.dither = 0
opts.frame_opts.samp_freq = 16000
opts.mel_opts.num_bins = 80
fbank = knf.OnlineFbank(opts)
fbank.accept_waveform(16000, signal.tolist())
fbank.input_finished()
feats = np.stack([fbank.get_frame(i) for i in range(fbank.num_frames_ready)]).astype("<f4")
OUT.write_bytes(feats.tobytes())
print(f"{OUT}: {feats.shape}")
