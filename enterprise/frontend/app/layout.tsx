import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Stale Enterprise - Fail-closed Guardrail Platform",
  description: "Enterprise security platform for agents that touch money. SOC2 ready, 99.99% SLA.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="bg-[#020617] text-slate-100 min-h-screen antialiased">
        {children}
      </body>
    </html>
  );
}
