import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Check, Cpu, Download, Loader2, Sparkles } from "lucide-react";

import { BottomMenu } from "../../../components/BottomMenu";
import { cn } from "../../../design-tokens";
import { useDownloadQueue } from "../../../../core/downloads/DownloadQueueContext";
import {
  kokoroDefaultAssetRoot,
  kokoroInstallModel,
  kokoroInstallVoices,
  kokoroSupportedVariants,
  upsertAudioProvider,
  type AudioProvider,
  type KokoroModelVariant,
  type KokoroSupportedVariant,
} from "../../../../core/storage/audioProviders";

export const STARTER_PACK_VOICE_IDS = ["af_heart", "am_adam", "bf_emma", "bm_george"];

const VARIANT_COPY: Record<KokoroModelVariant, { description: string; tag: string }> = {
  int8: {
    description:
      "Light on battery and quick to start. Natural enough for most character voices.",
    tag: "Mobile",
  },
  fp16: {
    description:
      "The sweet spot for desktop. Warmer voices than int8 with no real wait between replies.",
    tag: "Recommended",
  },
  fp32: {
    description:
      "The most lifelike characters, with the richest detail. Slower and heavier on memory.",
    tag: "High-end",
  },
};

export function KokoroSetupMenu({
  provider,
  onClose,
  onStarted,
}: {
  provider: AudioProvider | null;
  onClose: () => void;
  onStarted?: () => void;
}) {
  const { queue } = useDownloadQueue();
  const [variants, setVariants] = useState<KokoroSupportedVariant[]>([]);
  const [picked, setPicked] = useState<KokoroModelVariant | undefined>(undefined);
  const [includeStarter, setIncludeStarter] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!provider) return;
    let active = true;
    void kokoroSupportedVariants()
      .then((list) => {
        if (!active) return;
        setVariants(list);
        setPicked(
          provider.kokoroVariant ?? list.find((v) => v.id === "fp16")?.id ?? list[0]?.id,
        );
      })
      .catch((e) => active && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      active = false;
    };
  }, [provider]);

  const assetRoot = provider?.assetRoot?.trim() ?? "";
  const downloading = useMemo(
    () =>
      queue.some(
        (item) =>
          item.queueKind === "kokoro" &&
          item.assetRoot === assetRoot &&
          (item.status === "downloading" || item.status === "queued"),
      ),
    [queue, assetRoot],
  );

  const handleDownload = async () => {
    if (!provider || !picked || busy) return;
    setBusy(true);
    setError(null);
    try {
      let next = provider;
      let root = next.assetRoot?.trim() ?? "";
      if (!root) {
        root = await kokoroDefaultAssetRoot();
        next = { ...next, assetRoot: root };
      }
      if (next.kokoroVariant !== picked) {
        next = { ...next, kokoroVariant: picked };
      }
      if (next !== provider) {
        await upsertAudioProvider(next);
      }
      await kokoroInstallModel(root, picked);
      if (includeStarter) {
        await kokoroInstallVoices(root, STARTER_PACK_VOICE_IDS);
      }
      onStarted?.();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <BottomMenu
      isOpen={!!provider}
      onClose={() => {
        if (!busy) onClose();
      }}
      title="Set up voice engine"
    >
      <div className="space-y-3">
        <p className="text-[12px] leading-relaxed text-fg/55">
          This voice runs entirely on this device. Pick the quality that fits your hardware, then
          download. You can switch later.
        </p>

        <div className="space-y-1.5">
          {variants.map((v) => {
            const copy = VARIANT_COPY[v.id];
            const isSelected = picked === v.id;
            return (
              <button
                key={v.id}
                onClick={() => setPicked(v.id)}
                disabled={busy || downloading}
                className={cn(
                  "group relative w-full overflow-hidden rounded-xl border px-3.5 py-3 text-left transition disabled:opacity-60",
                  isSelected
                    ? "border-accent/35 bg-accent/8"
                    : "border-fg/10 bg-fg/3 hover:border-fg/20 hover:bg-fg/5",
                )}
              >
                <div className="flex items-center gap-3">
                  <span
                    className={cn(
                      "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg",
                      isSelected ? "bg-accent/15 text-accent" : "bg-fg/8 text-fg/55",
                    )}
                  >
                    <Cpu size={15} />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-[13px] font-medium text-fg">{v.label}</span>
                      {copy && (
                        <span
                          className={cn(
                            "shrink-0 rounded-full border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider",
                            v.id === "fp16"
                              ? "border-accent/30 bg-accent/10 text-accent/85"
                              : "border-fg/15 bg-fg/5 text-fg/55",
                          )}
                        >
                          {copy.tag}
                        </span>
                      )}
                    </div>
                    {copy && (
                      <p className="mt-0.5 truncate text-[11.5px] text-fg/45">{copy.description}</p>
                    )}
                  </div>
                  <span className="shrink-0 text-[12px] font-semibold tabular-nums text-fg/75">
                    {v.sizeMb} MB
                  </span>
                  <span
                    className={cn(
                      "flex h-5 w-5 shrink-0 items-center justify-center rounded-full border transition",
                      isSelected
                        ? "border-accent/50 bg-accent/15 text-accent"
                        : "border-fg/15 text-transparent",
                    )}
                  >
                    <Check size={11} strokeWidth={3} />
                  </span>
                </div>
              </button>
            );
          })}
        </div>

        <button
          onClick={() => setIncludeStarter((prev) => !prev)}
          disabled={busy || downloading}
          className={cn(
            "flex w-full items-center gap-3 rounded-xl border px-3.5 py-3 text-left transition disabled:opacity-60",
            includeStarter
              ? "border-accent/35 bg-accent/8"
              : "border-fg/10 bg-fg/3 hover:border-fg/20 hover:bg-fg/5",
          )}
        >
          <span
            className={cn(
              "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg",
              includeStarter ? "bg-accent/15 text-accent" : "bg-fg/8 text-fg/55",
            )}
          >
            <Sparkles size={15} />
          </span>
          <div className="min-w-0 flex-1">
            <p className="text-[13px] font-medium text-fg">Starter voice pack</p>
            <p className="mt-0.5 text-[11.5px] text-fg/45">
              Heart, Adam, Emma, and George. A balanced set of US and UK voices.
            </p>
          </div>
          <span
            className={cn(
              "flex h-5 w-5 shrink-0 items-center justify-center rounded-md border transition",
              includeStarter
                ? "border-accent/50 bg-accent/15 text-accent"
                : "border-fg/15 text-transparent",
            )}
          >
            <Check size={11} strokeWidth={3} />
          </span>
        </button>

        {error && (
          <div className="flex items-start gap-2 rounded-lg border border-danger/30 bg-danger/10 px-3 py-2">
            <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-danger/85" />
            <p className="flex-1 text-[12px] text-danger/85">{error}</p>
          </div>
        )}

        <button
          onClick={() => void handleDownload()}
          disabled={!picked || busy || downloading}
          className="flex w-full items-center justify-center gap-2 rounded-xl bg-accent px-4 py-2.5 text-[13px] font-semibold text-bg transition hover:brightness-110 disabled:opacity-50"
        >
          {busy || downloading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Download className="h-4 w-4" />
          )}
          {downloading ? "Downloading..." : "Download"}
        </button>
      </div>
    </BottomMenu>
  );
}
