"use client";

import { ThemeProvider } from "next-themes";
import { LibraryProvider } from "@/components/library-provider";
import { PlayerProvider } from "@/components/player-provider";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";

export function Providers({ children }: { children: React.ReactNode }) {
  return (
    <ThemeProvider attribute="class" forcedTheme="dark" enableSystem={false}>
      <TooltipProvider delayDuration={250}>
        <LibraryProvider>
          <PlayerProvider>
            {children}
            <Toaster position="bottom-center" />
          </PlayerProvider>
        </LibraryProvider>
      </TooltipProvider>
    </ThemeProvider>
  );
}
