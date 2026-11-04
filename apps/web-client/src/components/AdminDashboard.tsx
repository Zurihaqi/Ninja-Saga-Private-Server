import { useCallback, useEffect, useState } from "react";

const RESOURCES = [
    { key: "users", label: "Users" },
    { key: "characters", label: "Characters" },
    { key: "character_items", label: "Character Items" },
    { key: "character_skills", label: "Character Skills" },
    { key: "character_pets", label: "Character Pets" },
    { key: "items", label: "Items" },
    { key: "skills", label: "Skills" },
    { key: "pets", label: "Pets" },
    { key: "missions", label: "Missions" },
    { key: "enemies", label: "Enemies" },
    { key: "npcs", label: "NPCs" },
    { key: "talents", label: "Talents" },
    { key: "senjutsus", label: "Senjutsus" },
    { key: "xp_levels", label: "XP Levels" },
    { key: "game_events", label: "Game Events" },
    { key: "event_analytics", label: "Event Analytics" },
    { key: "giveaways", label: "Giveaways" },
    { key: "mails", label: "Character Mails" },
    { key: "mysterious_markets", label: "Mysterious Markets" },
    { key: "admin_commands", label: "Admin Commands" },
    { key: "clan_seasons", label: "Clan Seasons" },
    { key: "crew_seasons", label: "Crew Seasons" },
    { key: "shadow_war_seasons", label: "Shadow War Seasons" },
    { key: "clans", label: "Clans" },
    { key: "crews", label: "Crews" },
    { key: "pvp_battles", label: "PvP Battles" },
];

const COMMAND_TYPES = [
    "give_all_available",
    "give_all_skills",
    "give_all_pets",
    "give_all_weapons",
    "give_all_hairstyles",
    "give_all_backitems",
    "give_all_accessoryitems",
    "give_all_setitems",
    "give_all_materialitems",
    "give_all_essentialitems",
    "give_all_eventitems",
    "give_all_packageitems",
    "give_all_dealitems",
    "give_all_clanitems",
    "give_all_crewitems",
    "give_all_shadowwaritems",
    "give_all_spendingitems",
    "give_all_leaderboarditems",
    "give_all_itemitems",
    "give_all_category",
    "add_gold",
    "add_tokens",
    "set_level",
    "set_rank",
];

const CONFIG_KEYS = [
    "game_settings",
    "announcements",
    "clan_settings",
    "attendance_rewards",
    "roulette_rewards",
    "scratch_Config",
    "eudemon",
    "exotic_packages",
    "hunting_house_settings",
    "hunting_house_zone_rewards",
    "prestige_settings",
    "pvp_settings",
    "special_deals",
    "welcome_login_rewards",
    "academy_settings",
    "pet_combination_config",
    "chunin_package",
    "friendship_shop",
];

type TabKey = "overview" | "resources" | "commands" | "configs" | "chat";

export default function AdminDashboard() {
    const [token, setToken] = useState<string>(
        () => localStorage.getItem("ns_admin_token") || "",
    );
    const [username, setUsername] = useState("");
    const [password, setPassword] = useState("");
    const [secretInput, setSecretInput] = useState("");
    const [authError, setAuthError] = useState("");

    const [activeTab, setActiveTab] = useState<TabKey>("overview");
    const [statusBanner, setStatusBanner] = useState<string>("");

    // Overview state
    const [overview, setOverview] = useState<{
        counts?: Record<string, number>;
        seasons?: Record<string, Record<string, unknown> | null>;
    }>({});

    // Resource state
    const [selectedResource, setSelectedResource] = useState("characters");
    const [searchQuery, setSearchQuery] = useState("");
    const [charFilter, setCharFilter] = useState("");
    const [rows, setRows] = useState<Record<string, unknown>[]>([]);
    const [editorId, setEditorId] = useState<number | null>(null);
    const [editorJson, setEditorJson] = useState<string>("{\n  \n}");

    // Command runner state
    const [cmdCharId, setCmdCharId] = useState("1");
    const [cmdType, setCmdType] = useState("add_gold");
    const [cmdParamsJson, setCmdParamsJson] = useState('{"amount": 50000}');
    const [cmdResult, setCmdResult] = useState("");

    // Configs state
    const [selectedConfigKey, setSelectedConfigKey] = useState("game_settings");
    const [configValueJson, setConfigValueJson] = useState("{}");

    // Chat state
    const [chatChannel, setChatChannel] = useState("global");
    const [chatMessages, setChatMessages] = useState<Record<string, unknown>[]>(
        [],
    );
    const [announceText, setAnnounceText] = useState("");

    const apiFetch = useCallback(
        async (path: string, init?: RequestInit) => {
            const headers: Record<string, string> = {
                "Content-Type": "application/json",
                ...(init?.headers as Record<string, string>),
            };
            if (token) {
                headers["Authorization"] = `Bearer ${token}`;
                headers["X-Admin-Secret"] = token;
            }
            const res = await fetch(path, { ...init, headers });
            const data = await res.json().catch(() => ({}));
            if (res.status === 401 || res.status === 403) {
                setAuthError(
                    (data as { error?: string }).error || "Unauthorized",
                );
            }
            return { ok: res.ok, status: res.status, data };
        },
        [token],
    );

    const loadOverview = useCallback(async () => {
        const res = await apiFetch("/api/admin/overview");
        if (res.ok) {
            setOverview(res.data);
        }
    }, [apiFetch]);

    const loadResource = useCallback(async () => {
        const params = new URLSearchParams();
        if (searchQuery.trim()) params.set("search", searchQuery.trim());
        if (charFilter.trim()) params.set("character_id", charFilter.trim());
        const res = await apiFetch(
            `/api/admin/resources/${selectedResource}?${params.toString()}`,
        );
        if (res.ok && Array.isArray(res.data.data)) {
            setRows(res.data.data);
        }
    }, [apiFetch, selectedResource, searchQuery, charFilter]);

    const loadConfig = useCallback(
        async (key: string) => {
            const res = await apiFetch(`/api/admin/configs/${key}`);
            if (res.ok) {
                setConfigValueJson(
                    JSON.stringify(res.data.value ?? {}, null, 2),
                );
            }
        },
        [apiFetch],
    );

    const loadChat = useCallback(async () => {
        const res = await apiFetch(
            `/api/admin/chat/messages?channel=${encodeURIComponent(chatChannel)}`,
        );
        if (res.ok && Array.isArray(res.data.data)) {
            setChatMessages(res.data.data);
        }
    }, [apiFetch, chatChannel]);

    useEffect(() => {
        if (!token) return;
        if (activeTab === "overview") void loadOverview();
        if (activeTab === "resources") void loadResource();
        if (activeTab === "configs") void loadConfig(selectedConfigKey);
        if (activeTab === "chat") void loadChat();
    }, [
        token,
        activeTab,
        loadOverview,
        loadResource,
        loadConfig,
        selectedConfigKey,
        loadChat,
    ]);

    const handleLogin = async (e: React.FormEvent) => {
        e.preventDefault();
        setAuthError("");
        const payload = secretInput.trim()
            ? { secret: secretInput.trim() }
            : { username, password };
        const res = await fetch("/api/admin/login", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(payload),
        });
        const data = await res.json().catch(() => ({}));
        if (res.ok && data.token) {
            localStorage.setItem("ns_admin_token", data.token);
            setToken(data.token);
        } else {
            setAuthError(data.error || "Login failed");
        }
    };

    const handleSaveResource = async () => {
        try {
            const parsed = JSON.parse(editorJson);
            const path =
                editorId === null
                    ? `/api/admin/resources/${selectedResource}`
                    : `/api/admin/resources/${selectedResource}/${editorId}`;
            const method = editorId === null ? "POST" : "PUT";
            const res = await apiFetch(path, {
                method,
                body: JSON.stringify(parsed),
            });
            if (res.ok) {
                setStatusBanner(
                    editorId === null
                        ? `Created record #${res.data.id}`
                        : `Updated record #${editorId}`,
                );
                void loadResource();
            } else {
                setStatusBanner(`Error: ${res.data.error}`);
            }
        } catch (err) {
            setStatusBanner(`Invalid JSON: ${String(err)}`);
        }
    };

    const handleDeleteResource = async (id: number) => {
        const res = await apiFetch(
            `/api/admin/resources/${selectedResource}/${id}`,
            { method: "DELETE" },
        );
        if (res.ok) {
            setStatusBanner(`Deleted record #${id}`);
            void loadResource();
        }
    };

    const handleRunCommand = async () => {
        try {
            const params = JSON.parse(cmdParamsJson || "{}");
            const res = await apiFetch("/api/admin/commands/execute", {
                method: "POST",
                body: JSON.stringify({
                    character_id: Number(cmdCharId),
                    command_type: cmdType,
                    params,
                }),
            });
            setCmdResult(res.data.message || JSON.stringify(res.data));
        } catch (err) {
            setCmdResult(`Invalid params JSON: ${String(err)}`);
        }
    };

    const handleSaveConfig = async () => {
        try {
            const value = JSON.parse(configValueJson);
            const res = await apiFetch(
                `/api/admin/configs/${selectedConfigKey}`,
                {
                    method: "PUT",
                    body: JSON.stringify({ value }),
                },
            );
            if (res.ok) {
                setStatusBanner(`Saved config '${selectedConfigKey}'`);
            } else {
                setStatusBanner(`Error: ${res.data.error}`);
            }
        } catch (err) {
            setStatusBanner(`Invalid JSON: ${String(err)}`);
        }
    };

    const handleSeasonAction = async (kind: string, action: string) => {
        const res = await apiFetch(`/api/admin/seasons/${kind}/${action}`, {
            method: "POST",
            body: JSON.stringify({ duration_days: 30 }),
        });
        if (res.ok) {
            setStatusBanner(`Season action ${kind}/${action} succeeded`);
            void loadOverview();
        }
    };

    const handleSendAnnounce = async () => {
        if (!announceText.trim()) return;
        const res = await apiFetch("/api/admin/chat/announce", {
            method: "POST",
            body: JSON.stringify({
                channel: chatChannel,
                message: announceText,
            }),
        });
        if (res.ok) {
            setAnnounceText("");
            setStatusBanner("Announcement stored & broadcast");
            void loadChat();
        }
    };

    if (!token) {
        return (
            <div
                style={{
                    minHeight: "100vh",
                    background: "#0f172a",
                    color: "#f8fafc",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    fontFamily: "system-ui, sans-serif",
                }}
            >
                <form
                    onSubmit={handleLogin}
                    style={{
                        background: "#1e293b",
                        padding: 32,
                        borderRadius: 12,
                        width: 380,
                        boxShadow: "0 10px 30px rgba(0,0,0,0.5)",
                    }}
                >
                    <h2 style={{ marginTop: 0, marginBottom: 8 }}>
                        Ninja Saga Admin Panel
                    </h2>
                    <p
                        style={{
                            color: "#94a3b8",
                            fontSize: 13,
                            marginBottom: 20,
                        }}
                    >
                        Sign in with an Admin account (account_type 99) or Admin
                        Secret.
                    </p>
                    {authError && (
                        <div
                            style={{
                                background: "#7f1d1d",
                                color: "#fecaca",
                                padding: "8px 12px",
                                borderRadius: 6,
                                marginBottom: 12,
                                fontSize: 13,
                            }}
                        >
                            {authError}
                        </div>
                    )}
                    <div style={{ marginBottom: 12 }}>
                        <label style={{ fontSize: 12, color: "#cbd5e1" }}>
                            Username or Email
                        </label>
                        <input
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                            style={{
                                width: "100%",
                                padding: 8,
                                marginTop: 4,
                                borderRadius: 6,
                                border: "1px solid #334155",
                                background: "#0f172a",
                                color: "#fff",
                                boxSizing: "border-box",
                            }}
                        />
                    </div>
                    <div style={{ marginBottom: 12 }}>
                        <label style={{ fontSize: 12, color: "#cbd5e1" }}>
                            Password
                        </label>
                        <input
                            type="password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            style={{
                                width: "100%",
                                padding: 8,
                                marginTop: 4,
                                borderRadius: 6,
                                border: "1px solid #334155",
                                background: "#0f172a",
                                color: "#fff",
                                boxSizing: "border-box",
                            }}
                        />
                    </div>
                    <div style={{ marginBottom: 16 }}>
                        <label style={{ fontSize: 12, color: "#cbd5e1" }}>
                            Or Admin Secret Key
                        </label>
                        <input
                            type="password"
                            placeholder="ninjasage_admin_secret"
                            value={secretInput}
                            onChange={(e) => setSecretInput(e.target.value)}
                            style={{
                                width: "100%",
                                padding: 8,
                                marginTop: 4,
                                borderRadius: 6,
                                border: "1px solid #334155",
                                background: "#0f172a",
                                color: "#fff",
                                boxSizing: "border-box",
                            }}
                        />
                    </div>
                    <button
                        type="submit"
                        style={{
                            width: "100%",
                            padding: 10,
                            borderRadius: 6,
                            border: "none",
                            background: "#ea580c",
                            color: "#fff",
                            fontWeight: 600,
                            cursor: "pointer",
                        }}
                    >
                        Sign In
                    </button>
                    <div style={{ marginTop: 14, textAlign: "center" }}>
                        <a
                            href="/app/"
                            style={{ color: "#94a3b8", fontSize: 12 }}
                        >
                            Return to Game Client
                        </a>
                    </div>
                </form>
            </div>
        );
    }

    return (
        <div
            style={{
                height: "100vh",
                overflow: "auto",
                background: "#0f172a",
                color: "#f8fafc",
                fontFamily: "system-ui, sans-serif",
                display: "flex",
                flexDirection: "column",
            }}
        >
            <header
                style={{
                    background: "#1e293b",
                    padding: "12px 24px",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    borderBottom: "1px solid #334155",
                }}
            >
                <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
                    <strong style={{ fontSize: 18, color: "#fb923c" }}>
                        Ninja Saga Admin
                    </strong>
                    {(
                        [
                            ["overview", "Overview & Seasons"],
                            ["resources", "Database Resources (26)"],
                            ["commands", "Admin Commands (23)"],
                            ["configs", "Game Settings & Configs"],
                            ["chat", "World Chat & Announce"],
                        ] as [TabKey, string][]
                    ).map(([k, label]) => (
                        <button
                            key={k}
                            onClick={() => setActiveTab(k)}
                            style={{
                                background:
                                    activeTab === k ? "#ea580c" : "transparent",
                                color: "#f8fafc",
                                border: "1px solid #334155",
                                borderRadius: 6,
                                padding: "6px 12px",
                                cursor: "pointer",
                                fontSize: 13,
                            }}
                        >
                            {label}
                        </button>
                    ))}
                </div>
                <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                    {statusBanner && (
                        <span
                            style={{
                                fontSize: 12,
                                background: "#065f46",
                                padding: "4px 10px",
                                borderRadius: 4,
                            }}
                        >
                            {statusBanner}
                        </span>
                    )}
                    <a
                        href="/app/"
                        style={{
                            color: "#cbd5e1",
                            fontSize: 13,
                            textDecoration: "none",
                        }}
                    >
                        Launch Game
                    </a>
                    <button
                        onClick={() => {
                            localStorage.removeItem("ns_admin_token");
                            setToken("");
                        }}
                        style={{
                            background: "#334155",
                            color: "#fff",
                            border: "none",
                            borderRadius: 6,
                            padding: "6px 10px",
                            cursor: "pointer",
                            fontSize: 12,
                        }}
                    >
                        Logout
                    </button>
                </div>
            </header>

            <main style={{ padding: 24, flex: 1 }}>
                {activeTab === "overview" && (
                    <div>
                        <h3 style={{ marginTop: 0 }}>Database Overview</h3>
                        <div
                            style={{
                                display: "grid",
                                gridTemplateColumns:
                                    "repeat(auto-fill, minmax(160px, 1fr))",
                                gap: 12,
                                marginBottom: 28,
                            }}
                        >
                            {Object.entries(overview.counts || {}).map(
                                ([tbl, cnt]) => (
                                    <div
                                        key={tbl}
                                        style={{
                                            background: "#1e293b",
                                            padding: 14,
                                            borderRadius: 8,
                                            border: "1px solid #334155",
                                        }}
                                    >
                                        <div
                                            style={{
                                                fontSize: 12,
                                                color: "#94a3b8",
                                            }}
                                        >
                                            {tbl}
                                        </div>
                                        <div
                                            style={{
                                                fontSize: 22,
                                                fontWeight: 700,
                                                marginTop: 4,
                                            }}
                                        >
                                            {cnt}
                                        </div>
                                    </div>
                                ),
                            )}
                        </div>

                        <h3>Season Management</h3>
                        <div
                            style={{
                                display: "grid",
                                gridTemplateColumns: "repeat(3, 1fr)",
                                gap: 16,
                            }}
                        >
                            {(["clan", "crew", "shadow_war"] as const).map(
                                (kind) => {
                                    const info = overview.seasons?.[kind];
                                    return (
                                        <div
                                            key={kind}
                                            style={{
                                                background: "#1e293b",
                                                padding: 16,
                                                borderRadius: 8,
                                                border: "1px solid #334155",
                                            }}
                                        >
                                            <h4
                                                style={{
                                                    marginTop: 0,
                                                    textTransform: "uppercase",
                                                }}
                                            >
                                                {kind.replace("_", " ")} Season
                                            </h4>
                                            <pre
                                                style={{
                                                    fontSize: 12,
                                                    background: "#0f172a",
                                                    padding: 10,
                                                    borderRadius: 6,
                                                    overflow: "auto",
                                                }}
                                            >
                                                {JSON.stringify(
                                                    info ?? { status: "none" },
                                                    null,
                                                    2,
                                                )}
                                            </pre>
                                            <div
                                                style={{
                                                    display: "flex",
                                                    gap: 8,
                                                    marginTop: 10,
                                                }}
                                            >
                                                <button
                                                    onClick={() =>
                                                        void handleSeasonAction(
                                                            kind,
                                                            "end_and_advance",
                                                        )
                                                    }
                                                    style={{
                                                        background: "#ea580c",
                                                        color: "#fff",
                                                        border: "none",
                                                        borderRadius: 6,
                                                        padding: "6px 10px",
                                                        cursor: "pointer",
                                                        fontSize: 12,
                                                    }}
                                                >
                                                    Start / Next Season
                                                </button>
                                                {kind === "crew" && (
                                                    <button
                                                        onClick={() =>
                                                            void handleSeasonAction(
                                                                "crew",
                                                                "advance_phase",
                                                            )
                                                        }
                                                        style={{
                                                            background:
                                                                "#334155",
                                                            color: "#fff",
                                                            border: "none",
                                                            borderRadius: 6,
                                                            padding: "6px 10px",
                                                            cursor: "pointer",
                                                            fontSize: 12,
                                                        }}
                                                    >
                                                        Toggle Phase
                                                    </button>
                                                )}
                                            </div>
                                        </div>
                                    );
                                },
                            )}
                        </div>
                    </div>
                )}

                {activeTab === "resources" && (
                    <div
                        style={{
                            display: "grid",
                            gridTemplateColumns: "220px 1fr 380px",
                            gap: 16,
                        }}
                    >
                        <div
                            style={{
                                background: "#1e293b",
                                padding: 12,
                                borderRadius: 8,
                                display: "flex",
                                flexDirection: "column",
                                gap: 4,
                            }}
                        >
                            {RESOURCES.map((r) => (
                                <button
                                    key={r.key}
                                    onClick={() => {
                                        setSelectedResource(r.key);
                                        setEditorId(null);
                                    }}
                                    style={{
                                        textAlign: "left",
                                        padding: "7px 10px",
                                        borderRadius: 6,
                                        border: "none",
                                        background:
                                            selectedResource === r.key
                                                ? "#ea580c"
                                                : "transparent",
                                        color: "#f8fafc",
                                        cursor: "pointer",
                                        fontSize: 13,
                                    }}
                                >
                                    {r.label}
                                </button>
                            ))}
                        </div>

                        <div
                            style={{
                                background: "#1e293b",
                                padding: 16,
                                borderRadius: 8,
                                overflow: "auto",
                            }}
                        >
                            <div
                                style={{
                                    display: "flex",
                                    gap: 8,
                                    marginBottom: 12,
                                }}
                            >
                                <input
                                    placeholder="Search..."
                                    value={searchQuery}
                                    onChange={(e) =>
                                        setSearchQuery(e.target.value)
                                    }
                                    style={{
                                        padding: 7,
                                        borderRadius: 6,
                                        border: "1px solid #334155",
                                        background: "#0f172a",
                                        color: "#fff",
                                        flex: 1,
                                    }}
                                />
                                <input
                                    placeholder="character_id filter"
                                    value={charFilter}
                                    onChange={(e) =>
                                        setCharFilter(e.target.value)
                                    }
                                    style={{
                                        padding: 7,
                                        borderRadius: 6,
                                        border: "1px solid #334155",
                                        background: "#0f172a",
                                        color: "#fff",
                                        width: 140,
                                    }}
                                />
                                <button
                                    onClick={() => void loadResource()}
                                    style={{
                                        background: "#334155",
                                        color: "#fff",
                                        border: "none",
                                        borderRadius: 6,
                                        padding: "7px 12px",
                                        cursor: "pointer",
                                    }}
                                >
                                    Refresh
                                </button>
                                <button
                                    onClick={() => {
                                        setEditorId(null);
                                        setEditorJson("{\n  \n}");
                                    }}
                                    style={{
                                        background: "#ea580c",
                                        color: "#fff",
                                        border: "none",
                                        borderRadius: 6,
                                        padding: "7px 12px",
                                        cursor: "pointer",
                                    }}
                                >
                                    + New
                                </button>
                            </div>
                            <table
                                style={{
                                    width: "100%",
                                    borderCollapse: "collapse",
                                    fontSize: 12,
                                }}
                            >
                                <thead>
                                    <tr
                                        style={{
                                            borderBottom: "1px solid #334155",
                                            textAlign: "left",
                                        }}
                                    >
                                        <th style={{ padding: 6 }}>ID</th>
                                        <th style={{ padding: 6 }}>Summary</th>
                                        <th style={{ padding: 6 }}>Actions</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {rows.map((row, idx) => {
                                        const rid = Number(row.id ?? idx);
                                        return (
                                            <tr
                                                key={rid}
                                                style={{
                                                    borderBottom:
                                                        "1px solid #0f172a",
                                                }}
                                            >
                                                <td style={{ padding: 6 }}>
                                                    {String(row.id ?? "-")}
                                                </td>
                                                <td
                                                    style={{
                                                        padding: 6,
                                                        maxWidth: 420,
                                                        overflow: "hidden",
                                                        textOverflow:
                                                            "ellipsis",
                                                        whiteSpace: "nowrap",
                                                    }}
                                                >
                                                    {JSON.stringify(row)}
                                                </td>
                                                <td
                                                    style={{
                                                        padding: 6,
                                                        display: "flex",
                                                        gap: 6,
                                                    }}
                                                >
                                                    <button
                                                        onClick={() => {
                                                            setEditorId(
                                                                row.id
                                                                    ? Number(
                                                                          row.id,
                                                                      )
                                                                    : null,
                                                            );
                                                            setEditorJson(
                                                                JSON.stringify(
                                                                    row,
                                                                    null,
                                                                    2,
                                                                ),
                                                            );
                                                        }}
                                                        style={{
                                                            background:
                                                                "#334155",
                                                            color: "#fff",
                                                            border: "none",
                                                            borderRadius: 4,
                                                            padding: "3px 8px",
                                                            cursor: "pointer",
                                                        }}
                                                    >
                                                        Edit
                                                    </button>
                                                    {row.id !== undefined && (
                                                        <button
                                                            onClick={() =>
                                                                void handleDeleteResource(
                                                                    Number(
                                                                        row.id,
                                                                    ),
                                                                )
                                                            }
                                                            style={{
                                                                background:
                                                                    "#991b1b",
                                                                color: "#fff",
                                                                border: "none",
                                                                borderRadius: 4,
                                                                padding:
                                                                    "3px 8px",
                                                                cursor: "pointer",
                                                            }}
                                                        >
                                                            Del
                                                        </button>
                                                    )}
                                                </td>
                                            </tr>
                                        );
                                    })}
                                </tbody>
                            </table>
                        </div>

                        <div
                            style={{
                                background: "#1e293b",
                                padding: 16,
                                borderRadius: 8,
                                display: "flex",
                                flexDirection: "column",
                            }}
                        >
                            <h4 style={{ marginTop: 0 }}>
                                {editorId === null
                                    ? `Create ${selectedResource}`
                                    : `Edit ${selectedResource} #${editorId}`}
                            </h4>
                            <textarea
                                value={editorJson}
                                onChange={(e) => setEditorJson(e.target.value)}
                                style={{
                                    flex: 1,
                                    minHeight: 360,
                                    background: "#0f172a",
                                    color: "#f8fafc",
                                    border: "1px solid #334155",
                                    borderRadius: 6,
                                    padding: 10,
                                    fontFamily: "monospace",
                                    fontSize: 12,
                                }}
                            />
                            <button
                                onClick={() => void handleSaveResource()}
                                style={{
                                    marginTop: 10,
                                    padding: 10,
                                    background: "#ea580c",
                                    color: "#fff",
                                    border: "none",
                                    borderRadius: 6,
                                    cursor: "pointer",
                                    fontWeight: 600,
                                }}
                            >
                                {editorId === null
                                    ? "Create Record"
                                    : "Save Changes"}
                            </button>
                        </div>
                    </div>
                )}

                {activeTab === "commands" && (
                    <div
                        style={{
                            background: "#1e293b",
                            padding: 20,
                            borderRadius: 8,
                            maxWidth: 640,
                        }}
                    >
                        <h3 style={{ marginTop: 0 }}>
                            AdminCommandService Executor
                        </h3>
                        <div style={{ marginBottom: 12 }}>
                            <label style={{ fontSize: 12, color: "#cbd5e1" }}>
                                Target Character ID
                            </label>
                            <input
                                value={cmdCharId}
                                onChange={(e) => setCmdCharId(e.target.value)}
                                style={{
                                    width: "100%",
                                    padding: 8,
                                    marginTop: 4,
                                    borderRadius: 6,
                                    border: "1px solid #334155",
                                    background: "#0f172a",
                                    color: "#fff",
                                }}
                            />
                        </div>
                        <div style={{ marginBottom: 12 }}>
                            <label style={{ fontSize: 12, color: "#cbd5e1" }}>
                                Command Type
                            </label>
                            <select
                                value={cmdType}
                                onChange={(e) => setCmdType(e.target.value)}
                                style={{
                                    width: "100%",
                                    padding: 8,
                                    marginTop: 4,
                                    borderRadius: 6,
                                    border: "1px solid #334155",
                                    background: "#0f172a",
                                    color: "#fff",
                                }}
                            >
                                {COMMAND_TYPES.map((ct) => (
                                    <option key={ct} value={ct}>
                                        {ct}
                                    </option>
                                ))}
                            </select>
                        </div>
                        <div style={{ marginBottom: 12 }}>
                            <label style={{ fontSize: 12, color: "#cbd5e1" }}>
                                Runtime Params (JSON)
                            </label>
                            <textarea
                                value={cmdParamsJson}
                                onChange={(e) =>
                                    setCmdParamsJson(e.target.value)
                                }
                                style={{
                                    width: "100%",
                                    height: 90,
                                    padding: 8,
                                    marginTop: 4,
                                    borderRadius: 6,
                                    border: "1px solid #334155",
                                    background: "#0f172a",
                                    color: "#fff",
                                    fontFamily: "monospace",
                                }}
                            />
                        </div>
                        <button
                            onClick={() => void handleRunCommand()}
                            style={{
                                background: "#ea580c",
                                color: "#fff",
                                border: "none",
                                borderRadius: 6,
                                padding: "10px 18px",
                                cursor: "pointer",
                                fontWeight: 600,
                            }}
                        >
                            Execute Command
                        </button>
                        {cmdResult && (
                            <pre
                                style={{
                                    marginTop: 16,
                                    background: "#0f172a",
                                    padding: 12,
                                    borderRadius: 6,
                                    whiteSpace: "pre-wrap",
                                    fontSize: 12,
                                }}
                            >
                                {cmdResult}
                            </pre>
                        )}
                    </div>
                )}

                {activeTab === "configs" && (
                    <div
                        style={{
                            display: "grid",
                            gridTemplateColumns: "240px 1fr",
                            gap: 16,
                        }}
                    >
                        <div
                            style={{
                                background: "#1e293b",
                                padding: 12,
                                borderRadius: 8,
                                display: "flex",
                                flexDirection: "column",
                                gap: 4,
                            }}
                        >
                            {CONFIG_KEYS.map((k) => (
                                <button
                                    key={k}
                                    onClick={() => setSelectedConfigKey(k)}
                                    style={{
                                        textAlign: "left",
                                        padding: "7px 10px",
                                        borderRadius: 6,
                                        border: "none",
                                        background:
                                            selectedConfigKey === k
                                                ? "#ea580c"
                                                : "transparent",
                                        color: "#f8fafc",
                                        cursor: "pointer",
                                        fontSize: 13,
                                    }}
                                >
                                    {k}
                                </button>
                            ))}
                        </div>
                        <div
                            style={{
                                background: "#1e293b",
                                padding: 16,
                                borderRadius: 8,
                                display: "flex",
                                flexDirection: "column",
                            }}
                        >
                            <h3 style={{ marginTop: 0 }}>
                                Config: {selectedConfigKey}
                            </h3>
                            <textarea
                                value={configValueJson}
                                onChange={(e) =>
                                    setConfigValueJson(e.target.value)
                                }
                                style={{
                                    minHeight: 420,
                                    background: "#0f172a",
                                    color: "#f8fafc",
                                    border: "1px solid #334155",
                                    borderRadius: 6,
                                    padding: 12,
                                    fontFamily: "monospace",
                                    fontSize: 12,
                                }}
                            />
                            <button
                                onClick={() => void handleSaveConfig()}
                                style={{
                                    marginTop: 12,
                                    padding: 10,
                                    background: "#ea580c",
                                    color: "#fff",
                                    border: "none",
                                    borderRadius: 6,
                                    cursor: "pointer",
                                    fontWeight: 600,
                                    alignSelf: "flex-start",
                                }}
                            >
                                Save Configuration
                            </button>
                        </div>
                    </div>
                )}

                {activeTab === "chat" && (
                    <div
                        style={{
                            background: "#1e293b",
                            padding: 20,
                            borderRadius: 8,
                        }}
                    >
                        <h3 style={{ marginTop: 0 }}>
                            World Chat Moderation & System Announcements
                        </h3>
                        <div
                            style={{
                                display: "flex",
                                gap: 8,
                                marginBottom: 16,
                            }}
                        >
                            <input
                                value={chatChannel}
                                onChange={(e) => setChatChannel(e.target.value)}
                                placeholder="global or clan:1"
                                style={{
                                    padding: 8,
                                    borderRadius: 6,
                                    border: "1px solid #334155",
                                    background: "#0f172a",
                                    color: "#fff",
                                    width: 160,
                                }}
                            />
                            <input
                                value={announceText}
                                onChange={(e) =>
                                    setAnnounceText(e.target.value)
                                }
                                placeholder="System announcement text..."
                                style={{
                                    padding: 8,
                                    borderRadius: 6,
                                    border: "1px solid #334155",
                                    background: "#0f172a",
                                    color: "#fff",
                                    flex: 1,
                                }}
                            />
                            <button
                                onClick={() => void handleSendAnnounce()}
                                style={{
                                    background: "#ea580c",
                                    color: "#fff",
                                    border: "none",
                                    borderRadius: 6,
                                    padding: "8px 16px",
                                    cursor: "pointer",
                                    fontWeight: 600,
                                }}
                            >
                                Broadcast Announcement
                            </button>
                        </div>
                        <table
                            style={{
                                width: "100%",
                                borderCollapse: "collapse",
                                fontSize: 13,
                            }}
                        >
                            <thead>
                                <tr
                                    style={{
                                        borderBottom: "1px solid #334155",
                                        textAlign: "left",
                                    }}
                                >
                                    <th style={{ padding: 8 }}>ID</th>
                                    <th style={{ padding: 8 }}>Sender</th>
                                    <th style={{ padding: 8 }}>Message</th>
                                </tr>
                            </thead>
                            <tbody>
                                {chatMessages.map((m, idx) => (
                                    <tr
                                        key={String(m.id ?? idx)}
                                        style={{
                                            borderBottom: "1px solid #0f172a",
                                        }}
                                    >
                                        <td style={{ padding: 8 }}>
                                            {String(m.id ?? "-")}
                                        </td>
                                        <td style={{ padding: 8 }}>
                                            {String(m.character_name ?? "")}
                                        </td>
                                        <td style={{ padding: 8 }}>
                                            {String(m.message ?? "")}
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}
            </main>
        </div>
    );
}
