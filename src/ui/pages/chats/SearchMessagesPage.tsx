import { useState, useCallback, useEffect, useRef } from "react";
import { ArrowLeft, Loader2, X, Search } from "lucide-react";
import { useNavigate, useSearchParams, useParams } from "react-router-dom";
import { components, colors, interactive, radius, cn } from "../../design-tokens";
import { storageBridge } from "../../../core/storage/files";
import { Routes, useNavigationManager } from "../../navigation";
import { useI18n } from "../../../core/i18n/context";

interface SearchResult {
    messageId: string;
    content: string;
    createdAt: number;
    role: string;
}

export function SearchMessagesPage() {
    const navigate = useNavigate();
    const { characterId } = useParams<{ characterId: string }>();
    const { backOrReplace } = useNavigationManager();
    const { t } = useI18n();
    const [searchParams] = useSearchParams();
    const sessionId = searchParams.get("sessionId");

    const [query, setQuery] = useState("");
    const [results, setResults] = useState<SearchResult[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const searchTimeoutRef = useRef<number | null>(null);
    const searchRequestRef = useRef(0);
    const activeRequestIdRef = useRef<string | null>(null);

    const cancelActiveSearch = useCallback(() => {
        const requestId = activeRequestIdRef.current;
        activeRequestIdRef.current = null;
        if (requestId) {
            void storageBridge.abortRequest(requestId).catch(() => undefined);
        }
    }, []);

    useEffect(() => () => {
        if (searchTimeoutRef.current) {
            window.clearTimeout(searchTimeoutRef.current);
        }
        searchRequestRef.current += 1;
        cancelActiveSearch();
    }, [cancelActiveSearch]);

    const handleSearch = useCallback(async (searchQuery: string) => {
        const requestId = ++searchRequestRef.current;
        if (!sessionId || !searchQuery.trim()) {
            setResults([]);
            setLoading(false);
            return;
        }

        cancelActiveSearch();
        const backendRequestId = crypto.randomUUID();
        activeRequestIdRef.current = backendRequestId;
        setLoading(true);
        setError(null);
        try {
            const data = await storageBridge.searchMessages(sessionId, searchQuery, backendRequestId);
            if (requestId === searchRequestRef.current) {
                setResults(data);
            }
        } catch (err) {
            console.error("Search failed:", err);
            if (requestId === searchRequestRef.current) {
                setError(t("chats.search.failed"));
            }
        } finally {
            if (activeRequestIdRef.current === backendRequestId) {
                activeRequestIdRef.current = null;
            }
            if (requestId === searchRequestRef.current) {
                setLoading(false);
            }
        }
    }, [cancelActiveSearch, sessionId, t]);

    const onQueryChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const newQuery = e.target.value;
        setQuery(newQuery);
        // A full-history query may finish after the user has already typed more.
        // Ignore it until the debounced request for the latest input completes.
        searchRequestRef.current += 1;
        cancelActiveSearch();

        if (searchTimeoutRef.current) {
            window.clearTimeout(searchTimeoutRef.current);
        }

        searchTimeoutRef.current = window.setTimeout(() => {
            handleSearch(newQuery);
        }, 300);
    };

    const highlightMatch = (text: string, highlight: string) => {
        if (!highlight.trim()) return text;
        const parts = text.split(new RegExp(`(${highlight.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi'));
        return (
            <span>
                {parts.map((part, i) =>
                    part.toLowerCase() === highlight.toLowerCase() ? (
                        <span key={i} className="bg-warning/30 text-warning rounded px-0.5 font-medium">{part}</span>
                    ) : (
                        part
                    )
                )}
            </span>
        );
    };

    return (
        <div className={cn("flex h-screen flex-col", colors.surface.base, colors.text.primary)}>
            {/* Header */}
            <div className={cn(
                "flex items-center gap-3 border-b pl-3 pb-3 pt-[calc(var(--lettuce-safe-area-inset-top)+12px)] shrink-0 z-20",
                "pr-3",
                colors.glass.strong
            )}>
                <button
                    onClick={() => backOrReplace(characterId ? Routes.chatSession(characterId, sessionId) : Routes.chat)}
                    className={cn(
                        "flex shrink-0 items-center justify-center h-8 w-8",
                        radius.full,
                        "border bg-fg/5",
                        colors.border.subtle,
                        colors.text.primary,
                        interactive.hover.brightness,
                        interactive.active.scale,
                        interactive.focus.ring
                    )}
                    aria-label={t("chats.header.back")}
                >
                    <ArrowLeft size={14} />
                </button>
                <div className="flex-1 relative">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-fg/40" size={16} />
                    <input
                        type="text"
                        autoFocus
                        value={query}
                        onChange={onQueryChange}
                        placeholder={t("chats.search.placeholder")}
                        className={cn(
                            "w-full pl-10 pr-10 py-2.5 text-fg placeholder:text-fg/40",
                            components.input.base,
                            radius.lg
                        )}
                    />
                    {query && (
                        <button
                            onClick={() => {
                                setQuery("");
                                setResults([]);
                                setLoading(false);
                                searchRequestRef.current += 1;
                                cancelActiveSearch();
                                if (searchTimeoutRef.current) clearTimeout(searchTimeoutRef.current);
                            }}
                            className="absolute right-3 top-1/2 -translate-y-1/2 text-fg/40 hover:text-fg"
                        >
                            <X size={16} />
                        </button>
                    )}
                </div>
            </div>

            {/* Results */}
            <div className="flex-1 overflow-y-auto px-3 py-4">
                {loading ? (
                    <div className="flex justify-center py-10">
                        <Loader2 className="animate-spin text-fg/30" size={24} />
                    </div>
                ) : error ? (
                    <div className="text-center text-danger py-10">{error}</div>
                ) : results.length === 0 && query.trim() ? (
                    <div className="text-center text-fg/30 py-10">{t("chats.search.noMessagesFound")}</div>
                ) : (
                    <div className="space-y-4">
                        {results.map((result) => (
                            <button
                                key={result.messageId}
                                onClick={() => {
                                    if (!characterId) return;
                                    navigate(Routes.chatSession(characterId, sessionId, { jumpToMessage: result.messageId }));
                                }}
                                className={cn(
                                    "w-full text-left p-4 space-y-2 border border-fg/10 bg-fg/5 hover:bg-fg/10 transition",
                                    radius.lg
                                )}
                            >
                                <div className="flex justify-between items-center text-xs text-fg/40 uppercase font-medium tracking-wider">
                                    <span>{result.role === 'user' ? t("chats.search.you") : t("chats.search.character")}</span>
                                    <span>{new Date(result.createdAt).toLocaleDateString()}</span>
                                </div>
                                <p className="text-sm text-fg/80 line-clamp-3 leading-relaxed">
                                    {highlightMatch(result.content, query)}
                                </p>
                            </button>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
