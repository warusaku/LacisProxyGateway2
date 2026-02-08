'use client';

import { usePathname } from 'next/navigation';
import './globals.css';
import { ToastProvider } from '@/components/ui/Toast';
import { AuthProvider, useAuth } from '@/contexts/AuthContext';

const navItems = [
  { href: '/LacisProxyGateway2', label: 'Dashboard', icon: 'M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6' },
  { href: '/LacisProxyGateway2/celestial-globe', label: 'CelestialGlobe', icon: 'M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z' },
  { href: '/LacisProxyGateway2/server-routes', label: 'ServerRoutes', icon: 'M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4' },
  { href: '/LacisProxyGateway2/ddns', label: 'DDNS', icon: 'M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9' },
  { href: '/LacisProxyGateway2/aranea-sdk', label: 'araneaSDK', icon: 'M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z' },
  { href: '/LacisProxyGateway2/security', label: 'Security', icon: 'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z' },
  { href: '/LacisProxyGateway2/logs', label: 'Logs', icon: 'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2' },
  { href: '/LacisProxyGateway2/omada', label: 'OmadaControl', icon: 'M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01' },
  { href: '/LacisProxyGateway2/openwrt', label: 'OpenWrt', icon: 'M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z' },
  { href: '/LacisProxyGateway2/external', label: 'External', icon: 'M13 10V3L4 14h7v7l9-11h-7z' },
  { href: '/LacisProxyGateway2/wireguard', label: 'WireGuard', icon: 'M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z' },
  { href: '/LacisProxyGateway2/settings', label: 'Settings', icon: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z' },
];

function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { user, loading, logout } = useAuth();

  // usePathname() returns path WITHOUT basePath (e.g. '/login', '/', '/routes')
  const isLoginPage = pathname === '/login';

  const isActive = (href: string) => {
    // navItems hrefs include basePath for <a> tags; strip it for comparison with usePathname()
    const normalizedHref = href.replace(/^\/LacisProxyGateway2/, '') || '/';
    if (normalizedHref === '/') {
      return pathname === '/' || pathname === '';
    }
    return pathname?.startsWith(normalizedHref) ?? false;
  };

  // Login page: no sidebar
  if (isLoginPage) {
    return <>{children}</>;
  }

  // Show loading while auth check is in progress
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-gray-400">Loading...</div>
      </div>
    );
  }

  // Not authenticated: AuthContext handles redirect, show minimal UI
  if (!user) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-gray-400">Redirecting...</div>
      </div>
    );
  }

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Sidebar */}
      <aside className="w-64 bg-card border-r border-border p-4 flex flex-col shrink-0 overflow-y-auto">
        <h1 className="text-xl font-bold mb-8 text-blue-400">LacisProxyGateway2</h1>
        <nav className="space-y-1 flex-1">
          {navItems.map((item) => (
            <a
              key={item.href}
              href={item.href}
              className={`
                flex items-center gap-3 px-3 py-2 rounded-md transition-colors
                ${isActive(item.href)
                  ? 'bg-blue-600 text-white'
                  : 'text-gray-400 hover:bg-gray-800 hover:text-white'}
              `}
            >
              <svg
                className="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d={item.icon}
                />
              </svg>
              {item.label}
            </a>
          ))}
        </nav>

        {/* User info + logout */}
        <div className="pt-4 border-t border-border space-y-2">
          <div className="flex items-center gap-2">
            <span
              className={`shrink-0 px-1.5 py-0.5 text-[10px] font-bold rounded ${
                user.auth_method === 'lacisoath'
                  ? 'bg-blue-600 text-white'
                  : 'bg-yellow-600 text-white'
              }`}
            >
              {user.auth_method === 'lacisoath' ? 'LacisOath' : 'Master'}
            </span>
            <span className="text-xs text-gray-400 truncate" title={user.sub}>
              {user.sub}
            </span>
          </div>
          {user.lacis_id && (
            <div className="text-[10px] text-gray-500 truncate pl-1" title={user.lacis_id}>
              ID: {user.lacis_id}
            </div>
          )}
          <button
            onClick={logout}
            className="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-400 hover:bg-gray-800 hover:text-white rounded-md transition-colors"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
              />
            </svg>
            Logout
          </button>
        </div>

        <div className="pt-4 border-t border-border text-xs text-gray-500">
          v{process.env.npm_package_version || '0.1.0'}
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 p-8 overflow-y-auto">
        {children}
      </main>
    </div>
  );
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="ja">
      <head>
        <title>LacisProxyGateway2</title>
        <meta name="description" content="Reverse Proxy Gateway Management" />
      </head>
      <body className="min-h-screen bg-background text-text">
        <ToastProvider>
          <AuthProvider>
            <AppShell>{children}</AppShell>
          </AuthProvider>
        </ToastProvider>
      </body>
    </html>
  );
}
