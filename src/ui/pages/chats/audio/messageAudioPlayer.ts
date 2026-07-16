import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  abortAudioPreview,
  generateTtsForMessage,
  getTtsCached,
  getTtsCacheKey,
  playAudioFromBase64,
  saveTtsToCache,
  streamDoubaoTts,
  type AudioProviderType,
  type TtsPreviewResponse,
} from "../../../../core/storage/audioProviders";

const DOUBAO_STREAM_BUFFER_SECONDS = 0.7;
const S16_MAX = 32768;
const PCM_MIME_TYPE = "audio/pcm";
const WAV_MIME_TYPE = "audio/wav";
const DEFAULT_DOUBAO_SAMPLE_RATE = 44100;
// PCM responses can already peak near full scale. Extra amplification clips
// clone voices and makes them sound muffled, so keep playback at unity gain.
const PCM_PLAYBACK_GAIN = 1.0;

export interface MessageAudioRequest {
  providerId: string;
  providerType: AudioProviderType;
  modelId: string;
  voiceId: string;
  text: string;
  prompt?: string;
  requestId: string;
  sampleRate?: number;
  cached?: TtsPreviewResponse;
  streamDoubao?: boolean;
  onCache?: (response: TtsPreviewResponse) => void;
  onPlaybackStart?: () => void;
}

export interface MessageAudioPlayback {
  stop: () => void;
  done: Promise<void>;
}

type DoubaoStreamPayload =
  | { type: "start"; sampleRate: number; format: string; mimeType: string; nativePcm?: boolean }
  | { type: "chunk"; audioBase64: string }
  | { type: "end" }
  | { type: "error"; message?: string };

function decodeBase64Bytes(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function encodeBase64Bytes(value: Uint8Array): string {
  const chunkSize = 0x8000;
  let binary = "";
  for (let offset = 0; offset < value.length; offset += chunkSize) {
    const chunk = value.subarray(offset, offset + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

function concatByteChunks(chunks: Uint8Array[]): Uint8Array {
  const totalLength = chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0);
  const out = new Uint8Array(totalLength);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

function pcm16ToWav(bytes: Uint8Array, sampleRate: number, channels = 1): Uint8Array {
  const header = new ArrayBuffer(44);
  const view = new DataView(header);
  const writeAscii = (offset: number, value: string) => {
    for (let index = 0; index < value.length; index += 1) {
      view.setUint8(offset + index, value.charCodeAt(index));
    }
  };
  const bitsPerSample = 16;
  const blockAlign = channels * (bitsPerSample / 8);
  const byteRate = sampleRate * blockAlign;
  writeAscii(0, "RIFF");
  view.setUint32(4, 36 + bytes.byteLength, true);
  writeAscii(8, "WAVE");
  writeAscii(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, channels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  writeAscii(36, "data");
  view.setUint32(40, bytes.byteLength, true);
  const wav = new Uint8Array(44 + bytes.byteLength);
  wav.set(new Uint8Array(header), 0);
  wav.set(bytes, 44);
  return wav;
}

function resolvePcmSampleRate(request: MessageAudioRequest): number {
  if (Number.isFinite(request.sampleRate) && (request.sampleRate ?? 0) > 0) {
    return Math.round(request.sampleRate as number);
  }
  if (request.prompt) {
    try {
      const value = JSON.parse(request.prompt) as { sampleRate?: unknown };
      if (typeof value.sampleRate === "number" && Number.isFinite(value.sampleRate)) {
        return Math.max(8000, Math.min(48000, Math.round(value.sampleRate)));
      }
    } catch {
      // Invalid prompt JSON is reported by the backend when generating new audio.
    }
  }
  return DEFAULT_DOUBAO_SAMPLE_RATE;
}

function pcm16ToFloat32(bytes: Uint8Array): Float32Array<ArrayBuffer> {
  const sampleCount = Math.floor(bytes.byteLength / 2);
  const out = new Float32Array(sampleCount);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  for (let i = 0; i < sampleCount; i += 1) {
    out[i] = view.getInt16(i * 2, true) / S16_MAX;
  }
  return out;
}

class PcmStreamQueue {
  private audioContext: AudioContext | null = null;
  private sampleRate = DEFAULT_DOUBAO_SAMPLE_RATE;
  private queue: Float32Array<ArrayBuffer>[] = [];
  private queuedSamples = 0;
  private started = false;
  private ended = false;
  private stopped = false;
  private nextStartTime = 0;
  private pendingSources = 0;
  private resolveDone: (() => void) | null = null;
  private readonly donePromise = new Promise<void>((resolve) => {
    this.resolveDone = resolve;
  });

  constructor(private readonly onPlaybackStart?: () => void) {}

  get done() {
    return this.donePromise;
  }

  configure(sampleRate: number) {
    if (Number.isFinite(sampleRate) && sampleRate > 0) {
      this.sampleRate = sampleRate;
    }
  }

  push(bytes: Uint8Array) {
    if (this.stopped || bytes.byteLength < 2) return;
    const samples = pcm16ToFloat32(bytes);
    if (samples.length === 0) return;
    this.queue.push(samples);
    this.queuedSamples += samples.length;
    if (!this.started && this.queuedSamples / this.sampleRate >= DOUBAO_STREAM_BUFFER_SECONDS) {
      void this.start();
    } else if (this.started) {
      this.scheduleAvailable();
    }
  }

  finish() {
    this.ended = true;
    if (!this.started) {
      void this.start();
    } else {
      this.checkDone();
    }
  }

  async start() {
    if (this.started || this.stopped) return;
    this.started = true;
    this.audioContext = new AudioContext();
    await this.audioContext.resume();
    this.nextStartTime = this.audioContext.currentTime + 0.04;
    this.onPlaybackStart?.();
    this.scheduleAvailable();
    this.checkDone();
  }

  stop() {
    this.stopped = true;
    this.queue = [];
    this.queuedSamples = 0;
    if (this.audioContext) {
      void this.audioContext.close().catch(() => undefined);
      this.audioContext = null;
    }
    this.resolveDone?.();
    this.resolveDone = null;
  }

  private scheduleAvailable() {
    const ctx = this.audioContext;
    if (!ctx || this.stopped) return;
    while (this.queue.length > 0) {
      const samples = this.queue.shift();
      if (!samples) break;
      this.queuedSamples -= samples.length;
      const buffer = ctx.createBuffer(1, samples.length, this.sampleRate);
      buffer.copyToChannel(samples, 0);
      const source = ctx.createBufferSource();
      const gain = ctx.createGain();
      gain.gain.value = PCM_PLAYBACK_GAIN;
      source.buffer = buffer;
      source.connect(gain);
      gain.connect(ctx.destination);
      const startAt = Math.max(this.nextStartTime, ctx.currentTime + 0.02);
      this.nextStartTime = startAt + buffer.duration;
      this.pendingSources += 1;
      source.onended = () => {
        source.disconnect();
        gain.disconnect();
        this.pendingSources = Math.max(0, this.pendingSources - 1);
        this.checkDone();
      };
      source.start(startAt);
    }
  }

  private checkDone() {
    if (!this.ended || this.queue.length > 0 || this.pendingSources > 0 || this.resolveDone == null) {
      return;
    }
    this.resolveDone();
    this.resolveDone = null;
  }
}

async function startBufferedPlayback(request: MessageAudioRequest): Promise<MessageAudioPlayback> {
  const response =
    request.cached ??
    (await generateTtsForMessage(
      request.providerId,
      request.modelId,
      request.voiceId,
      request.text,
      request.prompt,
      request.requestId,
    ));
  if (!request.cached) {
    request.onCache?.(response);
  }
  request.onPlaybackStart?.();
  const audio = playAudioFromBase64(response.audioBase64, response.format);
  let resolveDone: (() => void) | null = null;
  const done = new Promise<void>((resolve) => {
    resolveDone = resolve;
  });
  audio.onended = () => {
    resolveDone?.();
    resolveDone = null;
  };
  audio.onerror = () => {
    resolveDone?.();
    resolveDone = null;
  };
  return {
    stop: () => {
      void abortAudioPreview(request.requestId).catch(() => undefined);
      audio.pause();
      audio.currentTime = 0;
      audio.onended = null;
      audio.onerror = null;
      resolveDone?.();
      resolveDone = null;
    },
    done,
  };
}

async function startCachedPcmWavPlayback(
  request: MessageAudioRequest,
  audioBase64: string,
  sampleRate: number,
): Promise<MessageAudioPlayback> {
  const wavBytes = pcm16ToWav(decodeBase64Bytes(audioBase64), sampleRate);
  const wavBuffer = new ArrayBuffer(wavBytes.byteLength);
  new Uint8Array(wavBuffer).set(wavBytes);
  const objectUrl = URL.createObjectURL(new Blob([wavBuffer], { type: "audio/wav" }));
  const audio = new Audio(objectUrl);
  audio.preload = "auto";
  let resolveDone: (() => void) | null = null;
  const done = new Promise<void>((resolve) => {
    resolveDone = resolve;
  });
  const cleanup = () => {
    URL.revokeObjectURL(objectUrl);
    audio.onended = null;
    audio.onerror = null;
  };
  audio.onended = () => {
    cleanup();
    resolveDone?.();
    resolveDone = null;
  };
  audio.onerror = () => {
    cleanup();
    resolveDone?.();
    resolveDone = null;
  };
  request.onPlaybackStart?.();
  await audio.play();
  return {
    stop: () => {
      audio.pause();
      audio.currentTime = 0;
      cleanup();
      resolveDone?.();
      resolveDone = null;
    },
    done,
  };
}

async function startDoubaoStreamPlayback(
  request: MessageAudioRequest,
): Promise<MessageAudioPlayback> {
  const cacheKey = await getTtsCacheKey(
    request.providerId,
    request.modelId,
    request.voiceId,
    request.text,
    request.prompt,
  );
  const cached = request.cached ?? (await getTtsCached(cacheKey));
  if (cached) {
    const legacyPcmSampleRate =
      cached.format === PCM_MIME_TYPE ? resolvePcmSampleRate(request) : undefined;
    console.debug("[Doubao TTS] cache hit", {
      voiceId: request.voiceId,
      format: cached.format,
      legacyPcmSampleRate,
    });
    request.onCache?.(cached);
    if (cached.format === PCM_MIME_TYPE) {
      return startCachedPcmWavPlayback(
        request,
        cached.audioBase64,
        legacyPcmSampleRate ?? DEFAULT_DOUBAO_SAMPLE_RATE,
      );
    }
    return startBufferedPlayback({ ...request, cached });
  }

  const queue = new PcmStreamQueue(request.onPlaybackStart);
  let nativePcm = false;
  let resolvePlaybackDone: (() => void) | null = null;
  const playbackDone = new Promise<void>((resolve) => {
    resolvePlaybackDone = resolve;
  });
  void queue.done.then(() => resolvePlaybackDone?.());
  const eventName = `tts-stream://${request.requestId}`;
  let unlisten: UnlistenFn | null = null;
  let streamError: Error | null = null;
  let stopped = false;
  const audioChunks: Uint8Array[] = [];
  let streamSampleRate = resolvePcmSampleRate(request);
  let streamedBytes = 0;
  let loggedFirstChunk = false;

  console.debug("[Doubao TTS] stream request", {
    providerId: request.providerId,
    modelId: request.modelId,
    voiceId: request.voiceId,
    sampleRate: request.sampleRate ?? "server-default",
    textLength: request.text.length,
    prompt: request.prompt ?? null,
  });

  unlisten = await listen<DoubaoStreamPayload | string>(eventName, (event) => {
    const rawPayload = event.payload;
    const payload =
      typeof rawPayload === "string"
        ? (JSON.parse(rawPayload) as DoubaoStreamPayload)
        : rawPayload;
    if (payload.type === "start") {
      console.debug("[Doubao TTS] stream start", payload);
      nativePcm = payload.nativePcm === true;
      streamSampleRate = payload.sampleRate;
      console.debug("[Doubao TTS] playback route", {
        nativePcm,
        route: nativePcm ? "android-audiotrack" : "webaudio",
      });
      if (nativePcm) {
        request.onPlaybackStart?.();
      } else {
        queue.configure(payload.sampleRate);
      }
      return;
    }
    if (payload.type === "chunk") {
      const bytes = decodeBase64Bytes(payload.audioBase64);
      audioChunks.push(bytes);
      streamedBytes += bytes.byteLength;
      if (!loggedFirstChunk) {
        console.debug("[Doubao TTS] first PCM chunk", {
          bytes: bytes.byteLength,
          totalBytes: streamedBytes,
          firstBytes: Array.from(bytes.slice(0, 16))
            .map((value) => value.toString(16).padStart(2, "0"))
            .join(" "),
        });
        loggedFirstChunk = true;
      }
      if (!nativePcm) queue.push(bytes);
      return;
    }
    if (payload.type === "end") {
      console.debug("[Doubao TTS] stream end", { totalBytes: streamedBytes });
      if (nativePcm) {
        resolvePlaybackDone?.();
        resolvePlaybackDone = null;
      } else {
        queue.finish();
      }
      return;
    }
    if (payload.type === "error") {
      streamError = new Error(payload.message || "Doubao TTS stream failed");
      queue.stop();
      resolvePlaybackDone?.();
      resolvePlaybackDone = null;
    }
  });

  const command = streamDoubaoTts(
    request.providerId,
    request.modelId,
    request.voiceId,
    request.text,
    request.requestId,
    request.prompt,
  ).finally(() => {
    unlisten?.();
    unlisten = null;
  });

  const commandCompletion = command.catch((error) => {
    streamError = error instanceof Error ? error : new Error(String(error));
    queue.stop();
  });
  const cacheSave = command.then(async () => {
    if (stopped || audioChunks.length === 0) return;
    const pcm = concatByteChunks(audioChunks);
    const wav = pcm16ToWav(pcm, streamSampleRate);
    const audioBase64 = encodeBase64Bytes(wav);
    console.debug("[Doubao TTS] WebAudio stream cached as WAV", {
      pcmBytes: pcm.byteLength,
      wavBytes: wav.byteLength,
      sampleRate: streamSampleRate,
    });
    request.onCache?.({ audioBase64, format: WAV_MIME_TYPE });
    await saveTtsToCache(cacheKey, audioBase64, WAV_MIME_TYPE).catch((error) => {
      console.warn("Failed to save streamed Doubao TTS audio to cache:", error);
    });
  });

  return {
    stop: () => {
      stopped = true;
      void abortAudioPreview(request.requestId).catch(() => undefined);
      unlisten?.();
      unlisten = null;
      queue.stop();
      resolvePlaybackDone?.();
      resolvePlaybackDone = null;
    },
    done: Promise.all([playbackDone, commandCompletion, cacheSave.catch(() => undefined)]).then(() => {
      if (streamError) throw streamError;
    }),
  };
}

export async function startMessageAudioPlayback(
  request: MessageAudioRequest,
): Promise<MessageAudioPlayback> {
  if (request.providerType === "doubao_tts" && request.streamDoubao !== false) {
    try {
      return await startDoubaoStreamPlayback(request);
    } catch (error) {
      console.warn("Doubao streaming TTS failed before playback; falling back to buffered TTS.", error);
    }
  }
  return startBufferedPlayback(request);
}
