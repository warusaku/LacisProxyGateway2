/** @type {import('next').NextConfig} */
const nextConfig = {
  basePath: '/LacisProxyGateway2',
  assetPrefix: '/LacisProxyGateway2/',
  output: 'standalone',
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'http://127.0.0.1:8081/api/:path*',
      },
    ];
  },
};

export default nextConfig;
