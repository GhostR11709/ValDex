import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  distDir: process.env.VIVI_DIST_DIR || ".next",
  output: "standalone",
  cacheComponents: true,
  partialPrefetching: true,
  allowedDevOrigins: ["127.0.0.1"],
  serverExternalPackages: ["youtubei.js", "soundcloud.ts"],
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
