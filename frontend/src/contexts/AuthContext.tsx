'use client';

import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import type { AuthUser } from '@/types';
import { authApi } from '@/lib/api';

interface AuthContextType {
  user: AuthUser | null;
  loading: boolean;
  login: (method: 'local', data: { email: string; password: string }) => Promise<void>;
  loginLacisOath: (token: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);
  const pathname = usePathname();
  const router = useRouter();

  const isLoginPage = pathname === '/LacisProxyGateway2/login';

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
      })
      .finally(() => {
        setLoading(false);
      });
  }, [isLoginPage]);

  const login = useCallback(
    async (_method: 'local', data: { email: string; password: string }) => {
      const res = await authApi.loginLocal(data.email, data.password);
      setUser(res.user);
      router.push('/LacisProxyGateway2');
    },
    [router],
  );

  const loginLacisOath = useCallback(
    async (token: string) => {
      const res = await authApi.loginLacisOath(token);
      setUser(res.user);
      router.push('/LacisProxyGateway2');
    },
    [router],
  );

  const logout = useCallback(async () => {
    await authApi.logout();
    setUser(null);
    router.push('/LacisProxyGateway2/login');
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
