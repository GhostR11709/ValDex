import type { Metadata, Viewport } from "next";
import { Geist } from "next/font/google";
import { Providers } from "@/components/providers";
import "./globals.css";

const geist = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Vivi for Windows",
  description: "A Windows 11 music client inspired by Vivi Music. Search YouTube Music, play songs, and keep a library on your PC.",
  applicationName: "Vivi",
  manifest: "/manifest.webmanifest",
  appleWebApp: { capable: true, title: "Vivi" },
};

export const viewport: Viewport = {
  themeColor: "#0b0b0e",
  width: "device-width",
  initialScale: 1,
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={`${geist.variable} dark h-full`} suppressHydrationWarning>
      <body className="h-full bg-[#0b0b0e] text-white antialiased">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
