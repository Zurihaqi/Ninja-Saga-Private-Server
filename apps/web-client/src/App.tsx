import { useEffect, useState } from "react";
import AdminDashboard from "./components/AdminDashboard";
import RuffleEmbed from "./components/RuffleEmbed";

/**
 * Full-viewport host for the Ruffle WASM player, or the Administrative
 * Dashboard when navigated to `/app/#admin` (or redirected from `/admin`).
 */
export default function App() {
    const [isAdmin, setIsAdmin] = useState<boolean>(() =>
        window.location.hash.startsWith("#admin"),
    );

    useEffect(() => {
        const onHashChange = () => {
            setIsAdmin(window.location.hash.startsWith("#admin"));
        };
        window.addEventListener("hashchange", onHashChange);
        return () => window.removeEventListener("hashchange", onHashChange);
    }, []);

    if (isAdmin) {
        return <AdminDashboard />;
    }

    return (
        <div className="swf-container">
            <RuffleEmbed />
        </div>
    );
}
