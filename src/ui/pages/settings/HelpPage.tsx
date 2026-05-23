import { useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { ArrowLeft, ChevronDown, ChevronRight, HelpCircle, ExternalLink } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { cn, typography } from "../../design-tokens";
import { DISCORD_SERVER_LINK } from "../../../core/utils/links";

interface FaqItem {
  question: string;
  answer: React.ReactNode;
}

function Crumb() {
  return (
    <ChevronRight
      size={12}
      strokeWidth={2}
      className="mx-0.5 inline-block shrink-0 -translate-y-px text-fg/35"
      aria-hidden
    />
  );
}

interface FaqSection {
  title: string;
  items: FaqItem[];
}

const SECTIONS: FaqSection[] = [
  {
    title: "Getting started",
    items: [
      {
        question: "What is LettuceAI?",
        answer: (
          <p>
            LettuceAI is a chat app for AI characters. Unlike apps that route your messages
            through a single hosted service, LettuceAI lets you bring your own API key from
            providers like Mistral, Cerebras, Google AI Studio, OpenAI, Anthropic, or run
            models locally on your device. Your chats stay on your device.
          </p>
        ),
      },
      {
        question: "What is an API key and why do I need one? (BYOK explained)",
        answer: (
          <div className="space-y-3">
            <p>
              BYOK means "Bring Your Own Key." Think of an API key as a personal pass
              card that lets an app talk to a service on your behalf. When you sign up
              with a provider like Mistral, Cerebras, Google AI Studio, or OpenAI, they
              give you a long string of letters and numbers. That's your key.
            </p>
            <p>
              The AI models that write the replies don't live on your phone. They run on
              big servers owned by those providers. Every message you send needs to
              travel to one of those servers, get a reply, and come back. The key tells
              the provider "this request is from me, charge it to my account."
            </p>
            <p>
              Most other AI chat apps hide this from you by running their own server in
              the middle: they hold one big key, you pay them a subscription, and your
              conversations pass through their machines. LettuceAI doesn't do that. You
              put your own key in the app, and messages go straight from your device to
              the provider you picked. Nothing in between.
            </p>
            <p>
              That's why you have to add a key once before you can chat, and also why
              the app is free, your chats stay private, and you only pay for what you
              actually use.
            </p>
          </div>
        ),
      },
      {
        question: "What's the cheapest or easiest way to start?",
        answer: (
          <div className="space-y-2">
            <p>
              <span className="font-medium text-fg">Mistral</span> has a generous free
              tier and good open-weight models. Sign up at console.mistral.ai, create a
              key, and paste it into Settings <Crumb /> Providers.
            </p>
            <p>
              <span className="font-medium text-fg">Cerebras</span> is the fastest option
              out there with a free tier on Llama and Qwen models. Sign up at
              cloud.cerebras.ai.
            </p>
            <p>
              <span className="font-medium text-fg">Google AI Studio</span> also offers a
              generous free tier for Gemini models.
            </p>
            <p>
              If you don't want to use any cloud service, you can run models locally via
              the built-in llama.cpp option on desktop (Windows, macOS, or Linux). Local
              models aren't supported on mobile.
            </p>
          </div>
        ),
      },
      {
        question: "Is this an alternative to other AI chat apps?",
        answer: (
          <p>
            Yes. If you've used an AI chat app where everything just worked but you
            couldn't pick your model, couldn't control your data, or felt limited by a
            subscription, LettuceAI is built for that. You bring your own API key, pick
            the model you want, and your chats stay on your device. The trade-off is a
            short one-time setup; the upside is total control over cost, quality, and
            privacy.
          </p>
        ),
      },
      {
        question: "How much does it cost?",
        answer: (
          <p>
            LettuceAI itself is free. You pay the model provider directly, usually a
            fraction of a cent per message. Free tiers exist on Mistral, Cerebras, and
            Google AI Studio. Heavy use of frontier models (Claude Opus 4.7, GPT-5.1,
            Gemini 3 Pro) can add up, so check the provider's pricing page before picking
            a model.
          </p>
        ),
      },
    ],
  },
  {
    title: "Privacy & safety",
    items: [
      {
        question: "Is my API key safe?",
        answer: (
          <p>
            Your API key is stored locally on your device and only ever sent to the
            provider you configured it for. LettuceAI does not have a server that sees
            your key or your messages.
          </p>
        ),
      },
      {
        question: "Where are my chats stored? Can anyone read them?",
        answer: (
          <p>
            Chats are stored locally in the app's database on your device. They are not
            uploaded anywhere. When you send a message, only that conversation's text is
            sent to your chosen provider for a reply, then their response comes back and
            is saved locally.
          </p>
        ),
      },
      {
        question: "Can my chats get deleted on their own?",
        answer: (
          <p>
            No. Chats live in a local database on your device and only you can delete
            them. They aren't synced to any LettuceAI server, so we can't wipe them, and
            no automatic cleanup runs in the background. The only ways your chats go
            away are: you delete them yourself, you uninstall the app, you reset the
            app, or your device storage is wiped.
          </p>
        ),
      },
      {
        question: "Can someone edit my data remotely?",
        answer: (
          <p>
            No. There's no remote admin panel, no LettuceAI account, and no server with
            a copy of your characters, chats, or settings. Nobody at LettuceAI can push
            a change to your data, lock you out, or reach into the app to modify
            anything. Your data only changes when you change it on your own device.
          </p>
        ),
      },
      {
        question: "What happens to my data if LettuceAI shuts down?",
        answer: (
          <p>
            Your characters and chats keep working as long as the app is on your device.
            Cloud models would stop replying if you have no key or your provider goes
            away, but everything you've created stays readable locally and exportable
            via Settings <Crumb /> Backup & Restore.
          </p>
        ),
      },
      {
        question: "Will providers train on my conversations?",
        answer: (
          <p>
            That depends on the provider's policy, not on LettuceAI. Most paid API tiers
            (OpenAI, Anthropic, Mistral) do not train on API traffic by default. Free
            tiers sometimes do. If this matters to you, read the privacy page of whichever
            provider you use.
          </p>
        ),
      },
    ],
  },
  {
    title: "Models & providers",
    items: [
      {
        question: "What's a 'model'? Which should I pick?",
        answer: (
          <div className="space-y-2">
            <p>
              A model is the AI that writes the replies. Different models have different
              personalities, costs, and quality.
            </p>
            <p>
              Good starting picks:{" "}
              <span className="font-medium text-fg">Gemma 4</span> (free, lightweight),{" "}
              <span className="font-medium text-fg">DeepSeek V4</span> (cheap and very
              capable), or <span className="font-medium text-fg">GLM 4.7 / GLM 5</span>{" "}
              (strong roleplay quality). Cerebras runs many of these at extreme speed on
              its free tier. You can switch any time from a chat's settings.
            </p>
          </div>
        ),
      },
      {
        question: "Are there free providers?",
        answer: (
          <p>
            Yes. <span className="font-medium text-fg">Mistral</span>,{" "}
            <span className="font-medium text-fg">Cerebras</span>, and{" "}
            <span className="font-medium text-fg">Google AI Studio</span> all offer free
            tiers that are enough for casual everyday chatting. You still need to sign
            up and create an API key, but you don't add a payment method to start. Each
            provider has its own rate limits (how many messages per minute or per day),
            so if you hit a wall, just switch to another free provider or upgrade.
          </p>
        ),
      },
      {
        question: "What's the difference between free and paid providers?",
        answer: (
          <div className="space-y-3">
            <p>
              <span className="font-medium text-fg">Free tiers</span> usually give you
              access to smaller or older models, slower speeds, lower rate limits, and
              sometimes the provider may use your messages to train their next model.
              Great for trying things out or light daily use.
            </p>
            <p>
              <span className="font-medium text-fg">Paid tiers</span> unlock the latest
              and largest models, higher rate limits, faster responses, and stronger
              privacy guarantees (most don't train on paid API traffic). You pay per
              message, typically a fraction of a cent, with no monthly minimum.
            </p>
            <p>
              You can mix and match: keep a free key for everyday chat and a paid key
              for when you want the best quality.
            </p>
          </div>
        ),
      },
      {
        question: "What is a token?",
        answer: (
          <div className="space-y-3">
            <p>
              A token is roughly a small piece of a word, about 4 characters or
              three-quarters of an English word on average. "Hello there!" is around 3
              tokens. A full sentence might be 15–20.
            </p>
            <p>
              Providers charge per token, not per message. Each request counts both the
              tokens you send (your message plus the character's setup and chat history)
              and the tokens the model writes back. That's why long chats with lots of
              context cost more than short fresh ones.
            </p>
            <p>
              You don't need to count tokens yourself. The app handles it. It just
              helps to know that "1 million tokens" on a pricing page is a lot of
              chatting.
            </p>
          </div>
        ),
      },
      {
        question: "I see 'no default model configured'",
        answer: (
          <div className="space-y-3">
            <p>
              This means either you haven't added any model to LettuceAI yet, or you
              added models but the character you're chatting with doesn't have one
              picked.
            </p>
            <p>
              Open Settings <Crumb /> Models and add a model from one of your configured
              providers. Then either set it as your global default on the same page, or
              open the character's settings and pick a model just for that character.
            </p>
            <p>
              Each character can use its own model, or fall back to your global default
              if none is set.
            </p>
          </div>
        ),
      },
      {
        question: "What's the difference between cloud and local models?",
        answer: (
          <p>
            Cloud models (Mistral, Cerebras, OpenAI, Google) run on someone else's
            hardware, need an internet connection, and cost money per message. Local
            models run directly on your desktop computer through the built-in llama.cpp
            option, are private and offline, but need a capable PC and take more storage.
            Local models aren't supported on mobile.
          </p>
        ),
      },
    ],
  },
  {
    title: "Characters & chats",
    items: [
      {
        question: "What's a character? Can I make my own?",
        answer: (
          <p>
            A character is an AI persona with a name, image, and description that shapes
            how it talks. Tap the + button on the Chats screen to create one, or import
            character cards from the discover tab or other communities.
          </p>
        ),
      },
      {
        question: "What's a persona?",
        answer: (
          <p>
            A persona is <span className="italic">your</span> side of the conversation:
            your name, pronouns, and details the character should know about you. Set one
            up under Settings <Crumb /> Personas and it'll be used across your chats.
          </p>
        ),
      },
      {
        question: "Are the characters I create private?",
        answer: (
          <p>
            Yes. Characters you create stay on your device. LettuceAI has no upload
            button and no server to upload them to. They're only ever shared if you
            explicitly export a character card file and send it somewhere yourself.
          </p>
        ),
      },
      {
        question: "How do I back up or move to another device?",
        answer: (
          <p>
            Settings <Crumb /> Backup & Restore lets you export everything to a file. To move to
            a new phone or computer, install LettuceAI there and use the "Sync from
            another device" option on the welcome screen, or restore from a backup file.
          </p>
        ),
      },
    ],
  },
];

const DOCS_URL = "https://www.lettuceai.app/docs";
const DISCORD_URL = DISCORD_SERVER_LINK;

function openExternal(url: string) {
  void (async () => {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch {
      window.open(url, "_blank");
    }
  })();
}

function FaqRow({ item, isOpen, onToggle }: { item: FaqItem; isOpen: boolean; onToggle: () => void }) {
  return (
    <div className="px-4">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-start gap-4 py-4 text-left"
      >
        <span
          className={cn(
            typography.body.size,
            "flex-1 font-normal leading-snug",
            isOpen ? "text-fg" : "text-fg/85",
          )}
        >
          {item.question}
        </span>
        <motion.span
          animate={{ rotate: isOpen ? 180 : 0 }}
          transition={{ duration: 0.18 }}
          className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center text-fg/35"
        >
          <ChevronDown size={16} strokeWidth={1.75} />
        </motion.span>
      </button>
      <AnimatePresence initial={false}>
        {isOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.18, ease: "easeOut" }}
            className="overflow-hidden"
          >
            <div
              className={cn(
                "pb-5 pr-9 text-[0.9rem] leading-[1.65] text-fg/65 [&_p]:m-0 [&_a]:text-accent [&_a]:underline [&_a]:underline-offset-2",
              )}
            >
              {item.answer}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export function HelpPage() {
  const [openKey, setOpenKey] = useState<string | null>("0-0");
  const location = useLocation();
  const navigate = useNavigate();
  const fromWelcome = Boolean((location.state as { fromWelcome?: boolean } | null)?.fromWelcome);

  return (
    <div className="mx-auto w-full max-w-3xl space-y-8 px-4 py-6 lg:px-8">
      {fromWelcome && (
        <button
          type="button"
          onClick={() => navigate("/welcome")}
          className={cn(
            "inline-flex items-center gap-2 rounded-md border border-fg/10 bg-fg/[0.03] px-3 py-1.5 text-fg/70 transition hover:bg-fg/[0.06] hover:text-fg",
            typography.caption.size,
          )}
        >
          <ArrowLeft size={14} strokeWidth={2} />
          Back to setup
        </button>
      )}
      <header className="flex items-start gap-3.5">
        <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl border border-fg/10 bg-fg/[0.04] text-fg/70">
          <HelpCircle size={20} strokeWidth={1.75} />
        </div>
        <div className="min-w-0">
          <h1 className={cn(typography.h1.size, "font-semibold tracking-tight text-fg")}>
            Help & FAQ
          </h1>
          <p className={cn("mt-1.5 max-w-prose text-[0.9rem] leading-[1.55] text-fg/55")}>
            New to LettuceAI? Start here. The basics, BYOK explained, and answers to the
            most common questions.
          </p>
        </div>
      </header>

      {SECTIONS.map((section, sectionIndex) => (
        <section key={section.title} className="space-y-2">
          <h2
            className={cn(
              "px-1 text-[0.7rem] font-semibold uppercase tracking-[0.14em] text-fg/35",
            )}
          >
            {section.title}
          </h2>
          <div className="overflow-hidden rounded-2xl border border-fg/10 bg-fg/[0.02] divide-y divide-fg/[0.05]">
            {section.items.map((item, itemIndex) => {
              const key = `${sectionIndex}-${itemIndex}`;
              return (
                <FaqRow
                  key={key}
                  item={item}
                  isOpen={openKey === key}
                  onToggle={() => setOpenKey(openKey === key ? null : key)}
                />
              );
            })}
          </div>
        </section>
      ))}

      <section className="space-y-2">
        <h2
          className={cn(
            typography.overline.size,
            typography.overline.weight,
            typography.overline.tracking,
            typography.overline.transform,
            "px-1 text-fg/40",
          )}
        >
          Still stuck?
        </h2>
        <div className="overflow-hidden rounded-xl border border-fg/10 bg-fg/[0.025] divide-y divide-fg/[0.06]">
          <button
            type="button"
            onClick={() => openExternal(DOCS_URL)}
            className="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-fg/[0.04]"
          >
            <span className="min-w-0 flex-1">
              <span className={cn("block", typography.body.size, "font-medium text-fg")}>
                Full documentation
              </span>
              <span className={cn("mt-0.5 block", typography.caption.size, "text-fg/45")}>
                Detailed guides for every feature.
              </span>
            </span>
            <ExternalLink className="h-4 w-4 shrink-0 text-fg/25" />
          </button>
          <button
            type="button"
            onClick={() => openExternal(DISCORD_URL)}
            className="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-fg/[0.04]"
          >
            <span className="min-w-0 flex-1">
              <span className={cn("block", typography.body.size, "font-medium text-fg")}>
                Join the Discord
              </span>
              <span className={cn("mt-0.5 block", typography.caption.size, "text-fg/45")}>
                Ask the community or report a bug.
              </span>
            </span>
            <ExternalLink className="h-4 w-4 shrink-0 text-fg/25" />
          </button>
        </div>
      </section>
    </div>
  );
}
