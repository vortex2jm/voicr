import type { InstrumentConfig, PlayMode, VoicingResult } from '../types/fret';

let sharedAudioCtx: AudioContext | null = null;
let sharedMasterGain: GainNode | null = null;

function getAudioEngine(): { ctx: AudioContext; destination: AudioNode } {
  if (!sharedAudioCtx) {
    const AudioCtx =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    sharedAudioCtx = new AudioCtx();

    const compressor = sharedAudioCtx.createDynamicsCompressor();
    compressor.threshold.value = -14;
    compressor.knee.value = 18;
    compressor.ratio.value = 4;
    compressor.attack.value = 0.003;
    compressor.release.value = 0.18;

    const bodyFilter = sharedAudioCtx.createBiquadFilter();
    bodyFilter.type = 'lowpass';
    bodyFilter.frequency.value = 6200;
    bodyFilter.Q.value = 0.7;

    const masterGain = sharedAudioCtx.createGain();
    masterGain.gain.value = 0.72;

    bodyFilter.connect(compressor);
    compressor.connect(masterGain);
    masterGain.connect(sharedAudioCtx.destination);

    sharedMasterGain = bodyFilter;
  }

  if (sharedAudioCtx.state === 'suspended') {
    void sharedAudioCtx.resume();
  }

  return {
    ctx: sharedAudioCtx,
    destination: sharedMasterGain ?? sharedAudioCtx.destination,
  };
}

/**
 * Converts a MIDI note number (possibly fractional for slight unison chorus)
 * into frequency in Hz using A4 = 440 Hz.
 */
export function midiToFrequency(midi: number): number {
  return 440 * Math.pow(2, (midi - 69) / 12);
}

/**
 * Synthesizes a plucked string waveform using fractional-delay Karplus-Strong.
 */
function createKarplusStrongBuffer(
  ctx: AudioContext,
  frequencyHz: number,
  durationSec: number,
  brightness = 0.5,
): AudioBuffer {
  const sampleRate = ctx.sampleRate;
  const totalSamples = Math.max(1, Math.floor(sampleRate * durationSec));
  const buffer = ctx.createBuffer(1, totalSamples, sampleRate);
  const output = buffer.getChannelData(0);

  // Exact fractional delay length for accurate tuning at high frets
  const exactPeriod = Math.max(2, sampleRate / Math.max(30, frequencyHz));
  const delayLen = Math.floor(exactPeriod);
  const frac = exactPeriod - delayLen;
  const ringSize = delayLen + 2;
  const ring = new Float32Array(ringSize);

  // Pick-position comb-filtered white noise burst to emulate acoustic pluck
  const pickPos = 0.22;
  const combDelay = Math.max(1, Math.floor(exactPeriod * pickPos));
  const rawNoise = new Float32Array(ringSize);
  let lpPrev = 0;
  for (let i = 0; i < ringSize; i++) {
    const white = Math.random() * 2 - 1;
    // Gentle 1-pole lowpass on excitation so treble isn't harsh
    lpPrev = lpPrev + (0.45 + 0.35 * brightness) * (white - lpPrev);
    rawNoise[i] = lpPrev;
  }
  for (let i = 0; i < ringSize; i++) {
    const delayed = i >= combDelay ? rawNoise[i - combDelay] : 0;
    ring[i] = rawNoise[i] - 0.65 * delayed;
  }

  // Frequency-dependent decay: bass notes ring naturally longer than high treble
  const pitchNorm = Math.min(1, Math.max(0, (frequencyHz - 80) / 700));
  const decay = 0.9965 - pitchNorm * 0.0038;
  const avgWeight = 0.5;

  let writeIdx = 0;
  const attackSamples = Math.floor(sampleRate * 0.002);
  const releaseStart = Math.floor(totalSamples * 0.85);

  for (let n = 0; n < totalSamples; n++) {
    const idx0 = (writeIdx - delayLen + ringSize) % ringSize;
    const idx1 = (idx0 - 1 + ringSize) % ringSize;
    const idx2 = (idx0 + 1) % ringSize;

    // Interpolate fractional delay + Karplus-Strong two-point averaging filter
    const s0 = ring[idx0] * (1 - frac) + ring[idx1] * frac;
    const s1 = ring[idx2] * (1 - frac) + ring[idx0] * frac;
    const nextSample = decay * (avgWeight * s0 + (1 - avgWeight) * s1);

    ring[writeIdx] = nextSample;
    writeIdx = (writeIdx + 1) % ringSize;

    let env = 1;
    if (n < attackSamples) {
      env = n / attackSamples;
    } else if (n > releaseStart) {
      env = Math.max(0, 1 - (n - releaseStart) / (totalSamples - releaseStart));
    }
    output[n] = nextSample * env;
  }

  // Normalize peak safely
  let peak = 0;
  for (let i = 0; i < totalSamples; i++) {
    const abs = Math.abs(output[i]);
    if (abs > peak) peak = abs;
  }
  if (peak > 0.001) {
    const scale = 0.7 / peak;
    for (let i = 0; i < totalSamples; i++) {
      output[i] *= scale;
    }
  }

  return buffer;
}

function triggerPluck(
  ctx: AudioContext,
  destination: AudioNode,
  midi: number,
  when: number,
  gainLevel: number,
  durationSec = 2.6,
): void {
  const freq = midiToFrequency(midi);
  const buffer = createKarplusStrongBuffer(ctx, freq, durationSec);

  const source = ctx.createBufferSource();
  source.buffer = buffer;

  const gain = ctx.createGain();
  gain.gain.setValueAtTime(gainLevel, when);

  source.connect(gain);
  gain.connect(destination);
  source.start(when);
}

/**
 * Plays all sounded notes of a voicing in strum (~35ms) or arpeggio (~180ms) mode.
 * Adds octave/unison paired strings for Viola Caipira bass & treble courses.
 */
export function playVoicing(
  voicing: VoicingResult,
  instrument: InstrumentConfig,
  mode: PlayMode,
): void {
  if (!voicing.notes.length) return;

  const { ctx, destination } = getAudioEngine();
  const now = ctx.currentTime + 0.02;
  const stepSec = mode === 'strum' ? 0.035 : 0.18;
  const durationSec = mode === 'strum' ? 2.8 : 2.3;
  const perStringGain = Math.min(0.42, 1.35 / Math.max(2, voicing.notes.length));

  voicing.notes.forEach((note, orderIdx) => {
    const startTime = now + orderIdx * stepSec;
    triggerPluck(ctx, destination, note.midi, startTime, perStringGain, durationSec);

    // Handle Viola Caipira paired courses (octave on bass pairs, subtle unison on treble pairs)
    const explicitOffset = note.octave_pair_offset;
    const isViolaBassFallback =
      explicitOffset === null &&
      instrument.kind === 'ViolaCaipira' &&
      note.string_index < 2;

    if (explicitOffset === 12 || isViolaBassFallback) {
      // Octave pair on bass courses: +12 semitones, slight natural pick stagger
      triggerPluck(
        ctx,
        destination,
        note.midi + 12,
        startTime + 0.005,
        perStringGain * 0.68,
        durationSec * 0.9,
      );
    } else if (explicitOffset === 0 && instrument.kind === 'ViolaCaipira') {
      // Unison pair on treble courses: subtle micro-detune shimmer
      triggerPluck(
        ctx,
        destination,
        note.midi + 0.035,
        startTime + 0.004,
        perStringGain * 0.55,
        durationSec * 0.92,
      );
    }
  });
}

/**
 * Plays a single plucked string/note (helpful for auditioning open strings or notes).
 */
export function playSingleMidi(midi: number, octavePairOffset: number | null = null): void {
  const { ctx, destination } = getAudioEngine();
  const now = ctx.currentTime + 0.01;
  triggerPluck(ctx, destination, midi, now, 0.45, 2.0);
  if (octavePairOffset === 12) {
    triggerPluck(ctx, destination, midi + 12, now + 0.005, 0.3, 1.8);
  }
}
