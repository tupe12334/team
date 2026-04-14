import ConfigPanel from "@/components/ConfigPanel";
import DaemonPanel from "@/components/DaemonPanel";
import QueuePanel from "@/components/QueuePanel";
import WorkersPanel from "@/components/WorkersPanel";

// Force server-render on every request so process.env is read at runtime,
// not baked in at build time (DAEMON_ADDR is a runtime env var in Docker).
export const dynamic = "force-dynamic";

// eslint-disable-next-line no-restricted-syntax
const daemonAddr = process.env.DAEMON_ADDR ?? (process.env.DAEMON_PORT ? `[::1]:${process.env.DAEMON_PORT}` : null);

export default function Home() {
  return (
    <div className="home min-h-screen flex flex-col">
      {/* ── Header ─────────────────────────────────────────────────────────── */}
      <header className="home-header border-b border-[#1c2736] bg-[#0d1117]/80 backdrop-blur-sm sticky top-0 z-50">
        <div className="home-header-inner max-w-5xl mx-auto px-6 h-14 flex items-center justify-between">
          <div className="home-header-brand flex items-center gap-3">
            <span className="animate-pulse-dot w-2 h-2 rounded-full bg-orange-500 shrink-0" />
            <span className="home-header-title font-sans font-bold tracking-widest text-sm text-[#c9d1d9] uppercase">
              Daemon Control
            </span>
          </div>
          <span className="home-project-id font-mono text-xs text-[#6e7681]">team / daemon</span>
        </div>
      </header>

      {/* ── Main content ───────────────────────────────────────────────────── */}
      <main className="home-main flex-1 max-w-5xl mx-auto w-full px-6 py-10 flex flex-col gap-8">
        <DaemonPanel />
        <ConfigPanel />
        <QueuePanel />
        <WorkersPanel />
      </main>

      <footer className="home-footer border-t border-[#1c2736] py-4">
        <p className="home-footer-text text-center font-mono text-xs text-[#6e7681]">
          {daemonAddr ? `gRPC → ${daemonAddr}` : "gRPC port not configured"}
        </p>
      </footer>
    </div>
  );
}
