'use client';

import { useState, useEffect, Suspense } from 'react';
import { useSearchParams } from 'next/navigation';
import { useAuth } from '@/contexts/AuthContext';

function LoginContent() {
  const { user, loading, login, loginLacisOath } = useAuth();
  const searchParams = useSearchParams();

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [submitting, setSubmitting] = useState(false);

  // Handle LacisOath callback
  useEffect(() => {
    const callback = searchParams.get('callback');
    const token = searchParams.get('lacisOathToken');

    if (callback === '1' && token) {
      setSubmitting(true);
      setError('');

      // Decode JWT payload to pre-check permission/fid (client-side validation)
      try {
        const payloadB64 = token.split('.')[1];
        const payload = JSON.parse(atob(payloadB64));
        const permission = payload.permission || 0;
        const fids: string[] = payload.fid || [];

        if (permission < 80) {
          setError(`Insufficient permission: ${permission} (required: >= 80)`);
          setSubmitting(false);
          return;
        }

        if (!fids.includes('9966') && !fids.includes('0000')) {
          setError('Access not authorized for this facility');
          setSubmitting(false);
          return;
        }

        // Send to backend for session creation
        loginLacisOath(token).catch((e) => {
          setError(e.message || 'LacisOath login failed');
          setSubmitting(false);
        });
      } catch {
        setError('Invalid LacisOath token format');
        setSubmitting(false);
      }
    }
  }, [searchParams, loginLacisOath]);

  // If already authenticated, redirect happens in AuthContext
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-gray-400">Loading...</div>
      </div>
    );
  }

  if (user) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-gray-400">Redirecting...</div>
      </div>
    );
  }

  const handleLocalLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setSubmitting(true);

    try {
      await login('local', { email, password });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Login failed';
      setError(message);
    } finally {
      setSubmitting(false);
    }
  };

  const lacisOathUrl = `https://lacisoath.web.app/login/?returnUrl=${encodeURIComponent(
    window.location.origin + '/LacisProxyGateway2/login?callback=1'
  )}`;

  return (
    <div className="min-h-screen flex items-center justify-center bg-background">
      <div className="w-full max-w-md">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-blue-400">LacisProxyGateway2</h1>
          <p className="text-gray-500 mt-2">Sign in to continue</p>
        </div>

        {/* Card */}
        <div className="bg-card border border-border rounded-lg p-8">
          {/* Error message */}
          {error && (
            <div className="mb-6 p-3 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">
              {error}
            </div>
          )}

          {/* LacisOath button */}
          <a
            href={lacisOathUrl}
            className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-md transition-colors font-medium"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
              />
            </svg>
            Login with LacisOath
          </a>

          {/* Divider */}
          <div className="flex items-center my-6">
            <div className="flex-1 border-t border-border" />
            <span className="px-4 text-gray-500 text-sm">or</span>
            <div className="flex-1 border-t border-border" />
          </div>

          {/* Local login form */}
          <form onSubmit={handleLocalLogin} className="space-y-4">
            <div>
              <label htmlFor="email" className="block text-sm font-medium text-gray-400 mb-1">
                Email
              </label>
              <input
                id="email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
                disabled={submitting}
                className="w-full px-3 py-2 bg-background border border-border rounded-md text-text focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
                placeholder="webadmin@mijeos.com"
              />
            </div>
            <div>
              <label htmlFor="password" className="block text-sm font-medium text-gray-400 mb-1">
                Password
              </label>
              <input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                disabled={submitting}
                className="w-full px-3 py-2 bg-background border border-border rounded-md text-text focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
              />
            </div>
            <button
              type="submit"
              disabled={submitting}
              className="w-full px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-md transition-colors font-medium disabled:opacity-50"
            >
              {submitting ? 'Signing in...' : 'Sign in with Local Account'}
            </button>
          </form>
        </div>

        {/* Footer */}
        <p className="text-center text-gray-600 text-xs mt-6">
          Reverse Proxy Gateway Management System
        </p>
      </div>
    </div>
  );
}

export default function LoginPage() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen flex items-center justify-center bg-background">
          <div className="text-gray-400">Loading...</div>
        </div>
      }
    >
      <LoginContent />
    </Suspense>
  );
}
