'use client';

import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import type { AuthUser } from '@/types';
import { authApi } from '@/lib/api';

interface AuthContextType {
  user: AuthUser | null;
  loading: boolean;
  login: (method: 'local', data: { email: string; password: string }) => Promise<void>;
  loginLacisOath: (code: string, redirectUri: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);
  const pathname = usePathname();
  const router = useRouter();

  // usePathname() returns path WITHOUT basePath (e.g. '/login', '/')
  const isLoginPage = pathname === '/login';

  // Check session on mount
  useEffect(() => {
    if (isLoginPage) {
      setLoading(false);
      return;
    }

    authApi
      .me()
      .then((res) => {
        setUser(res.user);
      })
      .catch(() => {
        setUser(null);
        // Redirect to login when auth check fails on non-login pages
        router.push('/login');
      })
      .finally(() => {
        setLoading(false);
      });
  }, [isLoginPage, router]);

  // router.push() auto-prepends basePath, so use paths WITHOUT basePath
  const login = useCallback(
    async (_method: 'local', data: { email: string; password: string }) => {
      const res = await authApi.loginLocal(data.email, data.password);
      setUser(res.user);
      router.push('/');
    },
    [router],
  );

  const loginLacisOath = useCallback(
    async (code: string, redirectUri: string) => {
      const res = await authApi.loginLacisOath(code, redirectUri);
      setUser(res.user);
      router.push('/');
    },
    [router],
  );

  const logout = useCallback(async () => {
    await authApi.logout();
    setUser(null);
    router.push('/login');
  }, [router]);

  return (
    <AuthContext.Provider value={{ user, loading, login, loginLacisOath, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextType {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error('useAuth must be used within AuthProvider');
  }
  return ctx;
}
