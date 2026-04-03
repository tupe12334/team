import ConfigPanel from "@/components/ConfigPanel";
import DaemonPanel from "@/components/DaemonPanel";
import QueuePanel from "@/components/QueuePanel";
import WorkersPanel from "@/components/WorkersPanel";

export const dynamic = "force-static";

export default function Home() {
  return (
    <div className="min-h-screen flex flex-col">
      {/* ── Header ─────────────────────────────────────────────────────────── */}
      <header className="border-b border-[#1c2736] bg-[#0d1117]/80 backdrop-blur-sm sticky top-0 z-50">
        <div className="max-w-5xl mx-auto px-6 h-14 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="animate-pulse-dot w-2 h-2 rounded-full bg-orange-500 shrink-0" />
            <span className="font-sans font-bold tracking-widest text-sm text-[#c9d1d9] uppercase">
              Daemon Control
            </span>
          </div>
          <span className="font-mono text-xs text-[#6e7681]">team / daemon</span>
        </div>
      </header>

      {/* ── Main content ───────────────────────────────────────────────────── */}
      <main className="flex-1 max-w-5xl mx-auto w-full px-6 py-10 flex flex-col gap-8">
        <DaemonPanel />
        <ConfigPanel />
        <QueuePanel />
        <WorkersPanel />
      </main>

      <footer className="border-t border-[#1c2736] py-4">
        <p className="text-center font-mono text-xs text-[#6e7681]">
          gRPC → localhost:{process.env.DAEMON_PORT}
        </p>
      </footer>
    </div>
  );
}
