import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  serverExternalPackages: ["@grpc/grpc-js", "@grpc/proto-loader"],
  turbopack: {
    root: __dirname,
  },
};

export default nextConfig;
