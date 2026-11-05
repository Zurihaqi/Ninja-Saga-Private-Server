import { useEffect, useRef } from "react";

/**
 * Mounts one <ruffle-player> instance pointing at the same-origin SWF.
 * The wasm runtime and its config come from the global install in index.html,
 * so this component only has to create the element and start the movie.
 */
export default function RuffleEmbed() {
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const container = containerRef.current;
        const factory = window.RufflePlayer?.newest?.();
        if (!container || !factory) {
            console.error(
                "ruffle runtime missing; was scripts/sync-ruffle.mjs run and ruffle.js loaded?",
            );
            return;
        }

        let cancelled = false;
        let player: ReturnType<typeof factory.createPlayer> | null = null;
        const frame = window.requestAnimationFrame(() => {
            if (cancelled) {
                return;
            }
            player = factory.createPlayer();
            player.style.display = "block";
            player.style.width = "100%";
            player.style.height = "100%";
            container.appendChild(player);

            const currentPlayer = player;
            const loaded = currentPlayer.load({ url: "/client/NinjaSage.swf" });
            loaded
                .catch((error: unknown) => {
                    if (!cancelled) {
                        console.error("failed to load NinjaSage.swf", error);
                    }
                })
                .finally(() => {
                    if (cancelled) {
                        currentPlayer.remove();
                    }
                });
        });

        return () => {
            cancelled = true;
            window.cancelAnimationFrame(frame);
            player?.remove();
        };
    }, []);

    return <div className="ruffle-player" ref={containerRef} />;
}
