import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  AlertTriangle,
  ArrowRight,
  ShieldCheck,
  Sparkles,
  Upload,
  Smartphone,
  FileArchive,
  Lock,
  Loader2,
  Eye,
  EyeOff,
  CheckCircle,
} from "lucide-react";
import { motion } from "framer-motion";

import { setOnboardingCompleted, setOnboardingSkipped } from "../../../core/storage/appState";
import { storageBridge } from "../../../core/storage/files";
import logoSvg from "../../../assets/logo.svg";
import { typography, radius, spacing, interactive, shadows, cn } from "../../design-tokens";
import { useI18n } from "../../../core/i18n/context";
import { LocaleSelector } from "../../components/LocaleSelector";
import { DynamicMemoryEmbeddingPrompt } from "./components/DynamicMemoryEmbeddingPrompt";

interface WelcomePageProps {
  onContinue?: () => void;
  onGoToSync?: () => void;
}

export function WelcomePage({ onContinue, onGoToSync }: WelcomePageProps = {}) {
  const { locale, setLocale, t } = useI18n();
  const navigate = useNavigate();
  const [showSkipWarning, setShowSkipWarning] = useState(false);
  const [showRestoreBackup, setShowRestoreBackup] = useState(false);

  // Ctrl+Shift+L to open logs page during onboarding
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === "L") {
        e.preventDefault();
        navigate("/settings/logs");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [navigate]);

  // Load brand fonts (Fraunces for display, Noto Sans for body — same as lettuceai.app)
  useEffect(() => {
    const id = "lai-brand-fonts";
    if (document.getElementById(id)) return;
    const preconnect = document.createElement("link");
    preconnect.rel = "preconnect";
    preconnect.href = "https://fonts.gstatic.com";
    preconnect.crossOrigin = "anonymous";
    document.head.appendChild(preconnect);
    const link = document.createElement("link");
    link.id = id;
    link.rel = "stylesheet";
    link.href =
      "https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,300..900&family=Noto+Sans:wght@400;500;600;700&display=swap";
    document.head.appendChild(link);
  }, []);

  const handleAddProvider = () => {
    if (onContinue) onContinue();
    else navigate("/onboarding/provider");
  };

  const handleConfirmSkip = async () => {
    await setOnboardingCompleted(true);
    await setOnboardingSkipped(true);
    // Small delay to ensure state is persisted before navigation
    await new Promise((resolve) => setTimeout(resolve, 100));
    navigate("/");
  };

  const handleRestoreComplete = async () => {
    await setOnboardingCompleted(true);
    navigate("/chat");
  };

  return (
    <div className="lai-welcome relative flex flex-col text-white">
      <style>{LAI_WELCOME_STYLES}</style>

      {/* Top bar */}
      <motion.div
        className="relative z-10 flex items-center justify-between px-6 pt-6 lg:px-12 lg:pt-8"
        initial={{ opacity: 0, y: -6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
      >
        <div className="flex items-center gap-2.5">
          <img src={logoSvg} alt="" className="h-7 w-7" />
          <span className="lai-wordmark-sm">LettuceAI</span>
        </div>

        {/* Mobile-only language selector — bare text + icon */}
        <div className="lai-locale-bare lg:hidden">
          <LocaleSelector
            value={locale}
            onChange={setLocale}
            label=""
            description=""
            title={t("components.localeSelector.title")}
            labelClassName="hidden"
            descriptionClassName="hidden"
            triggerClassName="lai-locale-trigger-bare"
            menuClassName=""
          />
        </div>
      </motion.div>

      {/* Main content — centered hero + action band */}
      <div className="relative z-10 flex flex-1 items-center justify-center px-6 py-6 lg:px-16 lg:py-12">
        <div className="w-full max-w-2xl flex flex-col items-center text-center">
          {/* Logo */}
          <motion.div
            className="relative overflow-visible"
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, ease: "easeOut" }}
          >
            <div className="lai-logo-halo absolute" aria-hidden="true" />
            <img
              src={logoSvg}
              alt={t("onboarding.welcome.appName")}
              className="relative h-14 w-14 lg:h-20 lg:w-20"
            />
          </motion.div>

          {/* Headline */}
          <motion.h1
            className="lai-headline mt-5 lg:mt-9"
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.05 }}
          >
            {t("onboarding.welcome.headline.lead")}{" "}
            <span className="lai-headline-accent">
              {t("onboarding.welcome.headline.accent")}
            </span>
          </motion.h1>

          {/* Tagline */}
          <motion.p
            className="lai-tagline mt-3 lg:mt-5 max-w-lg"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5, delay: 0.15 }}
          >
            {t("onboarding.welcome.tagline")}
          </motion.p>

          {/* Primary CTA */}
          <motion.button
            className="lai-cta-solo group mt-6 lg:mt-10 inline-flex items-center gap-2.5"
            onClick={handleAddProvider}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.22 }}
          >
            <span className="lai-cta-label">{t("onboarding.welcome.getStarted")}</span>
            <span className="lai-band-arrow">
              <ArrowRight size={16} strokeWidth={2.25} />
            </span>
          </motion.button>

          {/* Secondary actions */}
          <motion.div
            className="lai-secondary mt-3 lg:mt-5 flex flex-wrap items-center justify-center gap-x-5 gap-y-2"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5, delay: 0.28 }}
          >
            <button
              onClick={() => setShowRestoreBackup(true)}
              className="lai-link group inline-flex items-center gap-1.5 py-1"
            >
              <Upload size={13} strokeWidth={2} />
              <span className="lai-link-text">
                {t("onboarding.welcome.restoreFromBackup")}
              </span>
            </button>

            <span className="lai-link-sep" />

            <button
              onClick={() => (onGoToSync ? onGoToSync() : navigate("/onboarding/sync"))}
              className="lai-link group inline-flex items-center gap-1.5 py-1"
            >
              <Smartphone size={13} strokeWidth={2} />
              <span className="lai-link-text">{t("onboarding.welcome.syncFromDevice")}</span>
            </button>

            <span className="lai-link-sep" />

            <button
              onClick={() => setShowSkipWarning(true)}
              className="lai-link group inline-flex items-center py-1"
            >
              <span className="lai-link-text">
                {t("onboarding.welcome.skipForNow")}
              </span>
            </button>
          </motion.div>

          {/* Trust line */}
          <motion.div
            className="lai-trust mt-4 lg:mt-7 flex flex-wrap items-center justify-center gap-x-3 gap-y-1.5"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5, delay: 0.3 }}
          >
            <span className="inline-flex items-center gap-1.5">
              <ShieldCheck size={12} className="lai-accent-text" strokeWidth={2.25} />
              {t("onboarding.welcome.features.onDevice")}
            </span>
            <span className="lai-trust-sep">·</span>
            <span className="inline-flex items-center gap-1.5">
              <Sparkles size={12} className="lai-accent-text" strokeWidth={2.25} />
              {t("onboarding.welcome.features.characterReady")}
            </span>
            <span className="lai-trust-sep">·</span>
            <span>{t("onboarding.welcome.setupTime")}</span>
          </motion.div>
        </div>
      </div>

      {/* Bottom footer strip — desktop only */}
      <motion.div
        className="lai-footer relative z-10 hidden lg:block px-6 py-4 lg:px-12 lg:py-5"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.3 }}
      >
        <div className="flex flex-col-reverse gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="lai-locale-bare flex items-center w-full lg:w-auto">
            <LocaleSelector
              value={locale}
              onChange={setLocale}
              label=""
              description=""
              title={t("components.localeSelector.title")}
              labelClassName="hidden"
              descriptionClassName="hidden"
              triggerClassName="lai-locale-trigger-bare"
              menuClassName=""
            />
          </div>
          <p className="lai-footnote text-center lg:text-right">
            {t("onboarding.welcome.languageSelector.description")}
          </p>
        </div>
      </motion.div>

      {showSkipWarning && (
        <SkipWarning
          onClose={() => setShowSkipWarning(false)}
          onConfirm={handleConfirmSkip}
          onAddProvider={handleAddProvider}
        />
      )}

      {showRestoreBackup && (
        <RestoreBackupModal
          onClose={() => setShowRestoreBackup(false)}
          onComplete={handleRestoreComplete}
        />
      )}
    </div>
  );
}

const LAI_WELCOME_STYLES = `
  .lai-welcome {
    --lai-accent: #00d294;
    --lai-accent-soft: rgba(0, 210, 148, 0.85);
    --lai-accent-dim: rgba(0, 210, 148, 0.55);
    --lai-bg-0: #050505;
    --lai-ink: #f4f4f5;
    --lai-ink-2: rgba(244, 244, 245, 0.62);
    --lai-ink-3: rgba(244, 244, 245, 0.40);
    --lai-ink-4: rgba(244, 244, 245, 0.22);
    --lai-line: rgba(244, 244, 245, 0.08);
    background: transparent;
    font-family: "Noto Sans", ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
    overflow: hidden;
    height: 100vh;
    height: 100dvh;
    box-sizing: border-box;
    padding-top: env(safe-area-inset-top);
    padding-bottom: env(safe-area-inset-bottom);
  }

  .lai-bg-image {
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
  }
  .lai-bg-overlay {
    background:
      linear-gradient(180deg, rgba(5,5,5,0.45) 0%, rgba(5,5,5,0.50) 60%, rgba(5,5,5,0.65) 100%);
  }

  .lai-wordmark-sm {
    font-family: "Noto Sans", ui-sans-serif, system-ui, sans-serif;
    font-weight: 700;
    font-size: 18px;
    letter-spacing: -0.02em;
    color: var(--lai-ink);
  }

  .lai-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px 5px 10px;
    border-radius: 999px;
    border: 1px solid rgba(0, 210, 148, 0.35);
    background: rgba(8, 24, 18, 0.78);
    backdrop-filter: blur(14px);
  }
  .lai-pill-dot {
    width: 6px; height: 6px; border-radius: 999px;
    background: var(--lai-accent);
    box-shadow: 0 0 8px rgba(0, 210, 148, 0.55);
    animation: lai-pulse 2.4s ease-in-out infinite;
  }
  .lai-pill-label {
    font-size: 12.5px; font-weight: 500; letter-spacing: 0.01em;
    color: rgba(0, 210, 148, 0.95);
  }
  @keyframes lai-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.55; }
  }

  .lai-logo-halo {
    inset: -30px;
    border-radius: 9999px;
    background: radial-gradient(closest-side, rgba(0, 210, 148, 0.20), transparent 70%);
    filter: blur(22px);
  }

  .lai-headline {
    font-family: "Noto Sans", ui-sans-serif, system-ui, sans-serif;
    font-weight: 700;
    font-size: clamp(37px, 5vw, 57px);
    letter-spacing: -0.03em;
    line-height: 1.08;
    color: var(--lai-ink);
    max-width: 18ch;
  }
  .lai-headline-accent {
    font-family: "Fraunces", "Iowan Old Style", Georgia, serif;
    font-style: italic;
    font-weight: 500;
    color: var(--lai-accent);
    letter-spacing: -0.02em;
    font-variation-settings: "opsz" 96;
  }

  .lai-tagline {
    font-family: "Noto Sans", sans-serif;
    font-size: 16.5px;
    line-height: 1.55;
    color: var(--lai-ink-2);
    font-weight: 400;
  }

  .lai-trust {
    font-size: 12.5px;
    color: var(--lai-ink-3);
    font-weight: 500;
  }
  .lai-trust-sep { color: var(--lai-ink-4); }
  .lai-accent-text { color: var(--lai-accent); }

  .lai-cta-label {
    font-family: "Noto Sans", sans-serif;
    font-weight: 600;
    font-size: 16px;
    letter-spacing: -0.005em;
    color: #ffffff;
  }

  .lai-cta-solo {
    padding: 12px 18px 12px 22px;
    border-radius: 12px;
    background: linear-gradient(180deg, rgba(0, 210, 148, 0.28), rgba(0, 210, 148, 0.16));
    border: 1px solid rgba(0, 210, 148, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255,255,255,0.10),
      0 10px 28px -8px rgba(0, 210, 148, 0.40),
      0 18px 44px -12px rgba(0,0,0,0.6);
    transition: background 200ms ease, border-color 200ms ease, transform 200ms ease, box-shadow 240ms ease;
    cursor: pointer;
    backdrop-filter: blur(10px);
  }
  .lai-cta-solo:hover {
    background: linear-gradient(180deg, rgba(0, 210, 148, 0.36), rgba(0, 210, 148, 0.22));
    border-color: rgba(0, 210, 148, 0.65);
    box-shadow:
      inset 0 1px 0 rgba(255,255,255,0.12),
      0 14px 36px -8px rgba(0, 210, 148, 0.50),
      0 18px 44px -12px rgba(0,0,0,0.6);
  }
  .lai-cta-solo:active { transform: scale(0.985); }

  .lai-secondary {
    padding: 6px 14px;
    border-radius: 999px;
    background: rgba(10, 10, 10, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(14px);
  }
  .lai-band-arrow {
    display: inline-flex;
    width: 22px; height: 22px;
    align-items: center; justify-content: center;
    border-radius: 999px;
    background: rgba(255,255,255,0.10);
    color: #ffffff;
    transition: transform 200ms ease, background 200ms ease;
  }
  .lai-cta-solo:hover .lai-band-arrow {
    background: rgba(255,255,255,0.18);
    transform: translateX(2px);
  }

  @media (max-width: 640px) {
    .lai-headline {
      font-size: clamp(29px, 8vw, 37px);
      max-width: 14ch;
      line-height: 1.05;
    }
    .lai-tagline { font-size: 14px; line-height: 1.45; }
    .lai-cta-solo {
      width: 100%;
      justify-content: space-between;
      padding: 12px 16px;
    }
    .lai-secondary {
      flex-direction: column;
      align-items: stretch;
      width: 100%;
      max-width: 320px;
      gap: 0;
      padding: 2px 0;
      border-radius: 12px;
      background: rgba(10, 10, 10, 0.55);
      border: 1px solid rgba(255, 255, 255, 0.08);
    }
    .lai-secondary .lai-link {
      width: 100%;
      justify-content: center;
      padding: 9px 14px;
    }
    .lai-secondary .lai-link-sep {
      width: 100%;
      height: 1px;
      background: rgba(255, 255, 255, 0.06);
    }
    .lai-trust {
      font-size: 11.5px;
      gap: 4px 8px;
    }
    .lai-footer { padding: 8px 16px 10px; }
    .lai-footnote { font-size: 11px; }
  }

  .lai-link {
    background: transparent;
    color: var(--lai-ink-2);
    font-size: 13.5px;
    transition: color 150ms ease;
    cursor: pointer;
  }
  .lai-link:hover { color: var(--lai-ink); }
  .lai-link-text {
    text-decoration: underline;
    text-decoration-color: rgba(244, 244, 245, 0.18);
    text-underline-offset: 4px;
    transition: text-decoration-color 150ms ease;
  }
  .lai-link:hover .lai-link-text { text-decoration-color: var(--lai-accent-dim); }
  .lai-link-sep {
    width: 1px; height: 12px;
    background: var(--lai-line);
  }

  .lai-footer {
    border-top: 1px solid var(--lai-line);
  }
  .lai-footnote {
    font-size: 12px;
    color: var(--lai-ink-3);
    font-weight: 400;
  }

  .lai-locale-trigger {
    background: rgba(10, 10, 10, 0.78) !important;
    border: 1px solid rgba(255, 255, 255, 0.10) !important;
    color: var(--lai-ink) !important;
    backdrop-filter: blur(14px) !important;
  }
  .lai-locale-trigger:hover {
    background: rgba(20, 20, 20, 0.85) !important;
    border-color: rgba(255, 255, 255, 0.18) !important;
  }
  .lai-locale-trigger-bare {
    background: transparent !important;
    border: none !important;
    padding: 4px 0 !important;
    font-size: 14px !important;
    color: var(--lai-ink-2) !important;
    backdrop-filter: none !important;
    box-shadow: none !important;
  }
  .lai-locale-trigger-bare:hover {
    background: transparent !important;
    color: var(--lai-ink) !important;
  }
  .lai-locale-bare .tabular-nums { display: none !important; }
  .lai-locale-menu {
    background: #0a0a0a !important;
    border-color: var(--lai-line) !important;
  }
`;

function SkipWarning({
  onClose,
  onConfirm,
  onAddProvider,
}: {
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
  onAddProvider: () => void;
}) {
  const { t } = useI18n();
  const [isExiting, setIsExiting] = useState(false);

  const handleClose = () => {
    setIsExiting(true);
    setTimeout(onClose, 200);
  };

  const handleConfirm = () => {
    setIsExiting(true);
    setTimeout(() => void onConfirm(), 200);
  };

  const handleAddProvider = () => {
    setIsExiting(true);
    setTimeout(onAddProvider, 200);
  };

  return (
    <motion.div
      className="fixed inset-0 z-50 flex items-end justify-center bg-black/60 backdrop-blur-sm"
      initial={{ opacity: 0 }}
      animate={{ opacity: isExiting ? 0 : 1 }}
      transition={{ duration: 0.2 }}
      onClick={handleClose}
    >
      <motion.div
        className={cn(
          "w-full max-w-lg border border-white/10 bg-[#0b0b0d] p-6",
          "rounded-t-3xl sm:rounded-3xl sm:mb-8",
          shadows.xl,
        )}
        initial={{ y: "100%", opacity: 0 }}
        animate={{
          y: isExiting ? "100%" : 0,
          opacity: isExiting ? 0 : 1,
        }}
        transition={{
          type: "spring",
          damping: 30,
          stiffness: 350,
          duration: 0.2,
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <h3 className={cn(typography.h2.size, typography.h2.weight, "text-white")}>
            {t("onboarding.welcome.skipWarning.title")}
          </h3>
        </div>

        {/* Warning content */}
        <div
          className={cn(
            "flex items-start gap-3 border border-amber-400/20 bg-amber-400/5 p-4 mb-6",
            radius.md,
          )}
        >
          <div
            className={cn(
              "flex h-10 w-10 shrink-0 items-center justify-center bg-amber-400/20 text-amber-300",
              radius.md,
            )}
          >
            <AlertTriangle size={20} strokeWidth={2.5} />
          </div>
          <div className={spacing.tight}>
            <h4 className={cn(typography.body.size, typography.h3.weight, "text-white")}>
              {t("onboarding.welcome.skipWarning.warningTitle")}
            </h4>
            <p
              className={cn(
                typography.bodySmall.size,
                typography.bodySmall.lineHeight,
                "text-white/60",
              )}
            >
              {t("onboarding.welcome.skipWarning.warningMessage")}
            </p>
          </div>
        </div>

        {/* Actions */}
        <div className={cn("flex flex-col", spacing.field)}>
          <button
            className={cn(
              "inline-flex items-center justify-center gap-2 px-6 py-3",
              radius.md,
              "border border-emerald-400/40 bg-emerald-400/20 text-emerald-100",
              typography.body.size,
              typography.h3.weight,
              interactive.transition.fast,
              interactive.active.scale,
              "hover:border-emerald-400/60 hover:bg-emerald-400/30",
            )}
            onClick={handleAddProvider}
          >
            <span>{t("onboarding.welcome.skipWarning.addProvider")}</span>
            <ArrowRight size={16} strokeWidth={2.5} />
          </button>
          <button
            className={cn(
              "px-6 py-3",
              radius.md,
              "border border-white/10 bg-white/5 text-white/60",
              typography.body.size,
              interactive.transition.fast,
              interactive.active.scale,
              "hover:border-white/20 hover:bg-white/10 hover:text-white",
            )}
            onClick={handleConfirm}
          >
            {t("onboarding.welcome.skipWarning.skipAnyway")}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}

interface BackupInfo {
  version: number;
  createdAt: number;
  appVersion: string;
  encrypted: boolean;
  totalFiles: number;
  path: string;
  filename: string;
}

function RestoreBackupModal({
  onClose,
  onComplete: _onComplete,
}: {
  onClose: () => void;
  onComplete: () => void | Promise<void>;
}) {
  const { t } = useI18n();
  const [isExiting, setIsExiting] = useState(false);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedBackup, setSelectedBackup] = useState<BackupInfo | null>(null);
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showEmbeddingPrompt, setShowEmbeddingPrompt] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    loadBackups();
  }, []);

  const loadBackups = async () => {
    try {
      setLoading(true);
      const list = await storageBridge.backupList();
      setBackups(list);
    } catch (e) {
      console.error("Failed to load backups:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleBrowseForBackup = async () => {
    try {
      setError(null);
      setLoading(true);
      const result = await storageBridge.backupPickFile();
      if (!result) return;

      const { path, filename } = result;

      const info = await storageBridge.backupGetInfo(path);

      const backupInfo: BackupInfo = {
        ...info,
        path,
        filename,
      };

      setSelectedBackup(backupInfo);
      setPassword("");
    } catch (e) {
      console.error("Failed to browse for backup:", e);
      setError(e instanceof Error ? e.message : t("onboarding.welcome.restoreBackup.errors.failedToOpenFile"));
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    if (restoring) return;
    setIsExiting(true);
    setTimeout(onClose, 200);
  };

  const handleRestore = async () => {
    if (!selectedBackup) return;

    if (selectedBackup.encrypted && password.length < 1) {
      setError(t("onboarding.welcome.restoreBackup.errors.passwordRequired"));
      return;
    }

    try {
      setError(null);

      if (selectedBackup.encrypted) {
        const valid = await storageBridge.backupVerifyPassword(selectedBackup.path, password);
        if (!valid) {
          setError(t("onboarding.welcome.restoreBackup.errors.incorrectPassword"));
          return;
        }
      }

      setRestoring(true);

      const hasDynamicMemory = await storageBridge.backupCheckDynamicMemory(
        selectedBackup.path,
        selectedBackup.encrypted ? password : undefined,
      );

      await storageBridge.backupImport(
        selectedBackup.path,
        selectedBackup.encrypted ? password : undefined,
      );

      await setOnboardingCompleted(true);

      if (hasDynamicMemory) {
        const hasEmbeddingModel = await storageBridge.checkEmbeddingModel();
        if (!hasEmbeddingModel) {
          setRestoring(false);
          setShowEmbeddingPrompt(true);
          return;
        }
      }

      setIsExiting(true);
      setTimeout(() => {
        navigate("/");
      }, 200);
    } catch (e) {
      console.log(e);
      setError(e instanceof Error ? e.message : t("onboarding.welcome.restoreBackup.errors.failedToRestore"));
      setRestoring(false);
    }
  };

  const handleDownloadModel = () => {
    setShowEmbeddingPrompt(false);
    handleClose();
    navigate("/settings/embedding-download?returnTo=/");
  };

  const handleDisableAndContinue = async () => {
    setShowEmbeddingPrompt(false);
    setRestoring(true);

    try {
      await storageBridge.backupDisableDynamicMemory();

      navigate("/");
    } catch (error) {
      console.error("Failed to disable dynamic memory:", error);
      setError(error instanceof Error ? error.message : t("onboarding.welcome.restoreBackup.errors.failedToUpdateSettings"));
    } finally {
      setRestoring(false);
    }
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp);
    return `${date.toLocaleDateString(undefined, { month: "short", day: "numeric" })}, ${date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}`;
  };

  return (
    <motion.div
      className="fixed inset-0 z-50 flex items-end justify-center bg-black/60 backdrop-blur-sm"
      initial={{ opacity: 0 }}
      animate={{ opacity: isExiting ? 0 : 1 }}
      transition={{ duration: 0.2 }}
      onClick={handleClose}
    >
      <motion.div
        className={cn(
          "w-full max-w-lg border border-white/10 bg-[#0b0b0d] p-6",
          "rounded-t-3xl sm:rounded-3xl sm:mb-8",
          "max-h-[80vh] overflow-hidden flex flex-col",
          shadows.xl,
        )}
        initial={{ y: "100%", opacity: 0 }}
        animate={{
          y: isExiting ? "100%" : 0,
          opacity: isExiting ? 0 : 1,
        }}
        transition={{
          type: "spring",
          damping: 30,
          stiffness: 350,
          duration: 0.2,
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <h3 className={cn(typography.h2.size, typography.h2.weight, "text-white")}>
            {t("onboarding.welcome.restoreBackup.title")}
          </h3>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto space-y-4">
          {!selectedBackup ? (
            <>
              <div className="flex items-center justify-between">
                <p className={cn(typography.bodySmall.size, "text-white/50")}>
                  {t("onboarding.welcome.restoreBackup.selectMessage")}
                </p>
                <button
                  onClick={handleBrowseForBackup}
                  className="text-[13px] font-medium text-blue-400 hover:text-blue-300"
                >
                  {t("onboarding.welcome.restoreBackup.browse")}
                </button>
              </div>

              {/* Error display for list view */}
              {error && (
                <div
                  className={cn(
                    "flex items-start gap-2 border border-red-400/30 bg-red-400/10 px-3 py-2 text-[15px] text-red-200 mb-4",
                    radius.md,
                  )}
                >
                  <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
                  <span>{error}</span>
                </div>
              )}

              {loading ? (
                <div className="flex flex-col items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-white/30" />
                  <p className="mt-2 text-[15px] text-white/40">{t("onboarding.welcome.restoreBackup.processing")}</p>
                  <p className="text-[13px] text-white/20 mt-1">{t("onboarding.welcome.restoreBackup.processingNote")}</p>
                  <button
                    onClick={() => setLoading(false)}
                    className="mt-6 text-[13px] text-red-400/60 hover:text-red-300 transition-colors"
                  >
                    {t("onboarding.welcome.restoreBackup.cancel")}
                  </button>
                </div>
              ) : backups.length === 0 ? (
                <div className={cn("border border-white/10 bg-white/5 p-6 text-center", radius.md)}>
                  <FileArchive className="mx-auto h-8 w-8 text-white/20" />
                  <p className="mt-3 text-[15px] text-white/40">{t("onboarding.welcome.restoreBackup.noBackups")}</p>
                  <p className="mt-1 text-[13px] text-white/30">{t("onboarding.welcome.restoreBackup.noBackupsHint")}</p>
                  <button
                    onClick={handleBrowseForBackup}
                    className={cn(
                      "mt-4 inline-flex items-center gap-2 px-4 py-2 rounded-lg",
                      "border border-blue-400/30 bg-blue-400/10",
                      "text-[15px] text-blue-300 font-medium",
                      "hover:bg-blue-400/20 active:scale-[0.98]",
                      interactive.transition.default,
                    )}
                  >
                    <Upload className="h-4 w-4" />
                    {t("onboarding.welcome.restoreBackup.browseLettuce")}
                  </button>
                </div>
              ) : (
                <div className="space-y-2">
                  {backups.map((backup) => (
                    <button
                      key={backup.path}
                      onClick={() => {
                        setSelectedBackup(backup);
                        setPassword("");
                        setError(null);
                      }}
                      className={cn(
                        "w-full rounded-xl border border-white/10 bg-white/5 p-3 text-left",
                        interactive.transition.default,
                        "hover:border-white/20 hover:bg-white/8",
                        "active:scale-[0.99]",
                      )}
                    >
                      <div className="flex items-center gap-3">
                        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/10 bg-white/10">
                          <FileArchive className="h-4 w-4 text-white/60" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <p className="truncate text-[15px] font-medium text-white">
                              {backup.filename}
                            </p>
                            {backup.encrypted && (
                              <Lock className="h-3 w-3 shrink-0 text-amber-400/70" />
                            )}
                          </div>
                          <p className="mt-0.5 text-[12px] text-white/40">
                            {formatDate(backup.createdAt)} · v{backup.appVersion}
                          </p>
                        </div>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </>
          ) : (
            <>
              {/* Selected backup info */}
              <div className={cn("border border-white/10 bg-white/5 p-3", radius.md)}>
                <div className="flex items-center gap-3">
                  <FileArchive className="h-6 w-6 text-white/40" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[15px] font-medium text-white">
                      {selectedBackup.filename}
                    </p>
                    <p className="text-[13px] text-white/40">
                      {formatDate(selectedBackup.createdAt)} · v{selectedBackup.appVersion}
                    </p>
                  </div>
                </div>
              </div>

              {/* Info notice */}
              <div
                className={cn(
                  "flex items-start gap-2 border border-blue-400/30 bg-blue-400/10 px-3 py-2 text-[13px] text-blue-200",
                  radius.md,
                )}
              >
                <CheckCircle className="h-4 w-4 shrink-0 mt-0.5" />
                <span>{t("onboarding.welcome.restoreBackup.infoMessage")}</span>
              </div>

              {error && (
                <div
                  className={cn(
                    "flex items-center gap-2 border border-red-400/30 bg-red-400/10 px-3 py-2 text-[15px] text-red-200",
                    radius.md,
                  )}
                >
                  <AlertTriangle className="h-4 w-4 shrink-0" />
                  {error}
                </div>
              )}

              {selectedBackup.encrypted && (
                <div>
                  <label className="mb-1.5 block text-[13px] font-medium text-white/50">
                    {t("onboarding.welcome.restoreBackup.passwordLabel")}
                  </label>
                  <div className="relative">
                    <input
                      type={showPassword ? "text" : "password"}
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      placeholder={t("onboarding.welcome.restoreBackup.passwordPlaceholder")}
                      className={cn(
                        "w-full border border-white/10 bg-white/5 px-4 py-3 pr-12 text-white placeholder-white/30",
                        radius.lg,
                        "focus:border-white/20 focus:outline-none",
                      )}
                    />
                    <button
                      type="button"
                      onClick={() => setShowPassword(!showPassword)}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-white/40 hover:text-white/60"
                    >
                      {showPassword ? <EyeOff className="h-5 w-5" /> : <Eye className="h-5 w-5" />}
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>

        {/* Actions */}
        <div className={cn("flex flex-col pt-4", spacing.field)}>
          {selectedBackup ? (
            <>
              <button
                onClick={() => void handleRestore()}
                disabled={restoring || (selectedBackup.encrypted && password.length < 1)}
                className={cn(
                  "flex items-center justify-center gap-2 px-6 py-3",
                  radius.md,
                  "border border-emerald-400/40 bg-emerald-400/20 text-emerald-100",
                  typography.body.size,
                  typography.h3.weight,
                  interactive.transition.fast,
                  interactive.active.scale,
                  "hover:border-emerald-400/60 hover:bg-emerald-400/30",
                  "disabled:opacity-50",
                )}
              >
                {restoring ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t("onboarding.welcome.restoreBackup.restoring")}
                  </>
                ) : (
                  <>
                    <Upload size={16} />
                    {t("onboarding.welcome.restoreBackup.restoreButton")}
                  </>
                )}
              </button>
              <button
                onClick={() => setSelectedBackup(null)}
                disabled={restoring}
                className={cn(
                  "px-6 py-3",
                  radius.md,
                  "border border-white/10 bg-white/5 text-white/60",
                  typography.body.size,
                  interactive.transition.fast,
                  interactive.active.scale,
                  "hover:border-white/20 hover:bg-white/10 hover:text-white",
                  "disabled:opacity-50",
                )}
              >
                {t("onboarding.welcome.restoreBackup.back")}
              </button>
            </>
          ) : (
            <button
              onClick={handleClose}
              className={cn(
                "px-6 py-3",
                radius.md,
                "border border-white/10 bg-white/5 text-white/60",
                typography.body.size,
                interactive.transition.fast,
                interactive.active.scale,
                "hover:border-white/20 hover:bg-white/10 hover:text-white",
              )}
            >
              {t("onboarding.welcome.restoreBackup.cancel")}
            </button>
          )}
        </div>
      </motion.div>

      {showEmbeddingPrompt && (
        <DynamicMemoryEmbeddingPrompt
          onDownload={handleDownloadModel}
          onContinueWithout={handleDisableAndContinue}
        />
      )}
    </motion.div>
  );
}
