import { api } from "./api";

/** Voice output + microphone capture. The backend owns synthesis and
 *  transcription; this module only moves bytes and drives playback. */

let current: HTMLAudioElement | null = null;

function teardown() {
  if (current) {
    current.pause();
    current.src = "";
  }
  current = null;
}

export function isSpeaking(): boolean {
  return !!current && !current.paused && !current.ended;
}

export async function speak(text: string, voice?: string): Promise<void> {
  const clean = text.trim();
  if (!clean) return;
  stopSpeaking();
  const result = await api.ttsSpeak(clean, voice);
  const audio = new Audio(`data:${result.mime};base64,${result.audio_base64}`);
  current = audio;
  return new Promise((resolve) => {
    const finish = () => {
      if (current === audio) teardown();
      resolve();
    };
    audio.onended = finish;
    audio.onerror = finish;
    audio.play().catch(finish);
  });
}

export function stopSpeaking() {
  teardown();
}

// ---- microphone capture ----

export interface Recorder {
  /** Resolves with the finished clip. */
  stop: () => Promise<Blob>;
  /** Abandon the clip without resolving. */
  cancel: () => void;
}

export function startRecording(onLevel?: (level: number) => void): Recorder {
  let media: MediaRecorder | null = null;
  let stream: MediaStream | null = null;
  let ctx: AudioContext | null = null;
  let raf = 0;
  let cancelled = false;
  const chunks: BlobPart[] = [];

  const cleanup = () => {
    cancelAnimationFrame(raf);
    ctx?.close().catch(() => {});
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
    ctx = null;
  };

  const ready: Promise<void> = (async () => {
    stream = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: true, noiseSuppression: true },
    });
    const mime = ["audio/webm;codecs=opus", "audio/webm", "audio/ogg"].find((m) =>
      MediaRecorder.isTypeSupported(m),
    );
    media = mime ? new MediaRecorder(stream, { mimeType: mime }) : new MediaRecorder(stream);
    media.ondataavailable = (e) => {
      if (e.data.size > 0) chunks.push(e.data);
    };
    media.start(250);

    if (onLevel) {
      ctx = new AudioContext();
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 512;
      ctx.createMediaStreamSource(stream).connect(analyser);
      const buf = new Uint8Array(analyser.frequencyBinCount);
      const tick = () => {
        analyser.getByteTimeDomainData(buf);
        let peak = 0;
        for (let i = 0; i < buf.length; i++) peak = Math.max(peak, Math.abs(buf[i] - 128) / 128);
        onLevel(peak);
        raf = requestAnimationFrame(tick);
      };
      tick();
    }
  })();

  return {
    stop: async () => {
      try {
        await ready;
      } catch (e) {
        cleanup();
        throw e;
      }
      if (!media || media.state === "inactive") {
        cleanup();
        return new Blob(chunks, { type: "audio/webm" });
      }
      const blob = await new Promise<Blob>((resolve) => {
        media!.onstop = () =>
          resolve(new Blob(chunks, { type: media!.mimeType || "audio/webm" }));
        media!.stop();
      });
      cleanup();
      return blob;
    },
    cancel: () => {
      cancelled = true;
      try {
        if (media && media.state !== "inactive") media.stop();
      } catch {
        /* ignore */
      }
      cleanup();
      void cancelled;
    },
  };
}

export function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const url = String(reader.result);
      resolve(url.slice(url.indexOf(",") + 1));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

/** Record a clip, then hand it to the backend speech-to-text. */
export async function recordAndTranscribe(onLevel?: (level: number) => void): Promise<string> {
  const rec = startRecording(onLevel);
  const blob = await rec.stop();
  if (blob.size === 0) return "";
  const base64 = await blobToBase64(blob);
  return api.sttTranscribe(base64, blob.type || "audio/webm", "clip.webm");
}

export function imageToAttachment(file: File): Promise<{ mime: string; data_base64: string }> {
  return blobToBase64(file).then((data_base64) => ({ mime: file.type || "image/png", data_base64 }));
}
