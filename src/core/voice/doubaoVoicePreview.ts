export interface DoubaoVoiceAudioMetadata {
  previewUrl: string;
  format: "wav" | "unknown";
  sampleRate: number;
  bitRate: number;
  channels: number;
  bitsPerSample: number;
  analyzedAt: number;
}

const CACHE_PREFIX = "doubao-voice-preview-metadata-v1:";
let activePreviewAudio: HTMLAudioElement | null = null;

function cacheKey(providerId: string, voiceId: string) {
  return `${CACHE_PREFIX}${providerId}:${voiceId}`;
}

function readAscii(bytes: Uint8Array, offset: number, length: number) {
  return String.fromCharCode(...bytes.subarray(offset, offset + length));
}

function parseWavMetadata(bytes: Uint8Array, previewUrl: string): DoubaoVoiceAudioMetadata | null {
  if (bytes.length < 12 || readAscii(bytes, 0, 4) !== "RIFF" || readAscii(bytes, 8, 4) !== "WAVE") {
    return null;
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let offset = 12;
  while (offset + 8 <= bytes.length) {
    const chunkId = readAscii(bytes, offset, 4);
    const chunkSize = view.getUint32(offset + 4, true);
    const dataOffset = offset + 8;
    if (chunkId === "fmt " && dataOffset + 16 <= bytes.length) {
      const channels = view.getUint16(dataOffset + 2, true);
      const sampleRate = view.getUint32(dataOffset + 4, true);
      const bitRate = view.getUint32(dataOffset + 8, true) * 8;
      const bitsPerSample = view.getUint16(dataOffset + 14, true);
      if (sampleRate > 0 && channels > 0 && bitsPerSample > 0) {
        return {
          previewUrl,
          format: "wav",
          sampleRate,
          bitRate,
          channels,
          bitsPerSample,
          analyzedAt: Date.now(),
        };
      }
    }
    offset = dataOffset + chunkSize + (chunkSize % 2);
  }
  return null;
}

export function getCachedDoubaoVoicePreviewMetadata(
  providerId: string,
  voiceId: string,
): DoubaoVoiceAudioMetadata | null {
  if (typeof window === "undefined") return null;
  try {
    const value = window.localStorage.getItem(cacheKey(providerId, voiceId));
    if (!value) return null;
    const parsed = JSON.parse(value) as DoubaoVoiceAudioMetadata;
    return parsed.sampleRate > 0 ? parsed : null;
  } catch {
    return null;
  }
}

export async function analyzeDoubaoVoicePreview(
  providerId: string,
  voiceId: string,
  previewUrl: string,
): Promise<DoubaoVoiceAudioMetadata | null> {
  const cached = getCachedDoubaoVoicePreviewMetadata(providerId, voiceId);
  if (cached) return cached;

  const response = await fetch(previewUrl, { headers: { Range: "bytes=0-4095" } });
  if (!response.ok) throw new Error(`DemoAudio request failed: ${response.status}`);
  const metadata = parseWavMetadata(new Uint8Array(await response.arrayBuffer()), previewUrl);
  if (!metadata) return null;

  try {
    window.localStorage.setItem(cacheKey(providerId, voiceId), JSON.stringify(metadata));
  } catch {
    // Metadata remains usable for the current request when storage is unavailable.
  }
  return metadata;
}

export function stopDoubaoVoicePreview() {
  activePreviewAudio?.pause();
  activePreviewAudio = null;
}

export async function playDoubaoVoicePreview(
  providerId: string,
  voiceId: string,
  previewUrl: string,
) {
  const metadata = await analyzeDoubaoVoicePreview(providerId, voiceId, previewUrl);
  stopDoubaoVoicePreview();
  const audio = new Audio(previewUrl);
  activePreviewAudio = audio;
  audio.addEventListener("ended", () => {
    if (activePreviewAudio === audio) activePreviewAudio = null;
  }, { once: true });
  await audio.play();
  return { audio, metadata };
}
