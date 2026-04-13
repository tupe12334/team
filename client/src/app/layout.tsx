import type { ReactNode } from "react";
import type { Metadata } from "next";
import { Syne, IBM_Plex_Mono } from "next/font/google";
import "./globals.css";

const syne = Syne({
  variable: "--font-syne",
  subsets: ["latin"],
  weight: ["400", "600", "700", "800"],
});

const ibmPlexMono = IBM_Plex_Mono({
  variable: "--font-ibm-plex-mono",
  subsets: ["latin"],
  weight: ["400", "500", "600"],
});

export const metadata: Metadata = {
  title: "Team Daemon",
  description: "Task queue and worker pool manager",
};

export default function RootLayout({
  children,
}: Readonly<{ children: ReactNode }>) {
  return (
    <html
      lang="en"
      className={`${syne.variable} ${ibmPlexMono.variable} h-full`}
    >
      <body className="min-h-full flex flex-col bg-[#07090c] text-[#c9d1d9] font-sans antialiased">
        {children}
      </body>
    </html>
  );
}
